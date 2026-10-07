#!/bin/sh
set -e

echo "=================================================="
echo "          Starting OBS HomeRun                    "
echo "=================================================="

# Function to handle shutdown signals
cleanup() {
    echo ""
    echo "Stopping OBS HomeRun services..."
    if [ -n "$BROADCASTER_PID" ]; then
        kill -TERM "$BROADCASTER_PID" 2>/dev/null || true
    fi
    if [ -n "$MEDIAMTX_PID" ]; then
        kill -TERM "$MEDIAMTX_PID" 2>/dev/null || true
    fi
    wait "$BROADCASTER_PID" 2>/dev/null || true
    wait "$MEDIAMTX_PID" 2>/dev/null || true
    echo "OBS HomeRun stopped."
    exit 0
}

trap cleanup INT TERM

# Start MediaMTX in background
echo "[Init] Starting MediaMTX RTMP/RTSP ingest engine..."
/app/mediamtx /app/mediamtx.yml &
MEDIAMTX_PID=$!

# Brief pause to let MediaMTX bind ports
sleep 1

# Start the HDHomeRun / DLNA Broadcaster in foreground
echo "[Init] Starting DLNA / HDHomeRun broadcaster..."
python3 -u /app/broadcaster.py &
BROADCASTER_PID=$!

# Wait for broadcaster process
wait "$BROADCASTER_PID"
cleanup
