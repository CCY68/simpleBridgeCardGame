# CardArena Development Stories

> 本檔案以主 PM 的 QA 進度為主。
> Server/Client 的開發進度請分別參考 `progress/srv_stories.md` 與 `progress/clnt_stories.md`。
> 使用狀態標記追蹤 QA 進度：`TODO` → `IN_PROGRESS` → `DONE`

---

## Ownership Legend

| Tag | Owner | Scope |
|-----|-------|-------|
| `@Claude` | Claude Code | Server (Rust) |
| `@Gemini` | Gemini CLI | Client (Python) |
| `@Shared` | 雙方 | 文件、協調 |

## Status Legend

| Status | Meaning |
|--------|---------|
| `TODO` | 尚未開始 |
| `IN_PROGRESS` | 進行中 |
| `DONE` | 已完成 |
| `BLOCKED` | 被阻擋 |

---

## EPIC 0 - Repository & Documentation Scaffold `@Shared`

### S0.1 Initialize repository structure `[P0]` `DONE` `@Shared`

**依賴**: 無
**檔案**: 專案根目錄
**驗收指令**: `ls -la` 確認目錄結構存在

**DoD**:
- [x] 建立 `server/` 目錄 (Rust)
- [x] 建立 `clients/` 目錄 (Python)
- [x] 建立 `protocol/` 目錄 (文件)
- [x] 建立 `progress/` 目錄 (追蹤)
- [x] 建立 `scripts/` 目錄 (腳本)
- [x] 建立 `.gitignore` (Rust + Python + OS)
- [x] 建立 `developing/` 用於草案與研究文件

---

### S0.2 Create protocol specification `[P0]` `DONE` `@Shared`

**依賴**: S0.1
**檔案**: `protocol/protocol.md`
**驗收指令**: 檢查文件包含所有 message types

**DoD**:
- [x] 定義 NDJSON framing 規則
- [x] 定義所有 message types (HELLO, WELCOME, DEAL, YOUR_TURN, PLAY, etc.)
- [x] 包含範例訊息
- [x] 定義 error codes
- [x] 補齊整合測試規格文件 `protocol/integration_tests.md`

---

### S0.3 Create POSIX mapping document `[P1]` `DONE` `@Shared`

**依賴**: S0.1
**檔案**: `protocol/posix_mapping.md`
**驗收指令**: 表格包含 TCP + UDP lifecycle

**DoD**:
- [x] C/C++ vs Rust 對照表 (socket, bind, listen, accept, etc.)
- [x] 包含程式碼範例
- [x] 說明為何選擇 socket2

---

### S0.4 Create .gitignore `[P0]` `DONE` `@Shared`

**依賴**: S0.1
**檔案**: `.gitignore`

**DoD**:
- [x] Rust: `target/`, `Cargo.lock` (for library), `*.pdb`
- [x] Python: `__pycache__/`, `*.pyc`, `.venv/`, `*.egg-info/`
- [x] OS: `.DS_Store`, `Thumbs.db`
- [x] IDE: `.idea/`, `.vscode/`, `*.swp`

---

## EPIC 1 - TCP Networking Core (Rust Server) `@Claude`

### S1.1 TCP listener setup via socket2 `[P0]` `DONE` `@Claude`

**依賴**: S0.1
**檔案**: `server/src/net/listener.rs`, `server/Cargo.toml`
**驗收指令**: `cargo run` 顯示 "Listening on 0.0.0.0:8888"

**DoD**:
- [x] 建立 Rust 專案 (`cargo init`)
- [x] 加入 `socket2` dependency
- [x] `Socket::new(Domain::IPV4, Type::STREAM, Protocol::TCP)`
- [x] `set_reuse_address(true)`
- [x] `bind()` 到設定的 port
- [x] `listen(128)` 設定 backlog
- [x] CLI 印出 bind 成功訊息

---

