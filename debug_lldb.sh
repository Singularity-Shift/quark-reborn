#!/bin/bash

# LLDB Debug Script for Quark Bot
# This script helps you connect to the LLDB debug session running in Docker

echo "🔧 Setting up LLDB debugging for Quark Bot..."

# Build the debug image first
echo "📦 Building LLDB debug image..."
docker-compose -f docker-compose.debug.yml build quark-bot-lldb

# Start the debug service
echo "🚀 Starting LLDB debug service..."
docker-compose -f docker-compose.debug.yml up -d quark-bot-lldb

# Wait a moment for the service to start
sleep 3

echo "📡 LLDB debug server is running on port 1235"
echo ""
echo "To connect with LLDB from your host machine:"
echo "  1. Install LLDB on your host (if not already installed)"
echo "  2. Run: lldb target/debug/quark_bot"
echo "  3. In LLDB, run: gdb-remote localhost:1235"
echo ""
echo "Useful LLDB commands:"
echo "  - (lldb) breakpoint set --name main"
echo "  - (lldb) breakpoint set --file handler.rs --line 100"
echo "  - (lldb) continue"
echo "  - (lldb) thread backtrace"
echo "  - (lldb) frame variable"
echo "  - (lldb) step"
echo "  - (lldb) next"
echo ""
echo "To stop debugging:"
echo "  docker-compose -f docker-compose.debug.yml stop quark-bot-lldb"
echo ""
echo "To view logs:"
echo "  docker-compose -f docker-compose.debug.yml logs -f quark-bot-lldb" 