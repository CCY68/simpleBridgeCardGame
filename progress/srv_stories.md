# SimpleBridgeCardGame Server Development Stories

> Server (Rust) 開發進度追蹤 - 由 Claude Code 維護
>
> 最後更新: 2026-01-16
>
> **開發狀態: EPIC 10 (Remote Admin) 完成 ✅**

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
| EPIC 10 - Remote Admin Tools | 4 | 4 | 0 | 0 |
| **Total** | **25** | **25** | **0** | **0** |

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

## EPIC 10 - Remote Admin Tools `DONE`

> **目標**: 提供獨立的 TCP 管理介面，允許管理員遠端監控伺服器狀態、
> 查看遊戲訊息、重設遊戲、以及剔除玩家。

### S10.1 Admin Server Architecture `[P0]` `DONE`

**檔案**: `server/src/admin/mod.rs`, `server/src/admin/server.rs`
**驗收指令**: `nc localhost 8890` 連線後輸入 `HELP` 顯示可用指令

**DoD**:
- [x] 建立 `admin` 模組目錄結構
- [x] 在獨立 TCP port (8890) 監聯管理連線
- [x] 簡單文字協議 (每行一個指令)
- [x] 支援基本認證 (`AUTH <token>`)
- [x] 實作 `HELP` 指令列出所有可用指令
- [x] Admin thread 與 Game loop 透過 mpsc channel 通訊

**指令格式**:
```
AUTH <token>     # 認證 (必須先執行)
HELP             # 顯示可用指令
STATUS           # 顯示伺服器狀態
ROOMS            # 列出所有房間
PLAYERS          # 列出所有玩家
LOGS [n]         # 顯示最近 n 條訊息 (預設 20)
KICK <player_id> # 踢除玩家
RESET [room_id]  # 重設遊戲 (預設當前房間)
QUIT             # 斷開管理連線
```

---

### S10.2 Message Logging & Viewing `[P1]` `DONE`

**檔案**: `server/src/admin/logger.rs`
**驗收指令**: `LOGS 10` 顯示最近 10 條遊戲訊息

**DoD**:
- [x] 建立環形緩衝區 (Ring Buffer) 儲存最近 N 條訊息
- [x] 記錄關鍵事件 (玩家加入/離開、出牌、Trick 結果等)
- [x] 實作 `LOGS [n]` 指令
- [x] 訊息格式含時間戳記與事件類型
- [x] 支援按事件類型過濾

**訊息格式範例**:
```
[2026-01-16 12:34:56] PLAYER_JOIN: Alice (P1) joined R001
[2026-01-16 12:35:02] PLAYER_JOIN: Bob (P2) joined R001
[2026-01-16 12:35:02] GAME_START: R001 started (seed: 12345)
[2026-01-16 12:35:10] PLAY: P1 plays 5H (trick 1)
[2026-01-16 12:35:15] TRICK_RESULT: P3 wins trick 1
```

---

### S10.3 Game Reset Command `[P0]` `DONE`

**檔案**: `server/src/admin/commands.rs`, `server/src/main.rs`
**驗收指令**: `RESET R001` 重設指定房間的遊戲

**DoD**:
- [x] 實作 `RESET [room_id]` 指令
- [x] 若未指定 room_id，重設所有進行中的遊戲
- [x] 通知房間內所有玩家遊戲已重設
- [x] 房間回到 Waiting 狀態
- [x] 返回操作結果 (成功/失敗原因)

**回應格式**:
```
OK: Room R001 reset successfully
ERROR: Room R001 not found
ERROR: Room R001 is not in playing state
```

---

### S10.4 Kick Player Command `[P0]` `DONE`

**檔案**: `server/src/admin/commands.rs`, `server/src/main.rs`
**驗收指令**: `KICK P1` 踢除指定玩家

**DoD**:
- [x] 實作 `KICK <player_id>` 指令
- [x] 關閉該玩家的 TCP 連線
- [x] 觸發正常的斷線處理流程
- [x] 若遊戲進行中，依 Bridge Mode 規則處理
- [x] 返回操作結果

**回應格式**:
```
OK: Player P1 (Alice) kicked from R001
ERROR: Player P1 not found
ERROR: Cannot kick AI player
```

---

## 新增檔案結構

```
server/src/
├── ai/                      # [EPIC 9] AI 模組
│   ├── mod.rs               # AI 模組入口
│   ├── player.rs            # AiPlayer 定義
│   └── strategy.rs          # 出牌策略 (trait + 實作)
├── admin/                   # [EPIC 10] 遠端管理模組
│   ├── mod.rs               # Admin 模組入口
│   ├── server.rs            # Admin TCP server
│   ├── commands.rs          # 指令處理 (KICK, RESET 等)
│   └── logger.rs            # 訊息記錄與查看
├── main.rs                  # 整合所有模組
├── lobby/
│   └── room.rs              # Bridge Mode 房間管理
└── protocol/
    └── messages.rs          # 協議訊息類型
```

---

## File Structure

```
server/src/
├── main.rs              # Accept loop + Game loop + UDP Heartbeat + Admin 啟動
├── ai/                  # [EPIC 9] 內建 AI 模組
│   ├── mod.rs           # AI 模組入口
│   ├── player.rs        # AiPlayer 定義
│   └── strategy.rs      # 出牌策略 (trait + 實作)
├── admin/               # [EPIC 10] 遠端管理模組
│   ├── mod.rs           # Admin 模組入口
│   ├── server.rs        # Admin TCP server (port 8890)
│   ├── commands.rs      # 指令處理 (KICK, RESET, STATUS 等)
│   └── logger.rs        # 訊息記錄與查看 (Ring Buffer)
├── net/
│   ├── mod.rs
│   ├── listener.rs      # socket2 TCP listener
│   ├── connection.rs    # Connection ID 管理
│   ├── handler.rs       # Per-connection thread
│   ├── event.rs         # mpsc GameEvent 定義
│   └── heartbeat.rs     # UDP heartbeat server (HB_PING/HB_PONG)
├── protocol/
│   ├── mod.rs
│   ├── messages.rs      # 完整 message types
│   └── codec.rs         # NDJSON 編解碼
├── lobby/
│   ├── mod.rs
│   ├── handshake.rs     # HELLO/WELCOME + AI auth
│   └── room.rs          # RoomManager, Bridge Mode
└── game/
    ├── mod.rs
    ├── deck.rs          # 52張牌, Fisher-Yates shuffle
    └── engine.rs        # 遊戲引擎: 發牌/出牌/計分/結束
```

---

## Test Statistics

| Category | Count | Status |
|----------|-------|--------|
| Unit Tests | 58 | ✅ All Passed |
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

### 2026-01-16 (Update 2)
- **完成 EPIC 10 (S10.1 ~ S10.4) - Remote Admin Tools**
  - admin/mod.rs: Admin 模組入口
  - admin/server.rs: TCP 8890 管理伺服器
  - admin/commands.rs: 指令解析與格式化
  - admin/logger.rs: Ring Buffer 遊戲日誌
  - main.rs: 整合 admin 事件處理
  - lobby/room.rs: 新增 Admin 輔助方法
- 測試數量: 48 → 58

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
