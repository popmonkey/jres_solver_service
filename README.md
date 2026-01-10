# jres_solver_service

Wrapper around [popmonkey/jres_solver_cpp](https://github.com/popmonkey/jres_solver_cpp) library that allows running the solver remotely via a JSON API.

## Development Setup

### Prerequisites

*   **Rust Toolchain:** Ensure you have the latest stable Rust installed. You can install it via [rustup.rs](https://rustup.rs/).
*   **C++ Toolchain:** A compatible C++ compiler (Clang) is required for bridging with the C++ library.
*   **Highs Library:** The solver depends on the [Highs](https://github.com/ERGO-Code/HiGHS) optimization library.
    *   **macOS:** `brew install highs`
    *   **Debian/Ubuntu:** `sudo apt install libhighs-dev`

### Setting up Dependencies

This project relies on the `jres_solver_cpp` library. You must download the headers and the pre-compiled **static** library for your architecture.

1.  **Download the Latest Release:**
    Visit [https://github.com/popmonkey/jres_solver_cpp/releases/latest](https://github.com/popmonkey/jres_solver_cpp/releases/latest).

2.  **Create Vendor Directory:**
    Create the following directory structure in the project root:
    ```bash
    mkdir -p vendor/jres_solver/include
    mkdir -p vendor/jres_solver/lib
    ```

3.  **Install Files:**
    *   **Headers:** Copy the header files from the release (usually under `include/`) into `vendor/jres_solver/include/`.
    *   **Library:** Copy the static library file (`libjres_solver.a`) into `vendor/jres_solver/lib/`.

### Building the Project

With the vendor files in place, you can build the project using Cargo:

```bash
cargo build
```

### Running Locally

```bash
cargo run
```

## Production Setup

For production deployment on Debian 13, follow these steps:

### Requirements

*   **OS:** Debian 13 (Linux)
*   **Architecture:** Typically x86_64 (ensure you match the library build)
*   **Reverse Proxy:** Apache2 (configured to proxy to `localhost:8080`)
*   **Service Manager:** systemd

### Setup Steps

1.  **Get the Linux Library:**
    Download the Linux version of `jres_solver_cpp` (static library `libjres_solver.a`) from the [releases page](https://github.com/popmonkey/jres_solver_cpp/releases/latest) and place it in `vendor/jres_solver/lib/`.

2.  **Install Highs on Server:**
    Ensure `libhighs` is installed on the Debian server:
    ```bash
    sudo apt install libhighs-dev
    ```

3.  **Build the Service:**
    Build the Rust binary in release mode:
    ```bash
    cargo build --release
    ```
    The artifact will be located at `target/release/jres_solver_service`.

4.  **Deployment:**
    *   Copy the binary to the Debian 13 server.
    *   Since `jres_solver` is statically linked, you do not need to copy its library file.
    *   Configure a `systemd` unit to manage the service.