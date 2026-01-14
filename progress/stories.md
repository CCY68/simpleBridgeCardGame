# CardArena Development Stories

> 每個 Story 對應一個可交付的功能單元。
> 使用狀態標記追蹤進度：`TODO` → `IN_PROGRESS` → `DONE`

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
- [ ] 建立 `server/` 目錄 (Rust)
- [ ] 建立 `clients/` 目錄 (Python)
- [ ] 建立 `protocol/` 目錄 (文件)
- [ ] 建立 `progress/` 目錄 (追蹤)
- [ ] 建立 `scripts/` 目錄 (腳本)
- [ ] 建立 `.gitignore` (Rust + Python + OS)

---

### S0.2 Create protocol specification `[P0]` `DONE` `@Shared`

**依賴**: S0.1
**檔案**: `protocol/protocol.md`
**驗收指令**: 檢查文件包含所有 message types

**DoD**:
- [ ] 定義 NDJSON framing 規則
- [ ] 定義所有 message types (HELLO, WELCOME, DEAL, YOUR_TURN, PLAY, etc.)
- [ ] 包含範例訊息
- [ ] 定義 error codes

---

### S0.3 Create POSIX mapping document `[P1]` `DONE` `@Shared`

**依賴**: S0.1
**檔案**: `protocol/posix_mapping.md`
**驗收指令**: 表格包含 TCP + UDP lifecycle

**DoD**:
- [ ] C/C++ vs Rust 對照表 (socket, bind, listen, accept, etc.)
- [ ] 包含程式碼範例
- [ ] 說明為何選擇 socket2

---

### S0.4 Create .gitignore `[P0]` `DONE` `@Shared`

**依賴**: S0.1
**檔案**: `.gitignore`

**DoD**:
- [ ] Rust: `target/`, `Cargo.lock` (for library), `*.pdb`
- [ ] Python: `__pycache__/`, `*.pyc`, `.venv/`, `*.egg-info/`
- [ ] OS: `.DS_Store`, `Thumbs.db`
- [ ] IDE: `.idea/`, `.vscode/`, `*.swp`

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

### S3.1 Deterministic card dealing `[P0]` `TODO` `@Claude`

**依賴**: S2.3
**檔案**: `server/src/game/deck.rs`
**驗收指令**: 相同 seed 產生相同手牌

**DoD**:
- [ ] 52 張牌的 deck 表示
- [ ] 使用 seed 的 shuffle 演算法 (Fisher-Yates)
- [ ] 每人發 10 張 (或 13 張視規則調整)
- [ ] 送 DEAL message 給每位玩家
- [ ] Unit test: 相同 seed → 相同結果

---

### S3.2 Turn rotation & YOUR_TURN `[P0]` `TODO` `@Claude`

**依賴**: S3.1
**檔案**: `server/src/game/turn.rs`
**驗收指令**: 輪流收到 YOUR_TURN

**DoD**:
- [ ] 追蹤 current player (P1 → P2 → P3 → P4 → P1...)
- [ ] 每個 trick 由上一 trick 的贏家先出
- [ ] 計算合法出牌 (legal moves)
- [ ] 送 YOUR_TURN 含 legal 欄位
- [ ] 設定 timeout

---

### S3.3 PLAY validation `[P0]` `TODO` `@Claude`

**依賴**: S3.2
**檔案**: `server/src/game/validation.rs`
**驗收指令**: 非法出牌收到 PLAY_REJECT

**DoD**:
- [ ] 檢查：牌在手牌中
- [ ] 檢查：輪到該玩家
- [ ] 檢查：符合跟牌規則 (follow suit)
- [ ] 驗證失敗送 PLAY_REJECT 含 reason
- [ ] 驗證成功 broadcast PLAY_BROADCAST

---

### S3.4 Trick resolution & scoring `[P0]` `TODO` `@Claude`

**依賴**: S3.3
**檔案**: `server/src/game/trick.rs`, `server/src/game/scoring.rs`
**驗收指令**: 4 人出完牌後收到 TRICK_RESULT

**DoD**:
- [ ] 4 人都出牌後判定 trick winner
- [ ] 比較規則：同花色最大者勝
- [ ] 更新 team score
- [ ] Broadcast TRICK_RESULT
- [ ] 清除桌面，開始下一 trick

---

### S3.5 GAME_OVER & reset `[P1]` `TODO` `@Claude`

**依賴**: S3.4
**檔案**: `server/src/game/engine.rs`
**驗收指令**: 所有 tricks 結束後收到 GAME_OVER

