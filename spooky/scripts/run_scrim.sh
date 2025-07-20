#!/bin/bash

set -e

# Get the directory of the script
SCRIPT_DIR=$(dirname "$(realpath "$0")")
SPOOKY_DIR=$(dirname "$SCRIPT_DIR")

if [ -z "$1" ]; then
    CONFIG="${SPOOKY_DIR}/scrims/configs/self_play.toml"
else
    CONFIG="$1"
fi

# Build the latest release version
echo "Building current code..."
cargo build --release --manifest-path="${SPOOKY_DIR}/Cargo.toml"

# Run the scrimmage
echo "\nStarting scrimmage..."
"${SPOOKY_DIR}/scrims/venv/bin/python3" "${SPOOKY_DIR}/scrims/run_scrim.py" "${CONFIG}"
