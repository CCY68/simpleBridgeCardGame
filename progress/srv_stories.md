# CardArena Server Development Stories

> Server (Rust) 開發進度追蹤 - 由 Claude Code 維護
>
> 最後更新: 2026-01-16
>
> **開發狀態: EPIC 9 (Bridge Mode) 完成 ✅**

---

## Status Legend

| Status | Meaning |
|--------|---------|
| `TODO` | 尚未開始 |
| `IN_PROGRESS` | 進行中 |
| `DONE` | 已完成 |
| `BLOCKED` | 被阻擋 |

---

## Progress Summary

| EPIC | Total | Done | In Progress | TODO |
|------|-------|------|-------------|------|
| EPIC 1 - TCP Core | 5 | 5 | 0 | 0 |
| EPIC 2 - Lobby | 3 | 3 | 0 | 0 |
| EPIC 3 - Game Engine | 6 | 6 | 0 | 0 |
| EPIC 4 - UDP Heartbeat (Server) | 2 | 2 | 0 | 0 |
| EPIC 9 - Bridge Mode (Server AI) | 5 | 5 | 0 | 0 |
| **Total** | **21** | **21** | **0** | **0** |

---

## EPIC 1 - TCP Networking Core `DONE`

### S1.1 TCP listener setup via socket2 `[P0]` `DONE`

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

### S1.2 Accept loop & connection registry `[P0]` `DONE`

**檔案**: `server/src/net/listener.rs`, `server/src/net/connection.rs`
**驗收指令**: 使用 `nc localhost 8888` 連線，server 印出連線資訊

**DoD**:
- [x] `socket.accept()` 迴圈
- [x] 為每個連線分配 connection ID
- [x] 儲存 peer address
- [x] 印出 connect/disconnect log
- [x] 支援同時多個連線

---

### S1.3 NDJSON codec (server side) `[P0]` `DONE`

**檔案**: `server/src/protocol/codec.rs`, `server/src/protocol/messages.rs`
**驗收指令**: 送 `{"type":"PING"}\n` 能收到 `{"type":"PONG"}`

**DoD**:
- [x] 加入 `serde`, `serde_json` dependencies
- [x] 定義 `Message` enum (使用 serde tag)
- [x] `BufReader::read_line()` 讀取一行
- [x] `serde_json::from_str()` 解析
- [x] `serde_json::to_string()` + `\n` 發送
- [x] 處理 parse error (回傳 ERROR message)

---

### S1.4 Per-connection thread model `[P0]` `DONE`

**檔案**: `server/src/net/handler.rs`
**驗收指令**: 4 個 client 同時連線，各自獨立處理

**DoD**:
- [x] `std::thread::spawn()` 為每個連線建立 thread
- [x] 建立 `mpsc::channel` 傳送事件到 game loop
- [x] 每個 handler 持有 `Sender<GameEvent>`
- [x] Clean shutdown path (處理 thread panic)
- [x] 測試：4 client 同時連線

---

### S1.5 Main event loop with mpsc `[P1]` `DONE`

**檔案**: `server/src/main.rs`, `server/src/net/event.rs`
**驗收指令**: 事件能從 handler thread 傳到 main thread

**DoD**:
- [x] Main thread 持有 `Receiver<GameEvent>`
- [x] 事件 dispatch 架構
- [x] Broadcast 機制 (送訊息給所有 clients)
- [x] `HashMap<ConnectionId, ClientSender>` 管理連線

---

## EPIC 2 - Lobby & Matchmaking `DONE`

### S2.1 HELLO/WELCOME handshake `[P0]` `DONE`

**檔案**: `server/src/lobby/handshake.rs`
**驗收指令**: 送 HELLO，收到 WELCOME 含 player_id

**DoD**:
- [x] 解析 HELLO message (role, nickname, proto)
- [x] 驗證 nickname 長度 (1-16 chars)
- [x] 分配 player_id (P1-P4)
- [x] 處理 nickname 重複 (加後綴)
- [x] 回傳 WELCOME 或 ERROR

---

### S2.2 AI role authentication `[P1]` `DONE`

**檔案**: `server/src/lobby/handshake.rs`
**驗收指令**: AI 無 auth token 被拒絕

**DoD**:
- [x] AI client 必須提供 `auth` 欄位
- [x] 驗證 auth token (環境變數 `AI_AUTH_TOKEN`)
- [x] 驗證失敗回傳 ERROR(AUTH_FAILED)
- [x] HUMAN client 不需要 token

---

### S2.3 Room creation & start rule `[P0]` `DONE`

**檔案**: `server/src/lobby/room.rs`
**驗收指令**: 4 人加入後自動開始遊戲

