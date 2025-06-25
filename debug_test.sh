#!/bin/bash

echo "🚀 Testing Debug Setup for quark-bot"
echo "=================================="

echo "1. Building debug binary locally..."
cargo build --bin quark_bot
if [ $? -ne 0 ]; then
    echo "❌ Failed to build debug binary"
    exit 1
fi
echo "✅ Debug binary built successfully"

echo ""
echo "2. Starting debug Docker container..."
docker-compose -f docker-compose.debug.yml up -d --build quark-bot-debug
if [ $? -ne 0 ]; then
    echo "❌ Failed to start debug container"
    exit 1
fi
echo "✅ Debug container started"

echo ""
echo "3. Waiting for gdbserver to be ready..."
sleep 3

echo ""
echo "4. Testing GDB connection..."
timeout 10s gdb -batch \
    -ex "file target/debug/quark_bot" \
    -ex "target remote localhost:1234" \
    -ex "info breakpoints" \
    -ex "continue" \
    -ex "quit"

if [ $? -eq 0 ]; then
    echo "✅ GDB connection successful!"
else
    echo "❌ GDB connection failed"
fi

echo ""
echo "5. Container logs:"
docker-compose -f docker-compose.debug.yml logs --tail=10 quark-bot-debug

echo ""
echo "🎯 Setup complete! You can now:"
echo "   1. Set a breakpoint on line 33 in quark_bot/src/main.rs"
echo "   2. Use VS Code: Ctrl+Shift+D -> 'Debug quark_bot in Docker (Simple)' -> F5"
echo "   3. Or connect manually: gdb target/debug/quark_bot -> target remote localhost:1234"
echo ""
echo "To stop debug container: docker-compose -f docker-compose.debug.yml down" 