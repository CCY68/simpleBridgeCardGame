#!/bin/bash
#
# CardArena Automated Demo Script
# 自動執行完整遊戲 (1 Auto-Human + 3 AI)，無需人工互動
#
# Usage:
#   ./run_auto_demo.sh [--port PORT]
#

set -e

PORT="${1:-8888}"
if [ "$1" = "--port" ]; then
    PORT="$2"
fi

# 顏色
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

cd "$(dirname "$0")/.."
PROJECT_ROOT=$(pwd)

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  CardArena Automated Demo${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# 清理
cleanup() {
    pkill -f "card_arena_server" 2>/dev/null || true
    pkill -f "integration_test.py" 2>/dev/null || true
}
trap cleanup EXIT

# Build
echo -e "${GREEN}[1/3]${NC} Building server..."
cd "$PROJECT_ROOT/server"
cargo build --release 2>&1 | grep -E "Compiling|Finished" || true
cd "$PROJECT_ROOT"

# Start Server
echo -e "${GREEN}[2/3]${NC} Starting server..."
cd "$PROJECT_ROOT/server"
RUST_LOG=info cargo run --release -- --port "$PORT" &
SERVER_PID=$!
cd "$PROJECT_ROOT"
sleep 2

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Server failed to start!"
    exit 1
fi

# Run Integration Test
echo -e "${GREEN}[3/3]${NC} Running automated game..."
echo ""
python3 "$PROJECT_ROOT/scripts/integration_test.py"

echo ""
echo -e "${GREEN}Demo completed successfully!${NC}"
