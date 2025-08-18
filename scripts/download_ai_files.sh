#!/bin/bash

# Script to download AI files from Google Drive and replace them in the project
# This script compiles and runs the Rust downloader using Google Drive API

set -e

echo "🔧 Setting up AI files downloader..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: This script must be run from the project root directory"
    echo "   Current directory: $(pwd)"
    echo "   Expected: quark-reborn/"
    exit 1
fi

# Change to scripts directory
cd scripts

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust/Cargo is not installed"
    echo "   Please install Rust from https://rustup.rs/"
    exit 1
fi

# Compile the downloader
echo "📦 Compiling downloader..."
cargo build --release

# Run the downloader
echo "🚀 Running downloader..."
./target/release/download_ai_files

# Check if successful
if [ $? -eq 0 ]; then
    echo "✅ AI files download completed successfully!"
    echo "📁 Files have been replaced in their respective locations"
    echo "💾 Original files have been backed up with .backup extension"
else
    echo "❌ Download failed. Check the error messages above."
    exit 1
fi
