#!/bin/bash
set -e

# Configuration
SERVICE_NAME="jres_solver"
INSTALL_DIR="/opt/jres_solver_service"
USER_NAME="jres"
BINARY_NAME="jres_solver_service"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
  echo "Please run as root"
  exit 1
fi

echo "Updating package lists..."
apt-get update

echo "Installing dependencies..."
# Install build tools and Highs dependency
# Note: Debian 13 (Trixie) might have libhighs-dev. If not, it needs to be built from source.
if apt-cache show libhighs-dev >/dev/null 2>&1; then
    apt-get install -y build-essential curl pkg-config libhighs-dev
else
    echo "Warning: libhighs-dev not found in apt repositories. Please install Highs manually."
    echo "Attempting to continue with build essentials only..."
    apt-get install -y build-essential curl pkg-config
fi

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Create service user
if ! id "$USER_NAME" &>/dev/null; then
    echo "Creating service user '$USER_NAME'..."
    useradd -r -s /bin/false $USER_NAME
fi

# Prepare install directory
echo "Setting up installation directory at $INSTALL_DIR..."
mkdir -p $INSTALL_DIR
# We assume the script is run from inside the source directory
cp -r . $INSTALL_DIR/
chown -R $USER_NAME:$USER_NAME $INSTALL_DIR

# Build the service
echo "Building the service (release mode)..."
cd $INSTALL_DIR
# Ensure we are using the correct user for building if possible, or build as root and fix permissions
# Building as root is easier for this script, but strictly speaking we should fix ownership after target creation.
source $HOME/.cargo/env
cargo build --release

# Ensure permissions are correct after build
chown -R $USER_NAME:$USER_NAME $INSTALL_DIR

# Create systemd service file
echo "Creating systemd service file..."
cat > /etc/systemd/system/${SERVICE_NAME}.service <<EOF
[Unit]
Description=JRES Solver Service
After=network.target

[Service]
Type=simple
User=$USER_NAME
Group=$USER_NAME
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/target/release/$BINARY_NAME
Restart=always
RestartSec=5
# Environment variables if needed
# Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
echo "Reloading systemd daemon..."
systemctl daemon-reload

echo "Enabling service to start on boot..."
systemctl enable $SERVICE_NAME

echo "Starting service..."
systemctl start $SERVICE_NAME

echo "Installation complete!"
echo "Status:"
systemctl status $SERVICE_NAME --no-pager
