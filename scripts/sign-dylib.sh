#!/bin/bash
# Sign the dylib for local development
# This script helps avoid macOS security warnings when using the library

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DYLIB_PATH="$PROJECT_ROOT/resources/libmediaremote_rs.dylib"

if [ ! -f "$DYLIB_PATH" ]; then
    echo "Error: dylib not found at $DYLIB_PATH"
    echo "Please run 'cargo build --release' first"
    exit 1
fi

echo "Signing dylib at: $DYLIB_PATH"
codesign --force --deep -s - "$DYLIB_PATH"

echo "Verifying signature..."
codesign -dv "$DYLIB_PATH"

echo "✅ Dylib signed successfully!"