### S1.2 Accept loop & connection registry `[P0]` `DONE` `@Claude`

**依賴**: S1.1
**檔案**: `server/src/net/listener.rs`, `server/src/net/connection.rs`
**驗收指令**: 使用 `nc localhost 8888` 連線，server 印出連線資訊

**DoD**:
- [x] `socket.accept()` 迴圈
- [x] 為每個連線分配 connection ID
- [x] 儲存 peer address
- [x] 印出 connect/disconnect log
- [x] 支援同時多個連線

---

### S1.3 NDJSON codec (server side) `[P0]` `DONE` `@Claude`

**依賴**: S1.2
**檔案**: `server/src/protocol/codec.rs`, `server/src/protocol/messages.rs`
**驗收指令**: 送 `{"type":"PING"}\n` 能收到回應

**DoD**:
- [x] 加入 `serde`, `serde_json` dependencies
- [x] 定義 `Message` enum (使用 serde tag)
- [x] `BufReader::read_line()` 讀取一行
- [x] `serde_json::from_str()` 解析
- [x] `serde_json::to_string()` + `\n` 發送
- [x] 處理 parse error (回傳 ERROR message)

---

### S1.4 Per-connection thread model `[P0]` `DONE` `@Claude`

**依賴**: S1.3
**檔案**: `server/src/net/handler.rs`
**驗收指令**: 4 個 client 同時連線，各自獨立處理

**DoD**:
- [x] `std::thread::spawn()` 為每個連線建立 thread
- [x] 建立 `mpsc::channel` 傳送事件到 game loop
- [x] 每個 handler 持有 `Sender<GameEvent>`
- [x] Clean shutdown path (處理 thread panic)
- [x] 測試：4 client 同時 echo

---

### S1.5 Main event loop with mpsc `[P1]` `DONE` `@Claude`

**依賴**: S1.4
**檔案**: `server/src/main.rs`
**驗收指令**: 事件能從 handler thread 傳到 main thread

**DoD**:
- [x] Main thread 持有 `Receiver<GameEvent>`
- [x] 事件 dispatch 架構
- [x] Broadcast 機制 (送訊息給所有 clients)
- [x] `HashMap<ConnectionId, ClientSender>` 管理連線

---

## EPIC 2 - Lobby & Matchmaking `@Claude`

### S2.1 HELLO/WELCOME handshake `[P0]` `DONE` `@Claude`

**依賴**: S1.3
**檔案**: `server/src/lobby/handshake.rs`
**驗收指令**: 送 HELLO，收到 WELCOME 含 player_id

**DoD**:
- [x] 解析 HELLO message (role, nickname, proto)
- [x] 驗證 nickname 長度 (1-16 chars)
- [x] 分配 player_id (P1-P4)
- [x] 處理 nickname 重複 (加後綴)
- [x] 回傳 WELCOME 或 ERROR

---

### S2.2 AI role authentication `[P1]` `DONE` `@Claude`

**依賴**: S2.1
**檔案**: `server/src/lobby/handshake.rs`
**驗收指令**: AI 無 auth token 被拒絕

**DoD**:
- [x] AI client 必須提供 `auth` 欄位
- [x] 驗證 auth token (環境變數 `AI_AUTH_TOKEN`)
- [x] 驗證失敗回傳 ERROR(AUTH_FAILED)
- [x] HUMAN client 不需要 token

---

### S2.3 Room creation & start rule `[P0]` `DONE` `@Claude`

**依賴**: S2.1
**檔案**: `server/src/lobby/room.rs`
**驗收指令**: 4 人加入後自動開始遊戲

**DoD**:
- [x] 等待條件：n humans + (4-n) AIs, n ∈ {1,2,3,4}
- [x] 新玩家加入時 broadcast ROOM_WAIT
- [x] 滿足條件時 broadcast ROOM_START
- [x] 分配隊伍 (HUMAN team vs AI team)
- [x] 紀錄 seed (用於重現發牌)

