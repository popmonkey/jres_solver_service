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
# Install build tools
apt-get install -y build-essential curl pkg-config cmake git unzip rsync

# Setup local Highs dependency
HIGHS_DIR="vendor/highs"
REQUIRED_HIGHS_URL="https://github.com/popmonkey/jres_solver_cpp/releases/download/highs-static-v1.12.0/highs-v1.12.0-linux-x64.zip"

if [ ! -d "$HIGHS_DIR" ]; then
    echo "Setting up local Highs dependency in $HIGHS_DIR..."
    mkdir -p $HIGHS_DIR
    
    TMP_DIR=$(mktemp -d)
    echo "Downloading prebuilt Highs..."
    curl -L -o $TMP_DIR/highs.zip "$REQUIRED_HIGHS_URL"
    
    echo "Extracting Highs..."
    unzip -q $TMP_DIR/highs.zip -d $HIGHS_DIR
    
    rm -rf $TMP_DIR
    echo "Highs setup complete."
else
    echo "Local Highs dependency found in $HIGHS_DIR."
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
rsync -av --exclude='target' ./ "$INSTALL_DIR/"
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
systemctl restart $SERVICE_NAME

# Configure Apache Proxy
if [ -d "/etc/apache2" ]; then
    echo "Configuring Apache reverse proxy..."

    # Enable required modules
    if command -v a2enmod &> /dev/null; then
        a2enmod proxy proxy_http
    fi

    # Extract port from config.yaml
    CONFIG_PATH="$INSTALL_DIR/config.yaml"
    SERVICE_PORT="8080" # Default
    if [ -f "$CONFIG_PATH" ]; then
        DETECTED_PORT=$(grep "port:" "$CONFIG_PATH" | awk '{print $2}' | tr -d '[:space:]')
        if [ ! -z "$DETECTED_PORT" ]; then
            SERVICE_PORT="$DETECTED_PORT"
        fi
    fi
    echo "Using service port: $SERVICE_PORT"

    CONF_FILE="/etc/apache2/sites-enabled/json.racing.conf"
    APACHE_CHANGED=false

    if [ -f "$CONF_FILE" ]; then
        if ! grep -q "ProxyPass /api/solve" "$CONF_FILE"; then
            echo "Adding proxy configuration to $CONF_FILE..."
            # Insert proxy settings before the closing VirtualHost tag
            sed -i "/<\/VirtualHost>/i \\
    ProxyPreserveHost On\\
    ProxyPass /api/solve http://127.0.0.1:$SERVICE_PORT/solve\\
    ProxyPassReverse /api/solve http://127.0.0.1:$SERVICE_PORT/solve" "$CONF_FILE"
            APACHE_CHANGED=true
        else
            echo "Proxy configuration already exists in $CONF_FILE."
        fi
    else
        echo "Creating $CONF_FILE..."
        cat > "$CONF_FILE" <<EOF
<VirtualHost *:80>
    ServerName json.racing
    DocumentRoot "/home/www/json.racing"
    CustomLog "/var/log/apache2/json.racing.access_log" combined

    ProxyPreserveHost On
    ProxyPass /api/solve http://127.0.0.1:$SERVICE_PORT/solve
    ProxyPassReverse /api/solve http://127.0.0.1:$SERVICE_PORT/solve
</VirtualHost>
EOF
        APACHE_CHANGED=true
    fi

    # Reload Apache to apply changes
    if [ "$APACHE_CHANGED" = true ] && systemctl is-active --quiet apache2; then
        systemctl reload apache2
        echo "Apache reloaded."
    fi
else
    echo "Apache not found, skipping proxy configuration."
fi

echo "Installation complete!"
echo "Status:"
systemctl status $SERVICE_NAME --no-pager
