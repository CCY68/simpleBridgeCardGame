# CardArena Protocol Specification v1

> 本文件定義 CardArena 的完整通訊協議規格。

---

## 1. Transport Layer

### 1.1 TCP (Game Control)
- **用途**: 遊戲控制與狀態同步
- **Port**: 預設 8888 (可設定)
- **Framing**: NDJSON (Newline-Delimited JSON)
  - 每個訊息是一行 JSON
  - 以 `\n` (0x0A) 作為 frame boundary
  - 編碼: UTF-8

### 1.2 UDP (Heartbeat)
- **用途**: 連線品質監控 (RTT/loss rate)
- **Port**: 預設 8889 (TCP port + 1)
- **非關鍵**: 不影響遊戲邏輯

---

## 2. Message Format

所有訊息皆為 JSON 物件，必須包含 `type` 欄位。

```json
{"type": "MESSAGE_TYPE", ...fields}
```

---

## 3. Message Types - Connection Phase

### 3.1 HELLO (Client → Server)

Client 連線後發送的第一個訊息。

```json
{
  "type": "HELLO",
  "role": "HUMAN" | "AI",
  "nickname": "string (1-16 chars)",
  "proto": 1,
  "auth": "string (optional, AI only)"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| type | string | Yes | 固定 "HELLO" |
| role | string | Yes | "HUMAN" 或 "AI" |
| nickname | string | Yes | 玩家暱稱，1-16 字元 |
| proto | number | Yes | 協議版本，目前為 1 |
| auth | string | AI only | AI client 驗證 token |

### 3.2 WELCOME (Server → Client)

Server 接受連線後回覆。

```json
{
  "type": "WELCOME",
  "player_id": "P1",
  "nickname": "Player1",
  "room": "R001"
}
```

| Field | Type | Description |
|-------|------|-------------|
| player_id | string | 分配的玩家 ID (P1-P4) |
| nickname | string | 確認的暱稱 (可能被加後綴避免重複) |
| room | string | 房間 ID |

### 3.3 ERROR (Server → Client)

任何錯誤情況。

```json
{
  "type": "ERROR",
  "code": "INVALID_HELLO",
  "message": "Missing nickname field"
}
```

| Error Code | Description |
|------------|-------------|
| INVALID_HELLO | HELLO 訊息格式錯誤 |
| AUTH_FAILED | AI 驗證失敗 |
| ROOM_FULL | 房間已滿 |
| INVALID_MOVE | 非法出牌 |
| NOT_YOUR_TURN | 不是你的回合 |
| PROTOCOL_ERROR | 協議錯誤 |

---

## 4. Message Types - Lobby Phase

### 4.1 ROOM_WAIT (Server → Client)

等待其他玩家加入。

```json
{
  "type": "ROOM_WAIT",
  "room": "R001",
  "players": [
    {"id": "P1", "nickname": "Alice", "role": "HUMAN"},
    {"id": "P2", "nickname": "Bot1", "role": "AI"}
  ],
  "need": 2
}
```

| Field | Type | Description |
|-------|------|-------------|
| room | string | 房間 ID |
| players | array | 目前房間內的玩家 |
| need | number | 還需要幾位玩家 |

### 4.2 ROOM_START (Server → Client)

4 人到齊，遊戲開始。

```json
{
  "type": "ROOM_START",
  "room": "R001",
  "players": [
    {"id": "P1", "nickname": "Alice", "role": "HUMAN", "team": "HUMAN"},
    {"id": "P2", "nickname": "Bob", "role": "HUMAN", "team": "HUMAN"},
    {"id": "P3", "nickname": "Bot1", "role": "AI", "team": "AI"},
    {"id": "P4", "nickname": "Bot2", "role": "AI", "team": "AI"}
  ],
  "seed": 12345
}
```

---

## 5. Message Types - Game Phase

### 5.1 DEAL (Server → Client)

發牌給玩家。

```json
{
  "type": "DEAL",
  "hand": ["AS", "KH", "QD", "JC", "10S", "9H", "8D", "7C", "6S", "5H"],
  "total_tricks": 10
}
```

| Field | Type | Description |
|-------|------|-------------|
| hand | array | 手牌，使用標準撲克牌表示法 |
| total_tricks | number | 本局總 trick 數 |

**牌的表示法**:
- Rank: A, K, Q, J, 10, 9, 8, 7, 6, 5, 4, 3, 2
- Suit: S (Spades), H (Hearts), D (Diamonds), C (Clubs)
- 範例: "AS" = Ace of Spades, "10H" = 10 of Hearts

### 5.2 YOUR_TURN (Server → Client)

輪到你出牌。

```json
{
  "type": "YOUR_TURN",
  "trick": 1,
  "table": [],
  "legal": ["AS", "KH", "QD"],
  "timeout_ms": 30000
}
```

| Field | Type | Description |
|-------|------|-------------|
| trick | number | 第幾個 trick (1-based) |
| table | array | 目前桌面上的牌 `[{player_id, card}]` |
| legal | array | 合法可出的牌 |
| timeout_ms | number | 出牌時限 (毫秒) |

### 5.3 PLAY (Client → Server)

玩家出牌。

```json
{
  "type": "PLAY",
  "card": "AS"
}
```

### 5.4 PLAY_BROADCAST (Server → All Clients)

廣播某玩家的出牌。

```json
{
  "type": "PLAY_BROADCAST",
  "player_id": "P1",
  "card": "AS",
  "trick": 1
}
```

### 5.5 PLAY_REJECT (Server → Client)

出牌被拒絕。

```json
{
  "type": "PLAY_REJECT",
  "card": "AS",
  "reason": "NOT_IN_HAND"
}
```

| Reason | Description |
|--------|-------------|
| NOT_IN_HAND | 牌不在手牌中 |
| NOT_LEGAL | 不符合出牌規則 |
| NOT_YOUR_TURN | 不是你的回合 |

### 5.6 TRICK_RESULT (Server → All Clients)

一個 trick 結束。

```json
{
  "type": "TRICK_RESULT",
  "trick": 1,
  "plays": [
    {"player_id": "P1", "card": "AS"},
    {"player_id": "P2", "card": "KS"},
    {"player_id": "P3", "card": "QS"},
    {"player_id": "P4", "card": "JS"}
  ],
  "winner": "P1",
  "score": {
    "HUMAN": 1,
    "AI": 0
  }
}
```

### 5.7 GAME_OVER (Server → All Clients)

遊戲結束。

```json
{
  "type": "GAME_OVER",
  "final_score": {
    "HUMAN": 6,
    "AI": 4
  },
  "winner": "HUMAN",
  "history": [
    {"trick": 1, "winner": "P1", "cards": ["AS", "KS", "QS", "JS"]},
    ...
  ]
}
```

---

## 6. Message Types - UDP Heartbeat

### 6.1 HB_PING (Client → Server)

```json
{
  "type": "HB_PING",
  "seq": 42,
  "t_client_ms": 1704067200000
}
```

### 6.2 HB_PONG (Server → Client)

```json
{
  "type": "HB_PONG",
  "seq": 42,
  "t_client_ms": 1704067200000,
  "t_server_ms": 1704067200005
}
```

---

## 7. Game Rules (Trick Duel)

### 7.1 基本規則
- 4 人遊戲，分兩隊：HUMAN team vs AI team
- 每人發 10 張牌，共 10 個 tricks
- 每個 trick，4 人依序出一張牌

### 7.2 出牌規則
- 第一位玩家可出任意牌
- 後續玩家必須 **跟花色** (follow suit)
- 若無該花色，可出任意牌

### 7.3 勝負判定
- 每個 trick，出最大同花色牌者獲勝
- Rank 順序: A > K > Q > J > 10 > 9 > ... > 2
- 贏得 trick 較多的隊伍獲勝

---

## 8. State Machine

```
[DISCONNECTED]
      |
      | connect()
      v
