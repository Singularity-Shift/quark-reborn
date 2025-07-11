# LLDB Debugging Guide for Quark Bot

This guide explains how to debug the `quark_bot` using LLDB in a Docker container.

## Prerequisites

1. **LLDB installed on your host machine**:
   - **Arch Linux**: `sudo pacman -S lldb`
   - **Ubuntu/Debian**: `sudo apt install lldb`
   - **macOS**: Included with Xcode Command Line Tools
   - **VS Code**: Install the `CodeLLDB` extension

## Quick Start

### Option 1: Using the Script

```bash
# Make the script executable (if not already done)
chmod +x debug_lldb.sh

# Run the debug setup script
./debug_lldb.sh
```

### Option 2: Manual Setup

```bash
# Build the LLDB debug image
docker-compose -f docker-compose.debug.yml build quark-bot-lldb

# Start the LLDB debug container
docker-compose -f docker-compose.debug.yml up -d quark-bot-lldb
```

## Connecting with LLDB

### Command Line LLDB

1. **Build your local debug binary** (must match the container):
   ```bash
   cargo build --bin quark_bot
   ```

2. **Start LLDB with the binary**:
   ```bash
   lldb target/debug/quark_bot
   ```

3. **Connect to the remote debug server**:
   ```lldb
   (lldb) gdb-remote localhost:1235
   ```

4. **Set breakpoints and debug**:
   ```lldb
   (lldb) breakpoint set --name main
   (lldb) breakpoint set --file handler.rs --line 100
   (lldb) continue
   ```

### VS Code LLDB

1. **Open the project in VS Code**
2. **Install the CodeLLDB extension** (if not already installed)
3. **Go to Run and Debug (Ctrl+Shift+D)**
4. **Select "LLDB Remote Debug (Docker)"**
5. **Press F5 to start debugging**

## Useful LLDB Commands

| Command | Description |
|---------|-------------|
| `breakpoint set --name function_name` | Set breakpoint on function |
| `breakpoint set --file file.rs --line 100` | Set breakpoint on specific line |
| `breakpoint list` | List all breakpoints |
| `breakpoint delete 1` | Delete breakpoint by ID |
| `continue` or `c` | Continue execution |
| `step` or `s` | Step into |
| `next` or `n` | Step over |
| `finish` | Step out |
| `thread backtrace` or `bt` | Show call stack |
| `frame variable` or `v` | Show local variables |
| `frame variable var_name` | Show specific variable |
| `expression var_name = new_value` | Modify variable |
| `target modules lookup --address 0x...` | Find symbol at address |

## Configuration Details

### Docker Compose Service: `quark-bot-lldb`

- **Port**: `1235` (mapped to container port `1234`)
- **Dockerfile**: `quark_bot/Dockerfile.lldb`
- **Debug Server**: `lldb-server gdbserver :1234 ./quark_bot`
- **Capabilities**: `SYS_PTRACE` for debugging
- **Security**: `seccomp:unconfined` for debugging

### VS Code Tasks

- **Start LLDB Debug Container**: Starts the LLDB debug service
- **Start GDB Debug Container**: Starts the GDB debug service (alternative)
- **Stop Debug Containers**: Stops both debug services
- **Build Debug Images**: Rebuilds both debug images

## Troubleshooting

### Connection Issues

1. **Check if the container is running**:
   ```bash
   docker-compose -f docker-compose.debug.yml ps
   ```

2. **Check container logs**:
   ```bash
   docker-compose -f docker-compose.debug.yml logs quark-bot-lldb
   ```

3. **Verify port binding**:
   ```bash
   netstat -tulpn | grep 1235
   ```

### Binary Mismatch

If you get "binary mismatch" errors:

1. **Rebuild both local and container binaries**:
   ```bash
   cargo build --bin quark_bot
   docker-compose -f docker-compose.debug.yml build quark-bot-lldb
   ```

2. **Ensure same Rust version** (container uses Rust 1.85)

### LLDB Won't Connect

1. **Check if LLDB server is listening**:
   ```bash
   docker exec -it quark-reborn-quark-bot-lldb-1 netstat -tlpn
   ```

2. **Restart the debug container**:
   ```bash
   docker-compose -f docker-compose.debug.yml restart quark-bot-lldb
   ```

## Environment Variables

The debug container uses the same environment variables as the main bot:

- `TELOXIDE_TOKEN` - Telegram bot token
- `OPENAI_API_KEY` - OpenAI API key
- `APTOS_NETWORK` - Aptos network (mainnet/testnet/devnet)
- `CONTRACT_ADDRESS` - Smart contract address
- `MIN_DEPOSIT` - Minimum deposit amount in USD
- `PANORA_URL` - Panora API URL
- `PANORA_API_KEY` - Panora API key
- And more...

## Stopping Debug Session

```bash
# Stop the LLDB debug container
docker-compose -f docker-compose.debug.yml stop quark-bot-lldb

# Or stop all debug containers
docker-compose -f docker-compose.debug.yml stop
``` 