---

## EPIC 3 - Game Engine MVP (Trick Duel) `@Claude`

### S3.1 Deterministic card dealing `[P0]` `DONE` `@Claude`

**依賴**: S2.3
**檔案**: `server/src/game/deck.rs`
**驗收指令**: 相同 seed 產生相同手牌

**DoD**:
- [x] 52 張牌的 deck 表示
- [x] 使用 seed 的 shuffle 演算法 (Fisher-Yates with LCG)
- [x] 每人發 13 張
- [x] 送 DEAL message 給每位玩家
- [x] Unit test: 相同 seed → 相同結果

---

### S3.2 Turn rotation & YOUR_TURN `[P0]` `DONE` `@Claude`

**依賴**: S3.1
**檔案**: `server/src/game/engine.rs`
**驗收指令**: 輪流收到 YOUR_TURN

**DoD**:
- [x] 追蹤 current player (P1 → P2 → P3 → P4 → P1...)
- [x] 每個 trick 由上一 trick 的贏家先出
- [x] 計算合法出牌 (legal moves)
- [x] 送 YOUR_TURN 含 legal 欄位
- [x] 設定 timeout (30 秒)

---

### S3.3 PLAY validation `[P0]` `DONE` `@Claude`

**依賴**: S3.2
**檔案**: `server/src/game/engine.rs`, `server/src/main.rs`
**驗收指令**: 非法出牌收到 PLAY_REJECT

**DoD**:
- [x] 檢查：牌在手牌中
- [x] 檢查：輪到該玩家
- [x] 檢查：符合跟牌規則 (follow suit)
- [x] 驗證失敗送 PLAY_REJECT 含 reason
- [x] 驗證成功 broadcast PLAY_BROADCAST

---

### S3.4 Trick resolution & scoring `[P0]` `DONE` `@Claude`

**依賴**: S3.3
**檔案**: `server/src/game/engine.rs`
**驗收指令**: 4 人出完牌後收到 TRICK_RESULT

**DoD**:
- [x] 4 人都出牌後判定 trick winner
- [x] 比較規則：同花色最大者勝 (NoKing Rule)
- [x] 更新 team score
- [x] Broadcast TRICK_RESULT
- [x] 清除桌面，開始下一 trick

---

### S3.5 GAME_OVER & reset `[P1]` `DONE` `@Claude`

**依賴**: S3.4
**檔案**: `server/src/game/engine.rs`
**驗收指令**: 所有 tricks 結束後收到 GAME_OVER

**DoD**:
- [x] 所有 tricks 結束後計算 final score
- [x] 判定 winner team
- [x] Broadcast GAME_OVER 含 history
- [x] 支援重新開始 (回到 lobby 或自動開新局)

---

### S3.6 Verify NoKing / No Trump Rule `[P0]` `DONE` `@Claude`

**依賴**: S3.4
**檔案**: `server/src/game/engine.rs`
**驗收指令**: Code review 確認無王牌邏輯

**DoD**:
- [x] 確認 Trick winner 判定邏輯僅依賴 Lead Suit
- [x] 確認無任何 Trump Suit 設定
- [x] 確認 Deck 為標準 52 張 (A-K)

---

## EPIC 4 - UDP Heartbeat `@Claude` `@Gemini`

### S4.1 UDP server bind & ping/pong `[P1]` `DONE` `@Claude`

**依賴**: S1.1
**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: 用 `nc -u localhost 8889` 送 PING 收到 PONG

**DoD**:
- [x] `UdpSocket::bind()` 到 UDP port
- [x] 獨立 thread 處理 heartbeat
- [x] 解析 HB_PING (seq, t_client_ms)
- [x] 回覆 HB_PONG (加上 t_server_ms)
- [x] 紀錄每個 client 的最後 heartbeat 時間

---

