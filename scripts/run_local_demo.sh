#!/bin/bash
#
# CardArena Local Demo Script
# 啟動 Server + 1 Human CLI + 3 AI Clients 進行完整遊戲
#
# Usage:
#   ./run_local_demo.sh [OPTIONS]
#
# Options:
#   --port PORT      Server port (default: 8888)
#   --humans N       Number of human players (default: 1, max: 4)
#   --seed SEED      Random seed for dealing (default: random)
#   --no-build       Skip cargo build
#   --help           Show this help
#

set -e

# 預設值
PORT=8888
HUMANS=1
SEED=""
SKIP_BUILD=false
SERVER_PID=""
CLIENT_PIDS=()

# 顏色輸出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_help() {
    head -17 "$0" | tail -14
    exit 0
}

cleanup() {
    log_info "Cleaning up processes..."

    # 停止 clients
    for pid in "${CLIENT_PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done

    # 停止 server
    if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
    fi

    # 確保清理乾淨
    pkill -f "card_arena_server" 2>/dev/null || true
    pkill -f "human_cli/app.py" 2>/dev/null || true
    pkill -f "ai_cli/app.py" 2>/dev/null || true

    log_info "Cleanup complete."
}

trap cleanup EXIT

# 解析參數
while [[ $# -gt 0 ]]; do
    case $1 in
        --port)
            PORT="$2"
            shift 2
            ;;
        --humans)
            HUMANS="$2"
            shift 2
            ;;
        --seed)
            SEED="$2"
            shift 2
            ;;
        --no-build)
            SKIP_BUILD=true
            shift
            ;;
        --help)
            show_help
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            ;;
    esac
done

# 驗證參數
if [ "$HUMANS" -lt 1 ] || [ "$HUMANS" -gt 4 ]; then
    log_error "Humans must be between 1 and 4"
    exit 1
fi

AI_COUNT=$((4 - HUMANS))

# 切換到專案根目錄
cd "$(dirname "$0")/.."
PROJECT_ROOT=$(pwd)

echo ""
echo "=========================================="
echo -e "${BLUE}   CardArena Local Demo${NC}"
echo "=========================================="
echo "Port:    $PORT"
echo "Humans:  $HUMANS"
echo "AIs:     $AI_COUNT"
[ -n "$SEED" ] && echo "Seed:    $SEED"
echo "=========================================="
echo ""

# Step 1: Build Server
if [ "$SKIP_BUILD" = false ]; then
    log_info "Building server..."
    cd "$PROJECT_ROOT/server"
    cargo build --release 2>&1 | tail -3
    cd "$PROJECT_ROOT"
fi

# Step 2: Start Server
log_info "Starting server on port $PORT..."
cd "$PROJECT_ROOT/server"
RUST_LOG=info cargo run --release -- --port "$PORT" &
SERVER_PID=$!
cd "$PROJECT_ROOT"

# 等待 server 啟動
sleep 2

# 檢查 server 是否運行
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    log_error "Server failed to start!"
    exit 1
fi
log_info "Server started (PID: $SERVER_PID)"

# Step 3: Start AI Clients
log_info "Starting $AI_COUNT AI client(s)..."
for i in $(seq 1 $AI_COUNT); do
    python3 "$PROJECT_ROOT/clients/ai_cli/app.py" \
        --host 127.0.0.1 \
        --port "$PORT" \
        --name "Bot_$i" \
        --token "secret" \
        --no-llm &
    CLIENT_PIDS+=($!)
    sleep 0.3
done

# Step 4: Start Human CLI Client(s)
if [ "$HUMANS" -gt 0 ]; then
    log_info "Starting Human CLI client..."
    echo ""
    echo -e "${YELLOW}======================================${NC}"
    echo -e "${YELLOW}  Human Client - Interactive Mode${NC}"
    echo -e "${YELLOW}======================================${NC}"
    echo ""

    # 第一個 human 在前景執行
    python3 "$PROJECT_ROOT/clients/human_cli/app.py" \
        --host 127.0.0.1 \
        --port "$PORT" \
        --name "Player_1"

    # 如果有多個 human，其他的需要額外終端
    if [ "$HUMANS" -gt 1 ]; then
        log_warn "For multiple human players, please open additional terminals and run:"
        for i in $(seq 2 $HUMANS); do
            echo "  python3 clients/human_cli/app.py --host 127.0.0.1 --port $PORT --name Player_$i"
        done
    fi
fi

echo ""
log_info "Demo session ended."
