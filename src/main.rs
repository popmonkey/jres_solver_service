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

#[cxx::bridge]
mod ffi {
    struct SolverOptions {
        time_limit: i32,
        spotter_mode: i32,
        allow_no_spotter: bool,
        optimality_gap: f64,
        role_coupling_weight: f64,
        rotation_beat_weight: f64,
    }

    unsafe extern "C++" {
        include!("src/shim.h");

        fn solve_wrapper(input_json: String, options: &SolverOptions) -> String;
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
struct ServerConfig {
    ip: String,
    port: u16,
}

#[derive(Clone, Deserialize)]
struct AppConfig {
    solve: SolveConfig,
    server: ServerConfig,
}

struct AppState {
    config: Arc<AppConfig>,
    api_key: String,
}

// Query parameters
#[derive(Deserialize)]
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
            (StatusCode::UNAUTHORIZED, Json(error_body)).into_response()
        }
    }
}

async fn solve(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SolveQueryParams>,
    Json(input): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let config = &state.config;

    // Merge config defaults with query parameters
    let options = ffi::SolverOptions {
        time_limit: config.solve.time_limit,
        spotter_mode: params.spotter_mode.unwrap_or(config.solve.spotter_mode),
        allow_no_spotter: params.allow_no_spotter.unwrap_or(config.solve.allow_no_spotter),
        optimality_gap: config.solve.optimality_gap,
        role_coupling_weight: config.solve.role_coupling_weight,
        rotation_beat_weight: config.solve.rotation_beat_weight,
    };

    let input_str = input.to_string();

    // Spawn blocking task for the solver
    let result = task::spawn_blocking(move || {
        ffi::solve_wrapper(input_str, &options)
    }).await.expect("Task failed");

    let result_json: serde_json::Value = serde_json::from_str(&result).unwrap_or_else(|_| {
        serde_json::json!({ "error": "Invalid JSON returned from solver", "raw": result })
    });

    Json(result_json)
}

#[tokio::main]
async fn main() {
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
        .with_state(state);

    let addr_str = format!("{}:{}", config.server.ip, config.server.port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid IP or port in config.yaml");
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}