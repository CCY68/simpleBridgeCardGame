# Integration Test Specification

> 定義 Server 與 Client 整合測試的場景，確保雙方實作一致。
>
> **用途**: Claude (Server) 和 Gemini (Client) 並行開發時，以此作為驗收標準。

---

## Test Categories

| Category | Purpose |
|----------|---------|
| T1.x | Connection & Handshake |
| T2.x | Lobby & Matchmaking |
| T3.x | Game Flow |
| T4.x | Error Handling |
| T5.x | UDP Heartbeat |

---

## T1 - Connection & Handshake

### T1.1 Basic HELLO/WELCOME

**Setup**: Server listening on port 8888

**Steps**:
1. Client connects via TCP
2. Client sends: `{"type":"HELLO","role":"HUMAN","nickname":"Alice","proto":1}`
3. Server replies: `{"type":"WELCOME","player_id":"P1","nickname":"Alice","room":"R001"}`

**Expected**:
- Server assigns player_id in format "P1"-"P4"
- Server assigns room in format "R001" or similar
- nickname in WELCOME matches HELLO (unless duplicate)

---

### T1.2 HELLO with Missing Fields

**Steps**:
1. Client sends: `{"type":"HELLO","role":"HUMAN"}`  (missing nickname, proto)

**Expected**:
- Server replies: `{"type":"ERROR","code":"INVALID_HELLO","message":"..."}`
- Connection remains open

---

### T1.3 AI HELLO without Auth

**Steps**:
1. Client sends: `{"type":"HELLO","role":"AI","nickname":"Bot1","proto":1}`  (missing auth)

**Expected**:
- Server replies: `{"type":"ERROR","code":"AUTH_FAILED","message":"..."}`

---

### T1.4 AI HELLO with Valid Auth

**Steps**:
1. Client sends: `{"type":"HELLO","role":"AI","nickname":"Bot1","proto":1,"auth":"valid_token"}`

**Expected**:
- Server replies: `{"type":"WELCOME",...}`

---

### T1.5 Duplicate Nickname

**Steps**:
1. Client A sends HELLO with nickname "Alice" → receives WELCOME
2. Client B sends HELLO with nickname "Alice"

**Expected**:
- Client B receives WELCOME with modified nickname (e.g., "Alice_1" or "Alice2")

---

## T2 - Lobby & Matchmaking

### T2.1 Room Wait Status

**Setup**: Server with empty room

**Steps**:
1. Client A joins → receives WELCOME + ROOM_WAIT
2. Client B joins → both receive ROOM_WAIT update

**Expected ROOM_WAIT for Client A after B joins**:
```json
{
  "type": "ROOM_WAIT",
  "room": "R001",
  "players": [
    {"id": "P1", "nickname": "Alice", "role": "HUMAN"},
    {"id": "P2", "nickname": "Bob", "role": "HUMAN"}
  ],
  "need": 2
}
```

---

### T2.2 Room Start (4 Players)

**Steps**:
1. 2 HUMAN + 2 AI clients join

**Expected**: All clients receive ROOM_START:
```json
{
  "type": "ROOM_START",
  "room": "R001",
  "players": [
    {"id": "P1", "nickname": "...", "role": "HUMAN", "team": "HUMAN"},
    {"id": "P2", "nickname": "...", "role": "HUMAN", "team": "HUMAN"},
    {"id": "P3", "nickname": "...", "role": "AI", "team": "AI"},
    {"id": "P4", "nickname": "...", "role": "AI", "team": "AI"}
  ],
  "seed": <number>
}
```

---

### T2.3 Team Assignment Rules

**Rule**: HUMAN players → HUMAN team, AI players → AI team

**Test Cases**:
| Humans | AIs | Expected Teams |
|--------|-----|----------------|
| 4 | 0 | All HUMAN team (no AI team) |
| 3 | 1 | 3 HUMAN, 1 AI |
| 2 | 2 | 2 HUMAN, 2 AI |
| 1 | 3 | 1 HUMAN, 3 AI |

---

## T3 - Game Flow

### T3.1 Deal Cards

**Trigger**: After ROOM_START

**Expected**: Each player receives DEAL with:
- `hand`: Array of 10 cards (e.g., ["AS", "KH", ...])
- `total_tricks`: 10
- No duplicate cards across all 4 players
- Same seed → same deal (deterministic)

---

### T3.2 Turn Rotation

**Steps**:
1. After DEAL, P1 receives YOUR_TURN (first player)
2. P1 plays a card
3. P2 receives YOUR_TURN
4. Continue until all 4 played

**Expected YOUR_TURN for P2**:
```json
{
  "type": "YOUR_TURN",
  "trick": 1,
  "table": [{"player_id": "P1", "card": "AS"}],
  "legal": ["KS", "QS", ...],  // Only spades if P2 has spades
  "timeout_ms": 30000
}
```

---

### T3.3 Follow Suit Rule

**Scenario**: P1 plays "AS" (Spades)

**Expected for P2**:
- If P2 has Spades → `legal` contains only Spades
- If P2 has no Spades → `legal` contains all hand cards

---

### T3.4 Valid Play

**Steps**:
1. P1 receives YOUR_TURN with legal=["AS", "KH"]
2. P1 sends: `{"type":"PLAY","card":"AS"}`

