#!/bin/bash
set -e

# Configuration
SERVICE_NAME="jres_solver"
LOG_DIR="/home/jules/json.racing/jres_solver_service/service/logs"
BINARY_NAME="jres_solver_service"
APACHE_SITE_CONFIG="/etc/apache2/sites-available/json.racing.conf"

echo "Checking dependencies..."
for cmd in build-essential curl pkg-config cmake git unzip rsync logrotate pm2; do
    if ! command -v $cmd &> /dev/null;
    then
        echo "Missing dependency: $cmd. Please install it as root."
    fi
done

# Setup log directory
echo "Setting up local log directory at $LOG_DIR..."
mkdir -p $LOG_DIR

# Ensure config.yaml exists
if [ ! -f "config.yaml" ]; then
    echo "Creating config.yaml from config.prod.yaml..."
    ln -s config.yaml config.prod.yaml
fi

# Setup request directory if configured
REQUEST_DIR=$(grep "requestDirectory:" config.yaml | awk -F'"' '{print $2}')
if [ ! -z "$REQUEST_DIR" ]; then
    echo "Setting up request directory at $REQUEST_DIR..."
    mkdir -p "$REQUEST_DIR"
fi

# Setup local Highs dependency
HIGHS_DIR="vendor/highs"
REQUIRED_HIGHS_URL="https://github.com/popmonkey/jres_solver_cpp/releases/download/highs-static-v1.12.0/highs-v1.12.0-linux-x64.zip"

if [ ! -d "$HIGHS_DIR" ]; then
    echo "Setting up local Highs dependency in $HIGHS_DIR..."
    mkdir -p $HIGHS_DIR
    TMP_DIR=$(mktemp -d)
    curl -L -o $TMP_DIR/highs.zip "$REQUIRED_HIGHS_URL"
    unzip -q $TMP_DIR/highs.zip -d $HIGHS_DIR
    rm -rf $TMP_DIR
fi

# Build the service
echo "Building the service (release mode)..."
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi
cargo build --release

# Generate package.json with version info
VERSION=$(./target/release/jres_solver_service --version)
echo "Detected Version: $VERSION"
echo "{
  \"name\": \"jres_solver\",
  \"version\": \"$VERSION\",
  \"description\": \"JRES Solver Service\",
  \"private\": true
}" > package.json

# PM2 Deployment
echo "Deploying with PM2..."
pm2 startOrReload ecosystem.config.js --update-env
pm2 save

# Instructions for Apache
SERVICE_PORT=$(grep "port:" config.yaml | awk '{print $2}' | tr -d '[:space:]')
echo ""
echo "--- MANUAL STEP REQUIRED ---"
echo "To update the Apache proxy, ensure $APACHE_SITE_CONFIG contains:"
echo ""
echo "    ProxyPass /api/solve http://127.0.0.1:$SERVICE_PORT/solve"
echo "    ProxyPassReverse /api/solve http://127.0.0.1:$SERVICE_PORT/solve"
echo ""
echo "Then run: sudo apachectl configtest && sudo systemctl reload apache2"
echo "----------------------------"

echo "Local installation complete!"
pm2 list
