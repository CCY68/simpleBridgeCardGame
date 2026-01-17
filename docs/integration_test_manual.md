# SimpleBridgeCardGame 整合測試說明書

本文件整理整合測試執行方式，依 `protocol/integration_tests.md` 為驗收標準。

---

## 1. 測試前準備

- 啟動 Server：

```bash
cd server && cargo run
```

- 確認 TCP port（預設 8888）與 UDP port（預設 8889）可用。

---

## 2. T1 - Connection & Handshake

### T1.1 Basic HELLO/WELCOME

```bash
echo '{"type":"HELLO","role":"HUMAN","nickname":"Alice","proto":1}' | nc -w1 127.0.0.1 8888
```

Expected:
- 回應 `WELCOME`，含 `player_id`, `room`

### T1.3 AI HELLO without Auth

```bash
echo '{"type":"HELLO","role":"AI","nickname":"Bot1","proto":1}' | nc -w1 127.0.0.1 8888
```

Expected:
- 回應 `ERROR`，code = `AUTH_FAILED`

---

## 3. T2 - Lobby & Matchmaking

### T2.1 Room Wait Status

- 連線 1~3 位玩家
- 檢查收到 `ROOM_WAIT`，`need` 正確

### T2.2 Room Start (4 Players)

- 連線滿 4 人（人類或 AI）
- 所有玩家收到 `ROOM_START`

---

## 4. T3 - Game Flow

### T3.1 Deal Cards

- 收到 `ROOM_START` 後
- 每位玩家收到 `DEAL`
- 手牌數量 13，且無重複

### T3.2 Turn Rotation

- 依序收到 `YOUR_TURN`
- 出牌後廣播 `PLAY_BROADCAST`

### T3.7 Trick Resolution

- 四人出完牌後收到 `TRICK_RESULT`

### T3.9 Game Over

- 13 trick 後收到 `GAME_OVER`

---

## 5. T4 - Error Handling

### T4.1 Invalid JSON

```bash
printf '{invalid json\n' | nc -w1 127.0.0.1 8888
```

Expected:
- 回應 `ERROR`，code = `PROTOCOL_ERROR`

---

## 6. T5 - UDP Heartbeat

### T5.1 Basic Ping/Pong

```bash
echo '{"type":"HB_PING","seq":1,"t_client_ms":1704067200000}' | nc -u -w1 127.0.0.1 8889
```

Expected:
- 回應 `HB_PONG`，`seq` 與 `t_client_ms` 相同

---

## 7. 測試記錄建議

執行完成後，建議在 `progress/notes.md` 補上：

- 測試日期
- 測試範圍
- 是否通過
- 異常紀錄與處置