**Expected**:
- All clients receive: `{"type":"PLAY_BROADCAST","player_id":"P1","card":"AS","trick":1}`
- P2 receives YOUR_TURN

---

### T3.5 Invalid Play - Not in Hand

**Steps**:
1. P1 sends: `{"type":"PLAY","card":"2C"}` (not in hand)

**Expected**:
- P1 receives: `{"type":"PLAY_REJECT","card":"2C","reason":"NOT_IN_HAND"}`
- P1 receives another YOUR_TURN (retry)

---

### T3.6 Invalid Play - Not Legal

**Steps**:
1. P2 must follow spades but sends: `{"type":"PLAY","card":"KH"}` (hearts)

**Expected**:
- P2 receives: `{"type":"PLAY_REJECT","card":"KH","reason":"NOT_LEGAL"}`

---

### T3.7 Trick Resolution

**Scenario**: All 4 players played spades: AS, KS, QS, JS

**Expected TRICK_RESULT**:
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
  "score": {"HUMAN": 1, "AI": 0}
}
```

**Rule**: Highest card of lead suit wins (A > K > Q > J > 10 > ... > 2)

---

### T3.8 Trick Winner Leads Next

**After T3.7**: P1 won trick 1

**Expected**: P1 receives YOUR_TURN for trick 2 (winner leads)

---

### T3.9 Game Over

**Trigger**: After all 10 tricks completed

**Expected**:
```json
{
  "type": "GAME_OVER",
  "final_score": {"HUMAN": 6, "AI": 4},
  "winner": "HUMAN",
  "history": [...]
}
```

---

## T4 - Error Handling

### T4.1 Invalid JSON

**Steps**:
1. Client sends: `{invalid json`

**Expected**:
- Server replies: `{"type":"ERROR","code":"PROTOCOL_ERROR","message":"..."}`
- Connection remains open

---

### T4.2 Unknown Message Type

**Steps**:
1. Client sends: `{"type":"UNKNOWN_TYPE"}`

**Expected**:
- Server replies: `{"type":"ERROR","code":"PROTOCOL_ERROR","message":"Unknown message type"}`

---

### T4.3 Play Timeout

**Setup**: timeout_ms = 5000

**Steps**:
1. P1 receives YOUR_TURN
2. P1 does not respond for 5+ seconds

**Expected**:
- Server auto-plays smallest legal card for P1
- All clients receive PLAY_BROADCAST with P1's auto-played card
- Game continues

---

### T4.4 Client Disconnect Mid-Game

**Steps**:
1. Game in progress, P2 disconnects

**Expected Options** (implementation choice):
- Option A: AI takes over P2's moves
- Option B: Game pauses, other players notified
- Option C: Game ends with ERROR

**Recommended**: Option A (AI takeover)

---

## T5 - UDP Heartbeat

### T5.1 Basic Ping/Pong

**Steps**:
1. Client sends UDP: `{"type":"HB_PING","seq":1,"t_client_ms":1700000000000}`
2. Server replies UDP: `{"type":"HB_PONG","seq":1,"t_client_ms":1700000000000,"t_server_ms":1700000000005}`

**Expected**:
- `seq` matches
- `t_client_ms` echoed back
- `t_server_ms` is server's current time

---

### T5.2 RTT Calculation

**Client-side calculation**:
```
RTT = current_time - t_client_ms
```

---

### T5.3 Loss Rate Calculation

**Client-side calculation**:
```
If received seq=5 but last was seq=2
  lost = 5 - 2 - 1 = 2 packets lost
  loss_rate = lost / total_sent
```

---

## Test Execution Checklist

### For Server Developer (Claude)

Before merging, verify:
- [ ] T1.1 - T1.5 pass
- [ ] T2.1 - T2.3 pass
- [ ] T3.1 - T3.9 pass
- [ ] T4.1 - T4.4 pass
- [ ] T5.1 pass

### For Client Developer (Gemini)

Before merging, verify:
- [ ] Can complete T1.1 handshake
- [ ] Correctly handles ROOM_WAIT updates
- [ ] Correctly parses YOUR_TURN and sends PLAY
- [ ] Handles PLAY_REJECT and retries
- [ ] Displays TRICK_RESULT and GAME_OVER
- [ ] UDP heartbeat calculates RTT correctly

---

## Mock Server/Client for Testing

### Simple Mock Server (Python)

```python
# For Client development testing
import socket
import json

def mock_server(port=8888):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(('0.0.0.0', port))
    sock.listen(1)
    print(f"Mock server on port {port}")

    conn, addr = sock.accept()
    print(f"Connected: {addr}")

    # Read HELLO
    data = conn.recv(4096).decode()
    print(f"Received: {data}")

    # Send WELCOME
    welcome = {"type": "WELCOME", "player_id": "P1", "nickname": "Test", "room": "R001"}
    conn.send((json.dumps(welcome) + '\n').encode())

    # Continue with test scenario...
```

### Simple Mock Client (Python)

```python
# For Server development testing
import socket
import json

def mock_client(host='127.0.0.1', port=8888):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect((host, port))

    # Send HELLO
    hello = {"type": "HELLO", "role": "HUMAN", "nickname": "Test", "proto": 1}
    sock.send((json.dumps(hello) + '\n').encode())

    # Read WELCOME
    data = sock.recv(4096).decode()
    print(f"Received: {data}")

    # Continue with test scenario...
```
