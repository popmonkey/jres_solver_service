use axum::{
    extract::{Query, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task;
use tracing::{info, warn};
use tower_http::{
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cxx::bridge]
mod ffi {
    #[derive(Clone, Debug)]
    struct SolverOptions {
        time_limit: i32,
        spotter_mode: i32,
        allow_no_spotter: bool,
        optimality_gap: f64,
        role_coupling_weight: f64,
        rotation_beat_weight: f64,
        diagnose: bool,
    }

    unsafe extern "C++" {
        include!("src/shim.h");

        fn solve_wrapper(input_json: String, options: &SolverOptions) -> String;
        fn get_version_wrapper() -> String;
    }
}

// Config file structures
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolveConfig {
    time_limit: i32,
    optimality_gap: f64,
    role_coupling_weight: f64,
    rotation_beat_weight: f64,
    spotter_mode: i32,
    allow_no_spotter: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerConfig {
    ip: String,
    port: u16,
    request_directory: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    solve: SolveConfig,
    server: ServerConfig,
}

struct AppState {
    config: Arc<AppConfig>,
    api_key: String,
}

// Query parameters
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SolveQueryParams {
    spotter_mode: Option<i32>,
    allow_no_spotter: Option<bool>,
}

async fn auth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let api_key_header = headers
        .get("X-API-KEY")
        .and_then(|value| value.to_str().ok());

    match api_key_header {
        Some(key) if key == state.api_key => next.run(request).await,
        _ => {
            let error_body = serde_json::json!({ "error": "Unknown API key error" });
            let cf_ip = headers
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            warn!("Unauthorized request from IP: {}, Referer: {:?}", cf_ip, headers.get("referer"));
            (StatusCode::UNAUTHORIZED, Json(error_body)).into_response()
        }
    }
}



async fn solve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SolveQueryParams>,
    Json(input): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let instance_id = input.get("instanceId")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    let request_id = headers.get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    info!(instance_id = %instance_id, request_id = %request_id, "Solving request");

    let config = &state.config;

    // Log request if configured
    if let Some(req_dir) = &config.server.request_directory {
        let req_dir = req_dir.clone();
        let req_id = request_id.clone();
        let input_clone = input.clone();
        
        tokio::spawn(async move {
            let path = std::path::Path::new(&req_dir).join(&req_id);
            if let Err(e) = tokio::fs::create_dir_all(&path).await {
                warn!("Failed to create request directory {}: {}", path.display(), e);
                return;
            }
            if let Err(e) = tokio::fs::write(path.join("request.json"), input_clone.to_string()).await {
                warn!("Failed to write request.json: {}", e);
            }
        });
    }

    // Merge config defaults with query parameters
    let options = ffi::SolverOptions {
        time_limit: config.solve.time_limit,
        spotter_mode: params.spotter_mode.unwrap_or(config.solve.spotter_mode),
        allow_no_spotter: params.allow_no_spotter.unwrap_or(config.solve.allow_no_spotter),
        optimality_gap: config.solve.optimality_gap,
        role_coupling_weight: config.solve.role_coupling_weight,
        rotation_beat_weight: config.solve.rotation_beat_weight,
        diagnose: false,
    };

    let input_str = input.to_string();
    let input_str_solve = input_str.clone();
    let options_solve = options.clone();

    // Spawn blocking task for the solver
    let result = task::spawn_blocking(move || {
        ffi::solve_wrapper(input_str_solve, &options_solve)
    }).await.expect("Task failed");

    let mut result_json: serde_json::Value = serde_json::from_str(&result).unwrap_or_else(|_| {
        serde_json::json!({ "error": "Invalid JSON returned from solver", "raw": result })
    });

    let is_failure = result_json.get("diagnosis")
        .and_then(|d| d.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    if is_failure {
        info!(instance_id = %instance_id, "Solver failed, retrying with diagnosis...");
        let input_str_diag = input_str.clone();
        let mut options_diag = options.clone();
        options_diag.diagnose = true;

        let result_diag = task::spawn_blocking(move || {
            ffi::solve_wrapper(input_str_diag, &options_diag)
        }).await.expect("Task failed");

        result_json = serde_json::from_str(&result_diag).unwrap_or_else(|_| {
             serde_json::json!({ "error": "Invalid JSON returned from diagnosis", "raw": result_diag })
        });
    }

    // Log result if configured
    if let Some(req_dir) = &config.server.request_directory {
        let req_dir = req_dir.clone();
        let req_id = request_id.clone();
        let result_clone = result_json.clone();
        
        tokio::spawn(async move {
            let path = std::path::Path::new(&req_dir).join(&req_id);
            if let Err(e) = tokio::fs::create_dir_all(&path).await {
                warn!("Failed to create request directory {}: {}", path.display(), e);
                return;
            }

            if let Err(e) = tokio::fs::write(path.join("result.json"), result_clone.to_string()).await {
                warn!("Failed to write result.json: {}", e);
            }
        });
    }

    let diagnosis_array = result_json.get("diagnosis")
        .and_then(|d| d.as_array());

    let is_failure = diagnosis_array
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    if !is_failure {
        info!(instance_id = %instance_id, "SUCCESS");
    } else {
        let diagnosis_str = diagnosis_array
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_else(|| "Unknown failure".to_string());
        
        info!(instance_id = %instance_id, "{}", diagnosis_str);
    }

    Json(result_json)
}

#[tokio::main]
async fn main() {
    // Check for version flag
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--version".to_string()) {
        let lib_version = ffi::get_version_wrapper();
        println!("{} ({})", env!("CARGO_PKG_VERSION"), lib_version);
        return;
    }

    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("jres_solver_service=info,tower_http=info"));

    let log_dir = std::env::var("LOG_DIR").ok();
    
    if let Some(dir) = log_dir {
        let log_path = std::path::Path::new(&dir).join("jres_solver.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open log file");

        let (non_blocking, _guard) = tracing_appender::non_blocking(file);
        
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
            .init();
        
        // Note: _guard must be kept alive for the duration of the program
        // Since we are in main, we can leak it or move it to a global if needed, 
        // but for a simple service, it's often fine to just let it drop at the end of main 
        // if main never returns. However, axum's serve will return on shutdown.
        // To be safe, we can use Box::leak.
        Box::leak(Box::new(_guard));
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
    }

    info!("Starting JRES Solver Service");
    let version = ffi::get_version_wrapper();
    info!("JRES Solver Version: {}", version);

    // Load configuration
    let config_file = std::fs::File::open("config.yaml").expect("Failed to open config.yaml");
    let config: AppConfig = serde_yaml::from_reader(config_file).expect("Failed to parse config.yaml");
    let config = Arc::new(config);

    // Load API Key
    let api_key = std::fs::read_to_string("jres_api_key.txt")
        .expect("Failed to read jres_api_key.txt")
        .trim()
        .to_string();

    let state = Arc::new(AppState {
        config: config.clone(),
        api_key,
    });

    let app = Router::new()
        .route("/solve", post(solve))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let request_id = request.headers().get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    let referer = request.headers().get("referer")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    let cf_ip = request.headers().get("cf-connecting-ip")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    let origin = request.headers().get("origin")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    let uri = request.uri().to_string();

                    tracing::info_span!(
                        "request",
                        request_id = %request_id,
                        method = %request.method(),
                        uri = %uri,
                        referer = %referer,
                        origin = %origin,
                        cf_ip = %cf_ip,
                    )
                })
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .with_state(state);

    let addr_str = format!("{}:{}", config.server.ip, config.server.port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid IP or port in config.yaml");
    info!("listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    let server = axum::serve(listener, app);
    
    let graceful = server.with_graceful_shutdown(async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
        info!("Shutdown signal received");
    });

    if let Err(e) = graceful.await {
        warn!("Server error: {}", e);
    }

    info!("Stopping JRES Solver Service");
}