**DoD**:
- [x] 等待條件：n humans + (4-n) AIs, n ∈ {1,2,3,4}
- [x] 新玩家加入時 broadcast ROOM_WAIT
- [x] 滿足條件時 broadcast ROOM_START
- [x] 分配隊伍 (HUMAN team vs AI team)
- [x] 紀錄 seed (用於重現發牌)

---

## EPIC 3 - Game Engine MVP (Trick Duel) `DONE`

### S3.1 Deterministic card dealing `[P0]` `DONE`

**檔案**: `server/src/game/deck.rs`
**驗收指令**: 相同 seed 產生相同手牌

**DoD**:
- [x] 52 張牌的 deck 表示
- [x] 使用 seed 的 shuffle 演算法 (Fisher-Yates with LCG)
- [x] 每人發 13 張
- [x] 送 DEAL message 給每位玩家
- [x] Unit test: 相同 seed → 相同結果

---

### S3.2 Turn rotation & YOUR_TURN `[P0]` `DONE`

**檔案**: `server/src/game/engine.rs`
**驗收指令**: 輪流收到 YOUR_TURN

**DoD**:
- [x] 追蹤 current player (P1 → P2 → P3 → P4 → P1...)
- [x] 每個 trick 由上一 trick 的贏家先出
- [x] 計算合法出牌 (legal moves)
- [x] 送 YOUR_TURN 含 legal 欄位
- [x] 設定 timeout (30 秒)

---

### S3.3 PLAY validation `[P0]` `DONE`

**檔案**: `server/src/game/engine.rs`, `server/src/main.rs`
**驗收指令**: 非法出牌收到 PLAY_REJECT

**DoD**:
- [x] 檢查：牌在手牌中
- [x] 檢查：輪到該玩家
- [x] 檢查：符合跟牌規則 (follow suit)
- [x] 驗證失敗送 PLAY_REJECT 含 reason
- [x] 驗證成功 broadcast PLAY_BROADCAST

---

### S3.4 Trick resolution & scoring `[P0]` `DONE`

**檔案**: `server/src/game/engine.rs`
**驗收指令**: 4 人出完牌後收到 TRICK_RESULT

**DoD**:
- [x] 4 人都出牌後判定 trick winner
- [x] 比較規則：同花色最大者勝
- [x] 更新 team score
- [x] Broadcast TRICK_RESULT
- [x] 清除桌面，開始下一 trick

---

### S3.5 GAME_OVER & reset `[P1]` `DONE`

**檔案**: `server/src/game/engine.rs`
**驗收指令**: 所有 tricks 結束後收到 GAME_OVER

**DoD**:
- [x] 所有 tricks (13) 結束後計算 final score
- [x] 判定 winner team
- [x] Broadcast GAME_OVER 含 history
- [x] 遊戲狀態管理 (GamePhase enum)

---

### S3.6 Verify NoKing / No Trump Rule `[P0]` `DONE`

**檔案**: `server/src/game/engine.rs`
**驗收指令**: Unit test 確認只有同花色能贏，且無王牌邏輯

**DoD**:
- [x] 確認 Trick winner 判定邏輯僅依賴 Lead Suit
- [x] 確認無任何 Trump Suit 設定
- [x] 確認 Deck 為標準 52 張 (A-K)
- [x] 更新 `PROJECT.md` 規則說明

---

## EPIC 4 - UDP Heartbeat (Server Side) `DONE`

### S4.1 UDP server bind & ping/pong `[P1]` `DONE`

**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: 用 `nc -u localhost 8889` 送 PING 收到 PONG

**DoD**:
- [x] `UdpSocket::bind()` 到 UDP port
- [x] 獨立 thread 處理 heartbeat
- [x] 解析 HB_PING (seq, t_client_ms)
- [x] 回覆 HB_PONG (加上 t_server_ms)
- [x] 紀錄每個 client 的最後 heartbeat 時間

---

### S4.3 Stale client detection (optional) `[P2]` `DONE`

**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: Client 停止 heartbeat 後 server log 警告

**DoD**:
- [x] 追蹤每個 client 的 last_heartbeat_time
- [x] 超過 threshold (如 10 秒) 標記為 stale
- [x] Log 警告訊息
- [x] (選) 通知 game engine

---

## EPIC 9 - Bridge Mode (Server-side AI) `DONE`

> **目標**: 將 AI 玩家內建於 Server 端，Server 啟動後自動建立 2 位 AI 夥伴，
> 等待 2 位人類玩家加入即開始遊戲。人類玩家中途斷線則重新開始遊戲（不重啟 Server）。

### S9.1 Built-in AI Player Module `[P0]` `DONE`

