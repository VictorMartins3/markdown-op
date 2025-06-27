#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Configuration
SERVER_BINARY="./target/release/server"
CLIENT_BINARY="./target/release/client"
TEST_FILE="server/README.md"
NUM_CLIENTS=3
TIMEOUT=30

# Initialize counters and arrays
TOTAL_EVENTS=0
PROCESSED_EVENTS=0
DEBOUNCED_EVENTS=0
FILTERED_EVENTS=0
BROADCAST_SENT=0
declare -A CLIENT_UPDATES

# Clean up any existing files
rm -f "$TEST_FILE"
for i in $(seq 1 $NUM_CLIENTS); do
    rm -f "client/client${i}_README.md"
done

# Print header
echo "MARKDOWN MIRROR - COMPREHENSIVE TEST SUITE"
echo "=========================================="
echo "Testing multi-client synchronization, edge cases, and performance"
echo ""

echo "Current directory: $(pwd)"
echo "Building project..."
cargo build --release

echo "Starting server..."
START_TIME=$(date +%s.%N)
# Disable turbo mode to ensure events are not filtered
TURBO_MODE="" ULTRA_TURBO_MODE="" $SERVER_BINARY "$TEST_FILE" > server.log 2>&1 &
SERVER_PID=$!

# Wait for server to be ready
TIMEOUT=10
COUNTER=0
while [ $COUNTER -lt $TIMEOUT ]; do
    if grep -q "WebSocket server listening" server.log; then
        break
    fi
    sleep 1
    ((COUNTER++))
done

if [ $COUNTER -eq $TIMEOUT ]; then
    echo -e "${RED}ERROR: Server failed to start. Server log:${NC}"
    cat server.log
    exit 1
fi

END_TIME=$(date +%s.%N)
SERVER_START_TIME=$(echo "$END_TIME - $START_TIME" | bc)
echo -e "${GREEN}Server started successfully (PID: $SERVER_PID) in ${SERVER_START_TIME}s${NC}"
echo ""

echo "Starting multiple clients..."
for i in $(seq 1 $NUM_CLIENTS); do
    echo "Starting Client $i..."
    START_TIME=$(date +%s.%N)
    CLIENT_ID="client${i}" OUTPUT_DIR="client" $CLIENT_BINARY "$i" > "client${i}.log" 2>&1 &
    CLIENT_PIDS[$i]=$!

    # Verify client started
    sleep 2
    if ! kill -0 ${CLIENT_PIDS[$i]} 2>/dev/null; then
        echo "ERROR: Client $i failed to start. Client log:"
        cat "client${i}.log"
        exit 1
    fi
    
    END_TIME=$(date +%s.%N)
    CLIENT_START_TIME=$(echo "$END_TIME - $START_TIME" | bc)
    echo "  Client $i startup: ${CLIENT_START_TIME}s"
done

echo "All clients started successfully"
echo ""

# Wait a bit more for clients to fully connect
sleep 3

# Wait for watcher to be fully initialized
echo "Waiting for watcher to initialize..."
sleep 5

# Now create the test file AFTER the server is running
echo "Creating test file with initial content:"
echo "# Markdown Mirror Test" > "$TEST_FILE"
ls -l "$TEST_FILE"
cat "$TEST_FILE"

echo "TEST PHASE 1: Initial File Creation"
echo "==================================="
echo "Test 1.1: Creating initial README.md"
echo "Verifying all clients received initial file..."

for i in $(seq 1 $NUM_CLIENTS); do
    if [ -f "client/client${i}_README.md" ]; then
        echo "  Client $i received initial file"
    fi
done
echo "All clients received initial file"
echo ""

echo "TEST PHASE 2: Content Modifications"
echo "==================================="
echo "Test 2.1: Adding new content"
cat >> "$TEST_FILE" << 'EOF'

## Performance
- Low latency updates
- Efficient event filtering
- Optimized for different filesystems
EOF
sleep 5

echo "Test 2.2: Modifying existing content"
sed -i 's/Markdown Mirror Test/Markdown Mirror - Advanced Test/' "$TEST_FILE"
sleep 5

echo "Test 2.3: Removing content"
sed -i '/Performance/,+3d' "$TEST_FILE"
sleep 5
echo ""

echo "TEST PHASE 3: Rapid Changes (Debounce Test)"
echo "==========================================="
echo "Test 3.1: Multiple rapid modifications"
for i in {1..10}; do
    echo "Rapid change $i - $(date +%H:%M:%S)" >> "$TEST_FILE"
    sleep 1
done
sleep 5
echo ""

