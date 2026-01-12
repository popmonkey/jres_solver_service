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

For production deployment on Debian 13, you can use the provided automated installation script.

### Requirements

*   **OS:** Debian 13 (Linux)
*   **Architecture:** Typically x86_64 (ensure you match the library build in `vendor/jres_solver/lib/`)
*   **Reverse Proxy:** Apache2 (configured to proxy to `localhost:8080`)
*   **Permissions:** Root/sudo access for installation

### Automated Installation

1.  **Transfer Source:** Copy the project source code to your Debian server.
2.  **Verify Linux Library:** Ensure the `libjres_solver.a` in `vendor/jres_solver/lib/` is the Linux version.
3.  **Run Installer:**
    ```bash
    chmod +x install.sh
    sudo ./install.sh
    ```

The script will automatically install dependencies, create a dedicated `jres` user, build the service in release mode, and configure a `systemd` unit (`jres_solver.service`) to ensure the service starts at boot and recovers from crashes.

### Service Management

*   **Check status:** `sudo systemctl status jres_solver`
*   **View logs:** `sudo journalctl -u jres_solver -f` (for systemd logs) or check `/var/log/jres_solver/jres_solver.log`
*   **Restart:** `sudo systemctl restart jres_solver`

## Logging

The service uses the `tracing` crate for structured logging.

### Production

In production, the service logs to `/var/log/jres_solver/jres_solver.log`. 
- **Rotation:** Managed by `logrotate` (daily, 14 days retention).
- **Configuration:** Set via the `LOG_DIR` and `RUST_LOG` environment variables in the systemd service file.

### Development

When running locally without the `LOG_DIR` environment variable, logs are directed to **stdout**.

### What is logged

- Service start and stop operations.
- Every request, including:
    - HTTP method and URI.
    - Referrer header.
    - Query parameters (e.g., `spotterMode`, `allowNoSpotter`).
- Unauthorized access attempts.

