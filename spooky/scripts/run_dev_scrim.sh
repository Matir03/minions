#!/bin/bash

set -e

# Get the directory of the script
SCRIPT_DIR=$(dirname "$(realpath "$0")")
SPOOKY_DIR=$(dirname "$SCRIPT_DIR")

# Build the latest release version
echo "Building current code..."
cargo build --release --manifest-path="${SPOOKY_DIR}/Cargo.toml"

# Run the scrimmage
echo "\nStarting development scrimmage..."
"${SPOOKY_DIR}/scrims/venv/bin/python3" "${SPOOKY_DIR}/scrims/run_scrim.py" "${SPOOKY_DIR}/scrims/configs/dev_scrim.toml"
