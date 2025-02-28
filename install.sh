#!/bin/bash

set -e

echo "Checking for virtualen..."
if ! command -v virtualenv &> /dev/null
then
    echo "virtualenv is not installed. Please install virtualenv before running this script."
    exit 1
fi

virtualenv env
source env/bin/activate

echo "Installing Python dependencies..."
pip3 install torch numpy
pip3 install yfinance

echo "Building Rust project..."
cargo build --release

echo "Installation complete!"