### S4.2 Client heartbeat loop (Python) `[P1]` `TODO` `@Gemini`

> **Note**: 詳細 DoD 與進度請參閱 `clnt_stories.md`。

---

### S4.3 Stale client detection (optional) `[P2]` `DONE` `@Claude`

**依賴**: S4.1
**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: Client 停止 heartbeat 後 server log 警告

**DoD**:
- [x] 追蹤每個 client 的 last_heartbeat_time
- [x] 超過 threshold (10 秒) 標記為 stale
- [x] Log 警告訊息
- [x] (選) 通知 game engine

---

## EPIC 5 - Clients (Core) `@Gemini`

> **Note**: 本 Epic 所有 Story (S5.1 - S5.4) 詳細 DoD 與進度請參閱 `clnt_stories.md`。

---

## EPIC 6 - Demo & QA `@Shared`

### S6.1 One-command demo script `[P1]` `DONE` `@Shared`

**依賴**: S5.2, S5.3
**檔案**: `scripts/run_local_demo.sh`, `scripts/run_local_demo.ps1`, `scripts/run_auto_demo.sh`
**驗收指令**: 執行腳本能啟動完整遊戲

**DoD**:
- [x] 啟動 server (背景)
- [x] 啟動 n human + (4-n) AI clients
- [x] 等待遊戲結束
- [x] 清理 processes
- [x] 支援參數 (port, human count, seed)

**實作說明**:
- `run_local_demo.sh` - Linux/WSL 互動式 demo (支援 --port, --humans, --seed, --no-build)
- `run_local_demo.ps1` - Windows PowerShell 互動式 demo
- `run_auto_demo.sh` - 自動化 demo (無需人工互動，用於 CI/測試)

---

### S6.2 Logging standardization `[P1]` `DONE` `@Shared`

**依賴**: 全部
**檔案**: Server 使用 `env_logger`, Clients 使用 Python `logging`
**驗收指令**: Log 格式一致，可讀性高

**DoD**:
- [x] 統一 prefix: `[SERVER]`, `[ENGINE]`, `[HANDLER]`, `[HEARTBEAT]`, `[LOBBY]` 等
- [x] 時間戳記格式 (env_logger 自動提供)
- [x] Log level (DEBUG, INFO, WARN, ERROR)
- [x] 顏色輸出 (env_logger 支援)

**實作說明**:
- Server: 使用 `log` + `env_logger` crate，透過 `RUST_LOG` 環境變數控制
- Clients: 使用 Python `logging` 模組

---

### S6.3 Edge case handling `[P1]` `DONE` `@Shared`

**依賴**: 全部
**檔案**: 各模組
**驗收指令**: Server 在各種異常下不 crash

**DoD**:
- [x] Client disconnect mid-game → 通知其他玩家並清理連線
- [x] Invalid JSON → ERROR(PROTOCOL_ERROR)
- [x] Duplicate nickname → 自動加後綴 (e.g., `Alice_2`)
- [x] 連線 timeout → clean disconnect (handler 正常結束)
- [x] Server graceful shutdown → 由 demo script 的 trap 處理清理

**實作說明**:
- Server: `lobby/handshake.rs::ensure_unique_nickname()` 處理重複暱稱
- Server: `net/handler.rs` 處理 Invalid JSON 和 timeout
- Server: `lobby/room.rs::handle_disconnect()` 處理斷線
- Demo: 腳本使用 `trap cleanup EXIT` 確保 process 清理

---

## BONUS EPIC (Optional)

### B1 Web client gateway (WebSocket ↔ TCP) `[P3]` `TODO`

**依賴**: EPIC 5 完成
**DoD**:
- [ ] WebSocket server (可用 warp 或 axum)
- [ ] Protocol translation
- [ ] 瀏覽器 client

---

### B2 mio-based event loop `[P3]` `TODO`

