# CardArena Server Development Stories

> Server (Rust) 開發進度追蹤 - 由 Claude Code 維護
>
> 最後更新: 2026-01-14

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
| EPIC 3 - Game Engine | 5 | 0 | 0 | 5 |
| EPIC 4 - UDP Heartbeat (Server) | 2 | 0 | 0 | 2 |
| **Total** | **15** | **8** | **0** | **7** |

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

## EPIC 3 - Game Engine MVP (Trick Duel) `TODO`

### S3.1 Deterministic card dealing `[P0]` `TODO`

**檔案**: `server/src/game/deck.rs`
**驗收指令**: 相同 seed 產生相同手牌

**DoD**:
- [ ] 52 張牌的 deck 表示
- [ ] 使用 seed 的 shuffle 演算法 (Fisher-Yates)
- [ ] 每人發 13 張
- [ ] 送 DEAL message 給每位玩家
- [ ] Unit test: 相同 seed → 相同結果

---

### S3.2 Turn rotation & YOUR_TURN `[P0]` `TODO`

**檔案**: `server/src/game/turn.rs`
**驗收指令**: 輪流收到 YOUR_TURN

**DoD**:
- [ ] 追蹤 current player (P1 → P2 → P3 → P4 → P1...)
- [ ] 每個 trick 由上一 trick 的贏家先出
- [ ] 計算合法出牌 (legal moves)
- [ ] 送 YOUR_TURN 含 legal 欄位
- [ ] 設定 timeout

---

### S3.3 PLAY validation `[P0]` `TODO`

**檔案**: `server/src/game/validation.rs`
**驗收指令**: 非法出牌收到 PLAY_REJECT

**DoD**:
- [ ] 檢查：牌在手牌中
- [ ] 檢查：輪到該玩家
- [ ] 檢查：符合跟牌規則 (follow suit)
- [ ] 驗證失敗送 PLAY_REJECT 含 reason
- [ ] 驗證成功 broadcast PLAY_BROADCAST

---

### S3.4 Trick resolution & scoring `[P0]` `TODO`

**檔案**: `server/src/game/trick.rs`, `server/src/game/scoring.rs`
**驗收指令**: 4 人出完牌後收到 TRICK_RESULT

**DoD**:
- [ ] 4 人都出牌後判定 trick winner
- [ ] 比較規則：同花色最大者勝
- [ ] 更新 team score
- [ ] Broadcast TRICK_RESULT
- [ ] 清除桌面，開始下一 trick

---

### S3.5 GAME_OVER & reset `[P1]` `TODO`

**檔案**: `server/src/game/engine.rs`
**驗收指令**: 所有 tricks 結束後收到 GAME_OVER

**DoD**:
- [ ] 所有 tricks 結束後計算 final score
- [ ] 判定 winner team
- [ ] Broadcast GAME_OVER 含 history
- [ ] 支援重新開始 (回到 lobby 或自動開新局)

---

## EPIC 4 - UDP Heartbeat (Server Side) `TODO`

### S4.1 UDP server bind & ping/pong `[P1]` `TODO`

**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: 用 `nc -u localhost 8889` 送 PING 收到 PONG

**DoD**:
- [ ] `UdpSocket::bind()` 到 UDP port
- [ ] 獨立 thread 處理 heartbeat
- [ ] 解析 HB_PING (seq, t_client_ms)
- [ ] 回覆 HB_PONG (加上 t_server_ms)
- [ ] 紀錄每個 client 的最後 heartbeat 時間

---

### S4.3 Stale client detection (optional) `[P2]` `TODO`

**檔案**: `server/src/net/heartbeat.rs`
**驗收指令**: Client 停止 heartbeat 後 server log 警告

**DoD**:
- [ ] 追蹤每個 client 的 last_heartbeat_time
- [ ] 超過 threshold (如 10 秒) 標記為 stale
- [ ] Log 警告訊息
- [ ] (選) 通知 game engine

---

## File Structure

```
server/src/
├── main.rs              # Accept loop + Game loop
├── net/
│   ├── mod.rs
│   ├── listener.rs      # socket2 TCP listener
│   ├── connection.rs    # Connection ID 管理
│   ├── handler.rs       # Per-connection thread
│   ├── event.rs         # mpsc GameEvent 定義
│   └── heartbeat.rs     # (TODO) UDP heartbeat
├── protocol/
│   ├── mod.rs
│   ├── messages.rs      # 完整 message types
│   └── codec.rs         # NDJSON 編解碼
├── lobby/
│   ├── mod.rs
│   ├── handshake.rs     # HELLO/WELCOME + AI auth
│   └── room.rs          # RoomManager, 4人開始
└── game/
    ├── mod.rs           # (TODO)
    ├── deck.rs          # (TODO) 發牌
    ├── turn.rs          # (TODO) 回合管理
    ├── validation.rs    # (TODO) 出牌驗證
    ├── trick.rs         # (TODO) Trick 判定
    ├── scoring.rs       # (TODO) 計分
    └── engine.rs        # (TODO) 遊戲引擎
```

---

## Test Statistics

| Category | Count | Status |
|----------|-------|--------|
| Unit Tests | 20 | ✅ All Passed |
| Integration | Manual | ✅ Verified |

---

## Changelog

### 2026-01-14
- 完成 EPIC 1 (S1.1 ~ S1.5) - TCP Networking Core
- 完成 EPIC 2 (S2.1 ~ S2.3) - Lobby & Matchmaking
- 建立 srv_stories.md 獨立追蹤 server 進度