**DoD**:
- [ ] 所有 tricks 結束後計算 final score
- [ ] 判定 winner team
- [ ] Broadcast GAME_OVER 含 history
- [ ] 支援重新開始 (回到 lobby 或自動開新局)

---

## EPIC 4 - UDP Heartbeat `@Claude` `@Gemini`

### S4.1 UDP server bind & ping/pong `[P1]` `TODO` `@Claude`

**依賴**: S1.1
**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: 用 `nc -u localhost 8889` 送 PING 收到 PONG

**DoD**:
- [ ] `UdpSocket::bind()` 到 UDP port
- [ ] 獨立 thread 處理 heartbeat
- [ ] 解析 HB_PING (seq, t_client_ms)
- [ ] 回覆 HB_PONG (加上 t_server_ms)
- [ ] 紀錄每個 client 的最後 heartbeat 時間

---

### S4.2 Client heartbeat loop (Python) `[P1]` `TODO` `@Gemini`

> **Note**: 詳細 DoD 與進度請參閱 `clnt_stories.md`。

---

### S4.3 Stale client detection (optional) `[P2]` `TODO` `@Claude`

**依賴**: S4.1
**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: Client 停止 heartbeat 後 server log 警告

**DoD**:
- [ ] 追蹤每個 client 的 last_heartbeat_time
- [ ] 超過 threshold (如 10 秒) 標記為 stale
- [ ] Log 警告訊息
- [ ] (選) 通知 game engine

---

## EPIC 5 - Clients (Core) `@Gemini`

> **Note**: 本 Epic 所有 Story (S5.1 - S5.4) 詳細 DoD 與進度請參閱 `clnt_stories.md`。

---

## EPIC 6 - Demo & QA `@Shared`

### S6.1 One-command demo script `[P1]` `TODO` `@Shared`

**依賴**: S5.2, S5.3
**檔案**: `scripts/run_local_demo.sh`, `scripts/run_local_demo.ps1`
**驗收指令**: 執行腳本能啟動完整遊戲

**DoD**:
- [ ] 啟動 server (背景)
- [ ] 啟動 n human + (4-n) AI clients
- [ ] 等待遊戲結束
- [ ] 清理 processes
- [ ] 支援參數 (port, human count, seed)

---

### S6.2 Logging standardization `[P1]` `TODO` `@Shared`

**依賴**: 全部
**檔案**: `server/src/log.rs`, `clients/common/log.py`
**驗收指令**: Log 格式一致，可讀性高

**DoD**:
- [ ] 統一 prefix: `[SERVER]`, `[CLIENT]`, `[AI]`, `[HB]`
- [ ] 時間戳記格式
- [ ] Log level (DEBUG, INFO, WARN, ERROR)
- [ ] 顏色輸出 (可選)

---

### S6.3 Edge case handling `[P1]` `TODO` `@Shared`

**依賴**: 全部
**檔案**: 各模組
**驗收指令**: Server 在各種異常下不 crash

**DoD**:
- [ ] Client disconnect mid-game → AI 接管或遊戲暫停
- [ ] Invalid JSON → ERROR(PROTOCOL_ERROR)
- [ ] Duplicate nickname → 自動加後綴
- [ ] 連線 timeout → clean disconnect
- [ ] Server graceful shutdown (Ctrl+C)

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

## Progress Summary

> Client 端的詳細狀態請參考 `clnt_stories.md`。以下僅列出 Server 與 Shared 任務。

| EPIC | Owner | Total | Done | In Progress | TODO |
|------|-------|-------|------|-------------|------|
| EPIC 0 - Scaffold | @Shared | 4 | 4 | 0 | 0 |
| EPIC 1 - TCP Core | @Claude | 5 | 5 | 0 | 0 |
| EPIC 2 - Lobby | @Claude | 3 | 3 | 0 | 0 |
| EPIC 3 - Game Engine | @Claude | 5 | 0 | 0 | 5 |
| EPIC 4 - UDP Heartbeat | @Both | 3 | 0 | 0 | 3 |
| EPIC 5 - Clients (Core) | @Gemini | (See clnt_stories) | - | - | - |
| EPIC 6 - Demo & QA | @Shared | 3 | 0 | 0 | 3 |
| EPIC 7 - GUI Client | @Gemini | (See clnt_stories) | - | - | - |
| BONUS | @TBD | 3 | 0 | 0 | 3 |