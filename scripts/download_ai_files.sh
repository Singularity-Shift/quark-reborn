#!/bin/bash

# Script to download AI files from Google Cloud Storage
# Make sure you have the required environment variables set:
# - BUCKET: The Google Cloud Storage bucket name
# - PROJECT_ID: (optional) The Google Cloud project ID
# - GOOGLE_APPLICATION_CREDENTIALS: Path to service account key file (optional, uses ADC if not set)

set -e

echo "🚀 Building and running AI files download script..."

# Build the project
cargo build --release

# Run the download script
echo "📥 Starting download process..."
./target/release/quark-scripts --download

echo "✅ Download script completed!"
