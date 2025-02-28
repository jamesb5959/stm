#!/bin/bash

set -e

echo "Checking for pip3..."
if ! command -v pip3 &> /dev/null
then
    echo "pip3 is not installed. Please install pip3 before running this script."
    exit 1
fi

echo "Installing Python dependencies..."
pip3 install torch numpy
pip3 install yfinance

echo "Building Rust project..."
cargo build --release

echo "Installation complete!"

