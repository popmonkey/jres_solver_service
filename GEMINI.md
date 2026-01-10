# JRES Solver Service (Rust Wrapper)

## Overview

A high-performance web service built in Rust that wraps the `jres_solver_cpp` library. It exposes a JSON API via Apache2 (Reverse Proxy) to handle scheduling requests for json.racing.

## Tech Stack

* **Language:** Rust (Stable)
* **C++ Interop:** `cxx` crate
* **Web Framework:** `axum` (Tokio-based)
* **Serialization:** `serde` & `serde_json`
* **Build System:** `cargo` + `build.rs`
* **Dependency Version:** `jres_solver_cpp` v3.1.0
* **External Lib:** `Highs` optimization library (dylib)

## Architecture

1. **Request:** Client sends `POST` JSON to Apache.
2. **Proxy:** Apache forwards to `localhost:8080`.
3. **Rust:** Axum receives JSON, deserializes into Rust structs.
4. **Bridge:** Data is passed across the FFI boundary using `cxx`.
5. **C++:** `jres_solver_cpp` executes the logic.
6. **Response:** Result is passed back to Rust, serialized to JSON, and returned.

## Implementation Details

* **Vendoring:** The C++ library is located in `vendor/jres_solver`.
  * use the static library (`.a`) matching the dev architecture (e.g., macOS ARM64)
  * use the linux static library (`.a`) for production
* **FFI:** `cxx` is used to bridge Rust and C++. A shim layer (`src/shim.cc`) is used to bridge the library's classes to `cxx`.
* **Threading:** Heavy solver tasks are wrapped in `tokio::task::spawn_blocking` to prevent starving the async executor.

## Deployment Strategy

* Managed as a `systemd` service on the Apache server.
* Listens on `127.0.0.1:8080`.
