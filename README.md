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

## Configuration

The service behavior and environment can be adjusted via several configuration files.

### 1. Application Settings (`config.yaml`)

This file (copied from `config.prod.yaml` or `config.dev.yaml` during installation) controls the solver defaults and server networking.

*   **`solve` Section:**
    *   `timeLimit`: Maximum seconds the solver can run (default: `5`).
    *   `optimalityGap`: The target gap for the solver (default: `0.2`).
    *   `roleCouplingWeight`: Weight for role coupling constraints.
    *   `rotationBeatWeight`: Weight for rotation beat constraints.
    *   `spotterMode`: Default spotter logic mode.
    *   `allowNoSpotter`: Whether to allow solutions without a spotter.
*   **`server` Section:**
    *   `ip`: The IP address to bind to (usually `127.0.0.1`).
    *   `port`: The port the service listens on (default: `11080`).
    *   `requestDirectory`: (Optional) Path to a directory where raw JSON requests and results will be saved for debugging.

### 2. Security (`jres_api_key.txt`)

Create a file named `jres_api_key.txt` in the project root (or installation directory) containing a single string. This key must be provided by clients in the `X-API-KEY` HTTP header.

### 3. Installation Script (`install.sh`)

If you are deploying to a new site, you may need to edit the variables at the top of `install.sh`:

*   `SERVICE_NAME`: The name of the `systemd` service (default: `jres_solver`).
*   `INSTALL_DIR`: Where the code and binary will reside (default: `/opt/jres_solver_service`).
*   `LOG_DIR`: Where the application logs will be written (default: `/var/log/jres_solver`).
*   `USER_NAME`: The system user that will run the service (default: `jres`).
*   `APACHE_SITE_CONFIG`: The path to your Apache VirtualHost configuration file (e.g., `/etc/apache2/sites-available/your-site.conf`). The installer will append proxy rules to this file.

## Production Setup

For production deployment on Linux, you can use the provided automated installation script.

>[!NOTE]
>The install script has only been tested on Debian Trixie

### Requirements

*   **OS:** Debian (Linux)
*   **Architecture:** Typically x86_64 (ensure you match the library build in `vendor/jres_solver/lib/`)
*   **Reverse Proxy:** Apache2
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

>[!NOTE]
>The install script has only been tested on Debian Trixie

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