**檔案**: `server/src/ai/mod.rs`, `server/src/ai/player.rs`
**驗收指令**: Server 啟動時 log 顯示 "AI Partner 1/2 ready"

**DoD**:
- [x] 建立 `ai` 模組目錄結構
- [x] 定義 `AiPlayer` struct (player_id, nickname, team)
- [x] 實作 `AiPlayer::create_partners()` 建構函數
- [x] Room 建立時自動加入 2 個 AI 玩家 (P3, P4)
- [x] AI 玩家使用虛擬連線 ID (不佔用 TCP 連線)

---

### S9.2 Modified Room Start Rule (2 Humans) `[P0]` `DONE`

**檔案**: `server/src/lobby/room.rs`, `server/src/main.rs`
**驗收指令**: 2 位人類玩家加入後自動開始遊戲

**DoD**:
- [x] 修改 `Room::can_start()` 邏輯：2 Humans + 2 Built-in AI = 開始
- [x] Bridge Mode 房間建立時預先將 AI 玩家加入 (P3, P4 位置)
- [x] Human 加入時分配 P1, P2 位置 (insert before AI)
- [x] `ROOM_WAIT` 訊息顯示正確的等待人數
- [x] `ROOM_START` 正確顯示 4 位玩家資訊

---

### S9.3 AI Turn Handler (Server-side) `[P0]` `DONE`

**檔案**: `server/src/main.rs`
**驗收指令**: AI 輪到時自動出牌，無需外部輸入

**DoD**:
- [x] 在 Game Loop 中偵測輪到 AI 玩家 (via `Room::is_virtual_conn()`)
- [x] 呼叫 AI 決策模組 (`SmartStrategy`) 取得出牌
- [x] 自動執行 PLAY 動作 (不經過 TCP)
- [x] `process_ai_turns()` 函數處理連續 AI 回合
- [x] `broadcast_to_humans()` 只發送給真人玩家

---

### S9.4 AI Card Strategy (Pluggable) `[P1]` `DONE`

**檔案**: `server/src/ai/strategy.rs`
**驗收指令**: AI 使用指定策略出牌

**DoD**:
- [x] 定義 `AiStrategy` trait
- [x] 實作 `SmartStrategy` (智慧策略)
- [x] 策略透過 trait object 可擴充
- [x] Unit test 驗證策略邏輯 (6 tests)

**策略規則** (SmartStrategy):
```rust
// ========== 首家 (領牌) ==========
// 出「最長花色的最小牌」(試探策略)
//   1. 統計手牌中各花色數量
//   2. 選擇數量最多的花色 (若平手，依 S > H > D > C 優先)
//   3. 出該花色中點數最小的牌

// ========== 非首家 (跟牌) ==========
// 情況 A: 有同花色的牌 (必須跟牌)
//   1. 找出桌面同花色最大的牌 (highest)
//   2. 找「大於 highest 至少 3 點」的最小牌
//   3. 若有 → 出該牌 (嘗試贏取)
//   4. 若無 → 出同花色最小牌 (放棄本輪)
//
// 情況 B: 無同花色的牌 (墊牌)
//   - 出點數最小的牌 (任意花色)

// ========== 點數對照 ==========
// 2=2, 3=3, 4=4, 5=5, 6=6, 7=7, 8=8, 9=9, 10=10
// J=11, Q=12, K=13, A=14

// ========== 範例 ==========
// 桌面: 7H (highest=7)
// 手牌: 3H, 9H, QH, 5D
// 需要: 7+3=10 以上的最小牌
// 結果: 出 QH (9H 只比 7H 大 2 點，不符合 +3 條件)
```

---

### S9.5 Human Disconnect & Game Restart `[P0]` `DONE`

**檔案**: `server/src/main.rs`, `server/src/lobby/room.rs`
**驗收指令**: 人類玩家斷線後遊戲重啟，Server 不重啟

**DoD**:
- [x] 偵測人類玩家斷線 (TCP disconnect)
- [x] 若遊戲進行中，立即結束當前遊戲
- [x] 發送 ERROR 訊息通知其他玩家
- [x] 重置房間狀態為 `Waiting` (`Room::reset_for_bridge_mode()`)
- [x] AI 玩家保留，等待新的人類玩家加入
- [x] 記錄斷線原因到 log

---

## 新增檔案結構

```
server/src/
├── ai/                      # 新增: AI 模組
│   ├── mod.rs               # AI 模組入口
│   ├── player.rs            # AiPlayer 定義
│   ├── turn_handler.rs      # AI 出牌處理
│   └── strategy.rs          # 出牌策略 (trait + 實作)
├── main.rs                  # 修改: 整合 AI 模組
├── lobby/
│   └── room.rs              # 修改: 2 Human 開始規則
└── protocol/
    └── messages.rs          # 修改: 新增 GAME_ABORT 訊息
```

