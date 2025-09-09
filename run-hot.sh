#!/bin/bash

# Hot reload script for Forge development
# This will automatically rebuild and restart when code changes

echo "Starting Forge with hot reload..."
echo "Press Ctrl+C to stop"
echo ""

source "$HOME/.cargo/env"

# Use cargo-watch to automatically rebuild and run on changes
# -x run: Execute 'cargo run' command
# -c: Clear screen before each run
# -w src: Watch the src directory for changes
cargo watch -c -w src -x run