#!/bin/bash

# Script to upload/update AI files to Google Cloud Storage
# Make sure you have the required environment variables set:
# - BUCKET: The Google Cloud Storage bucket name
# - PROJECT_ID: (optional) The Google Cloud project ID
# - GOOGLE_APPLICATION_CREDENTIALS: Path to service account key file (optional, uses ADC if not set)

set -e

echo "🚀 Building and running AI files upload/update script..."

# Build the project
cargo build --release

# Run the upload/update script
echo "📤 Starting upload/update process..."
./target/release/quark-scripts --upload

echo "✅ Upload/update script completed!"