---

## File Structure

```
server/src/
├── main.rs              # Accept loop + Game loop + UDP Heartbeat 啟動
├── ai/                  # [EPIC 9] 內建 AI 模組
│   ├── mod.rs           # AI 模組入口
│   ├── player.rs        # AiPlayer 定義
│   ├── turn_handler.rs  # AI 出牌處理
│   └── strategy.rs      # 出牌策略 (trait + 實作)
├── net/
│   ├── mod.rs
│   ├── listener.rs      # socket2 TCP listener
│   ├── connection.rs    # Connection ID 管理
│   ├── handler.rs       # Per-connection thread
│   ├── event.rs         # mpsc GameEvent 定義
│   └── heartbeat.rs     # UDP heartbeat server (HB_PING/HB_PONG)
├── protocol/
│   ├── mod.rs
│   ├── messages.rs      # 完整 message types + HeartbeatPing/Pong + GAME_ABORT
│   └── codec.rs         # NDJSON 編解碼
├── lobby/
│   ├── mod.rs
│   ├── handshake.rs     # HELLO/WELCOME + AI auth
│   └── room.rs          # RoomManager, 2 Human 開始 (Bridge Mode)
└── game/
    ├── mod.rs
    ├── deck.rs          # 52張牌, Fisher-Yates shuffle
    └── engine.rs        # 遊戲引擎: 發牌/出牌/計分/結束
```

---

## Test Statistics

| Category | Count | Status |
|----------|-------|--------|
| Unit Tests | 48 | ✅ All Passed |
| Integration | Manual | ✅ Verified |

---

## Verification Commands

```bash
# 編譯與測試
cd server && cargo build
cd server && cargo test

# 啟動 Server
cd server && cargo run

# 測試 TCP PING (另開終端)
echo '{"type":"PING"}' | nc -w1 localhost 8888
# 預期回應: {"type":"PONG"}

# 測試 UDP Heartbeat (另開終端)
echo '{"type":"HB_PING","seq":1,"t_client_ms":1704067200000}' | nc -u -w1 localhost 8889
# 預期回應: {"type":"HB_PONG","seq":1,"t_client_ms":1704067200000,"t_server_ms":...}

# 測試 HELLO handshake
echo '{"type":"HELLO","role":"HUMAN","nickname":"TestUser","proto":1}' | nc -w1 localhost 8888
# 預期回應: {"type":"WELCOME",...}
```

---

## Changelog

### 2026-01-16
- **完成 EPIC 9 (S9.1 ~ S9.5) - Bridge Mode (Server-side AI)**
  - ai/mod.rs, ai/player.rs: AiPlayer 定義
  - ai/strategy.rs: SmartStrategy 實作 (首家/跟牌策略)
  - lobby/room.rs: Bridge Mode 房間管理 (2 Human + 2 AI)
  - main.rs: process_ai_turns(), broadcast_to_humans()
  - main.rs: handle_bridge_mode_disconnect() 斷線重置
- 測試數量: 37 → 48

### 2026-01-14 (Final)
- **Server 端開發全部完成**
- 手動驗證測試通過:
  - TCP 監聽 (port 8888): ✅ PING/PONG 正常
  - UDP Heartbeat (port 8889): ✅ HB_PING/HB_PONG 正常
  - Connection 管理: ✅ 連線/斷線 log 正常
- 所有 15 個 Stories 已完成

### 2026-01-14 (Update 3)
- 完成 EPIC 4 (S4.1, S4.3) - UDP Heartbeat Server
  - heartbeat.rs: UDP socket bind, HB_PING/HB_PONG 處理
  - 獨立 thread 處理 heartbeat
  - Stale client detection (10 秒 threshold)
  - 整合到 main.rs，UDP port = TCP port + 1
- 測試數量: 31 → 37

### 2026-01-14 (Update 2)
- 完成 EPIC 3 (S3.1 ~ S3.5) - Game Engine MVP
  - deck.rs: 52張牌、Fisher-Yates shuffle、確定性發牌
  - engine.rs: 完整遊戲引擎 (發牌/回合/出牌驗證/Trick結算/計分/結束)
  - 整合到 main.rs 處理 PLAY 訊息
- 測試數量: 20 → 31

### 2026-01-14
- 完成 EPIC 1 (S1.1 ~ S1.5) - TCP Networking Core
- 完成 EPIC 2 (S2.1 ~ S2.3) - Lobby & Matchmaking
- 建立 srv_stories.md 獨立追蹤 server 進度