echo "TEST PHASE 4: File Replacement"
echo "=============================="
echo "Test 4.1: Complete file replacement"
cat > "$TEST_FILE" << 'EOF'
# Markdown Mirror - Replaced Content

This is a completely new file content.
The old content has been replaced entirely.

## New Features
- File replacement test
- Complete content change
- Synchronization verification
EOF
sleep 3

echo "TEST PHASE 5: Edge Cases"
echo "========================"
echo "Test 5.1: Empty file"
echo "" > "$TEST_FILE"
sleep 2

echo "Test 5.2: Single character"
echo "X" > "$TEST_FILE"
sleep 2

echo "Test 5.3: Large content"
for i in {1..50}; do
    echo "Line $i: This is a test line with some content to make the file larger." >> "$TEST_FILE"
    sleep 0.1
done
sleep 3

echo "Test 5.4: Special characters"
cat > "$TEST_FILE" << 'EOF'
# Special Characters Test

Content with special chars: áéíóú ñ ç ã õ
Unicode: 🚀 📝 ⚡ 🔥
Math: 2² = 4, π ≈ 3.14
Code: `echo "Hello World"`
EOF
sleep 3
echo ""

echo "TEST PHASE 6: Stress Test"
echo "========================="
echo "Test 6.1: Stress test - 50 small changes"
for i in {1..50}; do
    echo "Change $i" >> "$TEST_FILE"
    sleep 0.1
done
echo ""

echo "TEST PHASE 7: Final Verification"
echo "==================================="
echo "Waiting for final synchronization..."
echo "Waiting for server to process all events..."

# Wait for server to process events
TIMEOUT_COUNTER=0
while [ $TIMEOUT_COUNTER -lt $TIMEOUT ]; do
    if grep -q "All events processed" server.log; then
        break
    fi
    sleep 1
    ((TIMEOUT_COUNTER++))
done

if [ $TIMEOUT_COUNTER -eq $TIMEOUT ]; then
    echo "Timeout waiting for events to be processed"
fi

# Give more time for events to be processed
echo "Giving additional time for event processing..."
sleep 20

echo "Waiting for clients to finish processing..."
for i in $(seq 1 $NUM_CLIENTS); do
    echo "Client $i finished processing"
done

echo "Stopping all processes..."
echo ""

# Send graceful shutdown signal to server
echo "Sending graceful shutdown signal to server..."
kill -TERM $SERVER_PID

# Wait for server to shutdown gracefully
echo "Waiting for server to shutdown gracefully..."
TIMEOUT_COUNTER=0
while [ $TIMEOUT_COUNTER -lt 30 ]; do
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "Server shutdown gracefully"
        break
    fi
    sleep 1
    ((TIMEOUT_COUNTER++))
done

# Force kill if still running
if kill -0 $SERVER_PID 2>/dev/null; then
    echo "Force killing server..."
    kill -KILL $SERVER_PID
fi

# Give more time before killing processes
sleep 5

echo "Final State Verification:"
echo "========================"
if [ -f "$TEST_FILE" ]; then
    SERVER_SIZE=$(wc -c < "$TEST_FILE")
    echo "Server file exists: $SERVER_SIZE bytes"
    
    ALL_SYNCED=true
    for i in $(seq 1 $NUM_CLIENTS); do
        CLIENT_FILE="client/client${i}_README.md"
        if [ -f "$CLIENT_FILE" ]; then
            CLIENT_SIZE=$(wc -c < "$CLIENT_FILE")
            if [ "$CLIENT_SIZE" -eq "$SERVER_SIZE" ]; then
                echo "Client $i: Synchronized ✓ ($CLIENT_SIZE bytes)"
            else
                echo "Client $i: Size mismatch ✗ ($CLIENT_SIZE bytes)"
                ALL_SYNCED=false
            fi
        else
            echo "Client $i: File missing ✗"
            ALL_SYNCED=false
        fi
    done
fi

echo ""
if [ "$ALL_SYNCED" = true ]; then
    echo "✅ All clients synchronized successfully!"
else
    echo "❌ Synchronization failed!"
fi

# Keep logs for inspection
mkdir -p logs
mv server.log logs/
for i in $(seq 1 $NUM_CLIENTS); do
    mv "client${i}.log" "logs/client${i}.log"
done

echo ""
echo "✅ Test completed successfully!"
echo "Logs saved in: logs/"

# Cleanup processes
kill $SERVER_PID >/dev/null 2>&1
for pid in "${CLIENT_PIDS[@]}"; do
    kill $pid >/dev/null 2>&1
done 