**依賴**: EPIC 1 完成
**DoD**:
- [ ] 移除 thread-per-connection
- [ ] 使用 mio 的 Poll
- [ ] 效能比較

---

### B3 Replay log & deterministic re-run `[P3]` `TODO`

**依賴**: S3.5
**DoD**:
- [ ] 紀錄所有 moves 到 log file
- [ ] 用 log 重現遊戲
- [ ] Viewer 模式

---

---

## EPIC 7 - Python GUI Client (Tkinter) `@Gemini`

> **Note**: 本 Epic 所有 Story (S7.1 - S7.4) 詳細 DoD 與進度請參閱 `clnt_stories.md`。

---

## EPIC 8 - C++ Client (POSIX Socket) `@Gemini`

> **Note**: 本 Epic 所有 Story (S8.1 - S8.4) 詳細 DoD 與進度請參閱 `clnt_stories.md`。

### S8.1 C++ Scaffold & Makefile `[P1]` `DONE` `@Gemini`

**依賴**: S0.1
**檔案**: `clients/cpp_cli/Makefile`, `clients/cpp_cli/main.cpp`
**驗收指令**: `make` 編譯成功，執行 `./client` 印出 Hello

**DoD**:
- [x] 建立 Makefile
- [x] 設定 compiler flags (-std=c++17 -Wall -pthread)
- [x] Hello World main.cpp

### S8.2 TCP Connection & Threading `[P1]` `DONE` `@Gemini`

**依賴**: S8.1
**檔案**: `clients/cpp_cli/network.cpp`
**驗收指令**: 連線到 local server，接收 WELCOME

**DoD**:
- [x] `socket(AF_INET, SOCK_STREAM, 0)`
- [x] `connect()` 到 127.0.0.1:8888
- [x] 讀取執行緒 (Reader Thread)
- [x] 簡單的 send/recv 封裝

### S8.3 NDJSON Protocol & Game Loop `[P1]` `TODO` `@Gemini`

**依賴**: S8.2
**檔案**: `clients/cpp_cli/protocol.cpp`
**驗收指令**: 能完成一局遊戲 (CLI 介面)

**DoD**:
- [ ] 手刻或引用簡易 JSON parser
- [ ] 解析 HELLO/WELCOME
- [ ] 處理 YOUR_TURN 顯示
- [ ] 讀取 stdin 輸入並送出 PLAY 訊息

### S8.4 UDP Heartbeat (C++) `[P2]` `TODO` `@Gemini`

**依賴**: S8.2
**檔案**: `clients/cpp_cli/heartbeat.cpp`
**驗收指令**: Server 收到 C++ client 的 UDP ping

**DoD**:
- [ ] 建立 UDP socket
- [ ] 獨立 thread 發送 HB_PING
- [ ] 計算 RTT

---

## Progress Summary (PM QA)

> Client 端與 Server 端的開發細節請參考 `progress/clnt_stories.md` 與 `progress/srv_stories.md`。

| EPIC | QA Status | Notes |
|------|-----------|-------|
| EPIC 0 - Scaffold | DONE | 環境與文件基礎已建立 |
| EPIC 1 - TCP Core | DONE | Server 端完成，驗收通過 |
| EPIC 2 - Lobby | DONE | Server 端完成，驗收通過 |
| EPIC 3 - Game Engine | DONE | Server 端完成 (含 S3.6 NoKing Rule 驗證) |
| EPIC 4 - UDP Heartbeat | DONE | Server 端完成，Stale detection 實作 |
| EPIC 5 - Clients (Core) | DONE | Python CLI/AI clients 完成 |
| EPIC 6 - Demo & QA | DONE | Demo scripts 完成，邊界處理完成 |
| EPIC 7 - GUI Client | DONE | Tkinter GUI 完成 |
| EPIC 8 - C++ Client | IN_PROGRESS | S8.1-S8.2 完成，S8.3-S8.4 進行中 |
| BONUS | TODO | 依需求再排定 |