[CONNECTED] --HELLO--> [WAITING_WELCOME]
      |                       |
      | ERROR                 | WELCOME
      v                       v
[DISCONNECTED]          [IN_LOBBY]
                              |
                              | ROOM_WAIT / ROOM_START
                              v
                        [IN_GAME]
                              |
                              | YOUR_TURN / PLAY / TRICK_RESULT
                              |
                              | GAME_OVER
                              v
                        [GAME_ENDED] --> [IN_LOBBY] (可重新開始)
```

---

## 9. Error Handling

### 9.1 Client 錯誤處理
- 收到 ERROR: 顯示錯誤訊息，根據 code 決定是否重連
- 連線斷開: 嘗試重連 (最多 3 次，間隔 2 秒)

### 9.2 Server 錯誤處理
- 收到無法解析的 JSON: 回覆 ERROR(PROTOCOL_ERROR)
- Client 超時未出牌: 回覆 ERROR(TIMEOUT)，自動出最小合法牌
- Client 斷線: 由 AI fallback 接管該玩家

---

## 10. Example Session

```
# Client connects
C→S: {"type":"HELLO","role":"HUMAN","nickname":"Alice","proto":1}
S→C: {"type":"WELCOME","player_id":"P1","nickname":"Alice","room":"R001"}
S→C: {"type":"ROOM_WAIT","room":"R001","players":[{"id":"P1","nickname":"Alice","role":"HUMAN"}],"need":3}

# More players join...
S→C: {"type":"ROOM_START","room":"R001","players":[...],"seed":12345}
S→C: {"type":"DEAL","hand":["AS","KH","QD","JC","10S"],"total_tricks":10}
S→C: {"type":"YOUR_TURN","trick":1,"table":[],"legal":["AS","KH","QD","JC","10S"],"timeout_ms":30000}

# Player plays a card
C→S: {"type":"PLAY","card":"AS"}
S→*: {"type":"PLAY_BROADCAST","player_id":"P1","card":"AS","trick":1}

# After all 4 players play
S→*: {"type":"TRICK_RESULT","trick":1,"plays":[...],"winner":"P1","score":{"HUMAN":1,"AI":0}}

# Game ends
S→*: {"type":"GAME_OVER","final_score":{"HUMAN":6,"AI":4},"winner":"HUMAN","history":[...]}
```
