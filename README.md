# Markdown Mirror

Real-time file synchronization system in Rust. Watches a Markdown file and pushes changes to connected clients via WebSocket.

## What it does

- **Server**: Watches a file and broadcasts changes via WebSocket
- **Client**: Connects to server and keeps a local copy in sync
- **Shared**: Common types and diff algorithms

## Quick Start

```bash
# Build
cargo build --release

# Start server (watches README.md by default)
./target/release/server

# Start client
CLIENT_ID="1" OUTPUT_DIR="client" ./target/release/client 1
```

## Project Structure

```
server/src/
├── main.rs      # Server entry point
├── watcher.rs   # File system monitoring
└── websocket.rs # WebSocket handling

client/src/
└── main.rs      # Client implementation

shared/src/
└── lib.rs       # Shared types and diff algorithm
```

## Testing

```bash
# Run comprehensive test suite
./test.sh
```

## Manual Testing

Follow these steps to test the system manually:

### 1. Build the Project
```bash
cargo build --release
```

### 2. Create a Test File
```bash
echo "# Teste do Markdown Mirror" > README.md
```

### 3. Start the Server
```bash
# In terminal 1
cargo run --release -p server &
```

### 4. Start the Client
```bash
# In terminal 2
cargo run --release -p client
```

You should see:
```
Starting Markdown Mirror Client
Client ID: 1
Output directory: client
Connected to server
Updated file: client/client1_README.md
```

### 5. Verify Initial Sync
```bash
cat client/client1_README.md
# Should show: "# Teste do Markdown Mirror"
```

### 6. Test Real-time Synchronization
```bash
# Add content to the original file
echo -e "\n## Nova seção\nEsta é uma linha nova para testar." >> README.md

# Wait a few seconds and check if client received the update
sleep 3
cat client/client1_README.md
```

### 7. Test Multiple Changes
```bash
# Make more changes
echo -e "\n### Subseção\n- Item 1\n- Item 2" >> README.md

# Verify synchronization
sleep 2
cat client/client1_README.md
```

### 8. Clean Up
```bash
# Stop server and client
pkill -f "target/release/server"
pkill -f "target/release/client"
```

### Expected Results
- ✅ Client connects to server successfully
- ✅ Initial file content is synchronized
- ✅ Changes are applied in real-time (within 1-3 seconds)
- ✅ Client file matches server file exactly
- ✅ No data corruption or loss

## How it works

1. Server watches a file using `notify` crate
2. When file changes, server creates diffs and broadcasts via WebSocket
3. Clients receive changes and apply them to local files
4. Debouncing prevents excessive updates from rapid changes

## Configuration

- **Port**: 3030 (change in `shared/src/lib.rs`)
- **Debounce**: 25ms (change in `server/src/watcher.rs`)
- **Client output**: Set via `OUTPUT_DIR` env var

## Example

```bash
# Terminal 1: Start server
./target/release/server my-file.md

# Terminal 2: Start client
CLIENT_ID="1" OUTPUT_DIR="client" ./target/release/client 1

# Terminal 3: Edit file
echo "New content" >> my-file.md
# Client automatically receives and applies the change
```
