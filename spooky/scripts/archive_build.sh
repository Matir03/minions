#!/bin/bash

set -e

# Get the short commit hash
COMMIT_HASH=$(git rev-parse --short HEAD)

# Get the current date in YYYY-MM-DD format
BUILD_DATE=$(date +%Y-%m-%d)

# Define the binary name
BINARY_NAME="spooky_${COMMIT_HASH}_${BUILD_DATE}"

# Define the target directory (project root)
if [ -z "$1" ]; then
    SPOOKY_DIR=$(dirname "$(dirname "$(realpath "$0")")")
else
    SPOOKY_DIR="$1"
fi

# Build the release target
echo "Building release target..."
cargo build --release --manifest-path="${SPOOKY_DIR}/Cargo.toml"

# Define source and destination paths
SOURCE_BINARY="${SPOOKY_DIR}/target/release/spooky"
DEST_BINARY="${SPOOKY_DIR}/bin/${BINARY_NAME}"

# Copy the binary
echo "Copying binary to ${DEST_BINARY}"
cp "${SOURCE_BINARY}" "${DEST_BINARY}"

echo "Build archived successfully as ${BINARY_NAME}"
