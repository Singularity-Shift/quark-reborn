# Debugging quark-bot in Docker

This guide explains how to debug the `quark-bot` service running in Docker Compose using VS Code.

## Setup

The debugging configuration includes:

1. **`quark_bot/Dockerfile.debug`** - Debug version of Dockerfile with debug symbols and gdbserver
2. **`docker-compose.debug.yml`** - Debug version of docker-compose with exposed debug port
3. **`.vscode/launch.json`** - Debug configurations for VS Code
4. **`.vscode/tasks.json`** - Tasks for managing debug containers

## Prerequisites

- VS Code with the following extensions:
  - **rust-analyzer** - Rust language support
  - **CodeLLDB** - LLDB debugger for Rust (recommended)
  - **C/C++** - Alternative GDB debugger support
- Docker and Docker Compose
- Environment variables configured (same as production)

## Debugging Steps

### Method 1: Using CodeLLDB (Recommended)

1. **Start the debug container:**
   ```bash
   docker-compose -f docker-compose.debug.yml up -d --build quark-bot-debug
   ```

2. **Set breakpoints** in your Rust code:
   - **Easy test**: Set a breakpoint on line 33 in `quark_bot/src/main.rs` (the `log::info!("Starting quark_bot...");` line)
   - **Other locations**: Any line in `quark_bot/src/` or `quark_core/src/`

3. **Start debugging:**
   - Go to VS Code's Run and Debug panel (Ctrl+Shift+D)
   - Select "Debug quark_bot in Docker" configuration
   - Press F5 or click the play button

### Method 2: Using GDB

1. **Start the debug container:**
   ```bash
   docker-compose -f docker-compose.debug.yml up -d --build quark-bot-debug
   ```

2. **Set breakpoints** in your Rust code

3. **Start debugging:**
   - Go to VS Code's Run and Debug panel (Ctrl+Shift+D)  
   - Select "Debug quark_bot in Docker (GDB)" configuration
   - Press F5 or click the play button

### Method 3: Manual Steps

1. **Build and start debug container:**
   ```bash
   # Build the debug image
   docker-compose -f docker-compose.debug.yml build quark-bot-debug
   
   # Start the container (it will wait for debugger connection)
   docker-compose -f docker-compose.debug.yml up quark-bot-debug
   ```

2. **Connect debugger manually:**
   - The container exposes port 1234 for debugging
   - Use any GDB-compatible debugger to connect to `localhost:1234`

## Useful Commands

```bash
# View logs from debug container
docker-compose -f docker-compose.debug.yml logs -f quark-bot-debug

# Stop debug containers
docker-compose -f docker-compose.debug.yml down

# Rebuild debug container
docker-compose -f docker-compose.debug.yml build --no-cache quark-bot-debug

# Start only specific services for debugging
docker-compose -f docker-compose.debug.yml up -d quark-server
docker-compose -f docker-compose.debug.yml up quark-bot-debug
```

## Debugging Features

- **Debug symbols**: Built with `cargo build` (debug mode) for full symbol information
- **Source mapping**: Source code is mounted as read-only volumes for live editing
- **Environment variables**: Same as production environment
- **Network access**: Full access to other services (quark-server, etc.)
- **Breakpoints**: Full breakpoint support in VS Code
- **Variable inspection**: Inspect variables, call stack, and memory
- **Step debugging**: Step through code line by line

## Troubleshooting

### Port already in use
```bash
# Kill any processes using port 1234
sudo lsof -ti:1234 | xargs kill -9
```

### Container not starting
```bash
# Check container logs
docker-compose -f docker-compose.debug.yml logs quark-bot-debug

# Check if gdbserver is running
docker-compose -f docker-compose.debug.yml exec quark-bot-debug ps aux
```

### Debugger not connecting
1. Ensure the debug container is running and waiting for connection
2. Check that port 1234 is exposed and accessible
3. Verify that the binary was built with debug symbols
4. Try connecting manually with gdb: `gdb -ex "target remote localhost:1234"`

### Source code not matching
1. Ensure the source code hasn't changed since the container was built
2. Rebuild the container if source code was modified
3. Check that source volumes are properly mounted

## Environment Variables

Make sure your `.env` file contains all necessary environment variables:

```bash
TELOXIDE_TOKEN=your_bot_token
OPENAI_API_KEY=your_openai_key
STORAGE_CREDENTIALS=your_gcs_credentials
GCS_BUCKET_NAME=your_bucket
APTOS_NETWORK=devnet
# ... other variables
```

## Performance Notes

- Debug builds are slower than release builds
- The debug container includes additional debugging tools
- Source code volumes allow for live editing but require container restart for changes 