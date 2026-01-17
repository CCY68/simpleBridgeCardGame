# PROJECT - SimpleBridgeCardGame Architecture & Plan

> 本文件描述 SimpleBridgeCardGame 的系統架構、模組拆解、協議設計、開發計畫與里程碑。
>
> **Related Documents**:
> - [protocol/protocol.md](protocol/protocol.md) - 完整協議規格
> - [protocol/posix_mapping.md](protocol/posix_mapping.md) - POSIX API 對照
> - [progress/stories.md](progress/stories.md) - 開發任務追蹤

---
## 1. Project Goals

### 1.1 Course Requirements Fit

- 可控制 TCP / UDP connection 建立與資料交換
- 能透過 CLI 清楚看到通訊流程與封包內容（application-layer protocol）
- 支援 LAN client-server 架構

### 1.2 Product Definition (Scope)

- 固定 4 人牌局（n 真人 + 4-n AI，n ∈ {1..4}）
- 回合制（turn-based）簡化紙牌規則（Trick Duel）
  - **NoKing / No Trump Rule**: 只有與領牌（Leading Card）同花色的牌能贏得 Trick，無王牌（Trump）設定。
  - 標準撲克牌 52 張 (A-K)，每人 13 張。
- TCP：遊戲協議（NDJSON）
- UDP：heartbeat（RTT/loss demo；非關鍵狀態）

---

## 2. High-Level Architecture

```text

              (LAN / WSL Host)

        +---------------------------+

        |        Host Node          |

        |  Rust + socket2 + std     |

        |---------------------------|

        | Lobby/Matchmaking         |

        | Game State Machine        |

        | TCP NDJSON Protocol       |

        | UDP Heartbeat             |

        +-----------+---------------+

                    |

     +--------------+------------------------------+

     |              |              |              |

 Human Client     Human Client     AI Client      AI Client

 (Python)         (Python)         (Python)       (Python)

```

### 2.1 Networking Core Decision

*`socket2`：POSIX lifecycle 對照（socket/bind/listen/accept/setsockopt/backlog）

*`std::net`：實際 I/O（TcpStream/UdpSocket）

*`std::thread + mpsc`：多工與事件傳遞（易解釋、規模小）

---

## 3. Module Breakdown

### 3.1 Server Modules (Rust)

1.`net::listener_tcp`

* socket2 建立 listener
* options（reuseaddr/reuseport）、listen(backlog)
* accept loop：接到連線後交給 connection handler

2.`net::conn_tcp`

* per-connection handler thread
* NDJSON framing：讀取 line → parse JSON → emit Event
* send queue：Server → Client 的 outbound messages

3.`net::heartbeat_udp`

* UdpSocket bind
* 接收 HB_PING / 回 HB_PONG
* 產生 metrics（RTT/loss）供 CLI log 或管理模組使用

4.`protocol`

* message types（HELLO/WELCOME/DEAL/YOUR_TURN/PLAY/STATE/ERROR/...）
* schema / validation（最低限度）

5.`lobby`

* 註冊 nickname、role(HUMAN/AI)
* 等待人數滿足開局條件
* 產生 match（room table）

6.`game::engine`

* state machine：setup → dealing → turns → scoring → game_over
* move validation（card 是否在手牌、是否輪到他）
* broadcast state

7.`log`

* 統一 log format（Server CLI 顯示）
* 可選：存檔 replay（bonus）

### 3.2 Client Modules (Python)

1.`clients/common/codec.py`
* NDJSON encode/decode (Length-prefixed or Newline-delimited)
* Common message models

2.`clients/human_gui` (Managed by @Gemini)
* **Architecture**: Threaded GUI model.
  * **Main Thread**: Tkinter `mainloop`, handling UI events and rendering.
  * **Network Thread**: Blocking socket receive loop, pushing messages to a thread-safe `queue.Queue`.
* **Features**:
  * Login screen (IP, Port, Nickname)
  * Game board (Canvas-based card rendering)
  * Real-time status updates (Scoreboard, Game Log)

3.`clients/ai_cli` (Managed by @Gemini)
* connect → HELLO(role=AI, auth)
* 收到 YOUR_TURN → 產生 prompt → Gemini API call
* parse response（must be JSON-like）
* fallback：timeout/invalid/illegal → rule-based move

4.`clients/cpp_cli` (Managed by @Gemini)
* **Core**: `sys/socket.h` (Linux) and `winsock2.h` (Windows) abstraction.
* **Compatibility Layer**: Simple `#ifdef _WIN32` wrapper for socket initialization (`WSAStartup`).
* **Threading**: `std::thread` (Cross-platform since C++11).
* **UI**: Text-based interface (CLI) with `iostream`.
* **JSON**: Minimal JSON parser/builder (or `nlohmann/json`).

5.`clients/common/heartbeat.py`
* UDP ping loop
* CLI/GUI 顯示 RTT/loss

---

## 4. Python GUI Architecture (Detailed)

### 4.1 UI Components (Tkinter)
- **LoginFrame**: Connection settings and player identification.
- **LobbyFrame**: Waiting room status, showing connected players.
- **GameFrame**:
    - **Canvas**: Main drawing area for cards, table, and animations.
    - **Control Panel**: Text log for game events, player list with scores.

### 4.2 Card Rendering Strategy
To avoid dependency on external image assets, we will implement a `CardPainter` utility:
- Use `Canvas.create_rectangle` for card shapes (rounded corners).
- Use `Canvas.create_text` for rank and suit symbols (Unicode supported).
- Color-coded: Red for Hearts/Diamonds, Black for Spades/Clubs.

### 4.3 Threading & Synchronization
```python
# Conceptual loop in human_gui.py
def process_messages(self):
    try:
        while True:
            msg = self.queue.get_nowait()
            self.handle_server_msg(msg)
    except queue.Empty:
        pass
    self.after(100, self.process_messages)
```
This ensures the GUI remains responsive while the network thread handles blocking I/O.

---

## 4. Protocol Design

### 4.1 Transport

* TCP：game control + state sync
* UDP：heartbeat only (no game-critical data)

### 4.2 Message Framing

* NDJSON：每個 JSON 一行，`\n` 作為 frame boundary

### 4.3 Minimal Message Types (MVP)

*`HELLO` / `WELCOME` / `ERROR`

*`ROOM_WAIT` / `ROOM_START`

*`DEAL`

*`YOUR_TURN`

*`PLAY` / `PLAY_REJECT`

*`PLAY_RESULT`

*`GAME_OVER`

### 4.4 Role/Auth (AI separation without extra port)

* AI client 使用 `role="AI"` 並附 `auth`（shared secret or token）
* Human client 使用 `role="HUMAN"`，可不需 token
* Server 端進行 role-based validation

---

## 5. Threading Model (Server)

**MVP 建議：thread-per-connection + central game loop**

* Thread A: TCP accept loop
* Thread B..E: client handler threads（最多 4）
* Thread G: game engine loop（從 mpsc 收事件、更新狀態、broadcast）
* Thread H: UDP heartbeat loop（可獨立）

Event flow:

* conn thread → `mpsc::Sender<Event>` → game loop
* game loop → per-conn outbound queue（或 sender channel）

---

## 6. Milestones (Suggested)

### M0 - Repo scaffold

* directory structure
* protocol.md + stories.md

### M1 - TCP server skeleton

* socket2 listener + accept
* single client echo + NDJSON decode/encode
* server CLI logs

### M2 - Lobby + 4-player start rule

* register nickname/role
* enforce n humans + (4-n) AIs
* room creation + broadcast ROOM_START

### M3 - Game engine MVP

* deal cards
* turn rotation
* play validation + scoring
* game over

### M4 - UDP heartbeat

* ping/pong + metrics display

### M5 - AI client (Gemini + fallback)

* prompt template
* response parsing + fallback
* safety: illegal move → retry/fallback

### M6 - Demo scripts + report

* one-command local demo
* README finalize + diagrams

---

## 7. Testing Strategy

* Unit tests (Rust):

  * protocol parsing
  * game validation
  * scoring logic
* Integration tests:

  * spawn server + 4 clients (script)
  * deterministic seed for dealing (reproducible)

---

## 8. Deliverables Checklist (for final demo)

- [ ] TCP connection lifecycle visible in logs
- [ ] UDP heartbeat metrics visible
- [ ] One full game completes deterministically
- [ ] AI fallback works without external API

---

## 9. Module API Specifications

### 9.1 Server Modules (Rust)

#### `net::listener` - TCP Listener

```rust
/// TCP Server Listener using socket2
pub struct TcpListener {
    socket: Socket,
    addr: SocketAddr,
}

impl TcpListener {
    /// Create and bind a new TCP listener
    /// - Sets SO_REUSEADDR
    /// - Calls listen(backlog)
    pub fn bind(addr: SocketAddr, backlog: i32) -> io::Result<Self>;

    /// Accept incoming connection
    /// Returns (TcpStream, peer_addr)
    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)>;
}
```

#### `net::connection` - Connection Handler

```rust
/// Per-connection handler running in its own thread
pub struct ConnectionHandler {
    id: ConnectionId,
    stream: TcpStream,
    peer_addr: SocketAddr,
    tx: Sender<GameEvent>,
}

impl ConnectionHandler {
    /// Main loop: read NDJSON messages and forward to game loop
    pub fn run(&mut self);

    /// Send message to client
    pub fn send(&mut self, msg: &Message) -> io::Result<()>;
}

/// Events sent from connection handlers to game loop
pub enum GameEvent {
    Connected { conn_id: ConnectionId, addr: SocketAddr },
    Disconnected { conn_id: ConnectionId },
    Message { conn_id: ConnectionId, msg: Message },
}
```

#### `protocol::messages` - Message Types

```rust
/// All protocol message types
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    // Connection phase
    Hello { role: Role, nickname: String, proto: u8, auth: Option<String> },
    Welcome { player_id: String, nickname: String, room: String },
    Error { code: String, message: String },

    // Lobby phase
    RoomWait { room: String, players: Vec<PlayerInfo>, need: u8 },
    RoomStart { room: String, players: Vec<PlayerInfo>, seed: u64 },

    // Game phase
    Deal { hand: Vec<Card>, total_tricks: u8 },
    YourTurn { trick: u8, table: Vec<PlayedCard>, legal: Vec<Card>, timeout_ms: u32 },
    Play { card: Card },
    PlayBroadcast { player_id: String, card: Card, trick: u8 },
    PlayReject { card: Card, reason: String },
    TrickResult { trick: u8, plays: Vec<PlayedCard>, winner: String, score: Score },
    GameOver { final_score: Score, winner: String, history: Vec<TrickHistory> },
}
```

#### `lobby::room` - Room Manager

```rust
/// Room manager handling matchmaking
pub struct RoomManager {
    rooms: HashMap<RoomId, Room>,
}

impl RoomManager {
    /// Add player to room, returns room state
    pub fn add_player(&mut self, player: PlayerInfo) -> RoomState;

    /// Check if room is ready to start
    pub fn is_ready(&self, room_id: &RoomId) -> bool;

    /// Get room for player
    pub fn get_player_room(&self, player_id: &PlayerId) -> Option<&Room>;
}
```

#### `game::engine` - Game Engine

```rust
/// Game state machine
pub struct GameEngine {
    state: GameState,
    players: [Player; 4],
    deck: Deck,
    current_trick: Vec<PlayedCard>,
    scores: Score,
}

impl GameEngine {
    /// Start new game with seed
    pub fn new(players: [PlayerInfo; 4], seed: u64) -> Self;

    /// Deal cards to all players
    pub fn deal(&mut self) -> Vec<DealResult>;

    /// Process a play action
    pub fn play(&mut self, player_id: &PlayerId, card: Card) -> PlayResult;

    /// Get legal moves for current player
    pub fn legal_moves(&self) -> Vec<Card>;

    /// Get current game state
    pub fn state(&self) -> &GameState;
}
```

### 9.2 Client Modules (Python)

#### `common/codec.py` - NDJSON Codec

```python
class NDJsonCodec:
    """NDJSON encoder/decoder for TCP socket"""

    def __init__(self, sock: socket.socket):
        self.sock = sock
        self.buffer = b""

    def send(self, msg: dict) -> None:
        """Send a message as NDJSON"""
        data = json.dumps(msg).encode('utf-8') + b'\n'
        self.sock.sendall(data)

    def recv(self) -> Optional[dict]:
        """Receive a message, returns None if connection closed"""
        while b'\n' not in self.buffer:
            data = self.sock.recv(4096)
            if not data:
                return None
            self.buffer += data
        line, self.buffer = self.buffer.split(b'\n', 1)
        return json.loads(line.decode('utf-8'))
```

#### `common/messages.py` - Message Models

```python
@dataclass
class Hello:
    role: str  # "HUMAN" or "AI"
    nickname: str
    proto: int = 1
    auth: Optional[str] = None

    def to_dict(self) -> dict:
        d = {"type": "HELLO", "role": self.role, "nickname": self.nickname, "proto": self.proto}
        if self.auth:
            d["auth"] = self.auth
        return d

@dataclass
class YourTurn:
    trick: int
    table: List[dict]
    legal: List[str]
    timeout_ms: int

    @classmethod
    def from_dict(cls, d: dict) -> "YourTurn":
        return cls(d["trick"], d["table"], d["legal"], d["timeout_ms"])
```

#### `ai_cli/gemini.py` - Gemini AI Client

```python
class GeminiPlayer:
    """AI player using Gemini API for decision making"""

    def __init__(self, api_key: str, fallback: FallbackStrategy):
        self.client = genai.GenerativeModel('gemini-pro')
        self.fallback = fallback

    def choose_card(self, hand: List[str], legal: List[str],
                    table: List[dict], trick: int) -> str:
        """Choose a card to play using Gemini API"""
        try:
            prompt = self._build_prompt(hand, legal, table, trick)
            response = self.client.generate_content(prompt)
            card = self._parse_response(response.text)
            if card in legal:
                return card
        except Exception as e:
            log.warning(f"Gemini API failed: {e}")

        # Fallback
        return self.fallback.choose(legal)

    def _build_prompt(self, hand, legal, table, trick) -> str:
        """Build prompt for Gemini"""
        return f"""You are playing a card game.
Your hand: {hand}
Legal moves: {legal}
Cards on table: {table}
Trick number: {trick}

Choose ONE card from legal moves. Reply with JSON: {{"card": "XX"}}"""
```

#### `ai_cli/fallback.py` - Fallback Strategy

```python
class FallbackStrategy:
    """Rule-based fallback when Gemini fails"""

    def choose(self, legal: List[str]) -> str:
        """Choose the smallest legal card"""
        return min(legal, key=self._card_value)

    def _card_value(self, card: str) -> int:
        """Get numeric value of card for comparison"""
        rank = card[:-1]  # Remove suit
        rank_order = {"A": 14, "K": 13, "Q": 12, "J": 11, "10": 10,
                      "9": 9, "8": 8, "7": 7, "6": 6, "5": 5,
                      "4": 4, "3": 3, "2": 2}
        return rank_order.get(rank, 0)
```

---

## 10. Error Handling Strategy

### Server-side (Rust)

| Error Type | Handling |
|------------|----------|
| Invalid JSON | Send ERROR(PROTOCOL_ERROR), keep connection |
| Invalid Message | Send ERROR with reason, keep connection |
| Connection Lost | Remove from room, notify others |
| Timeout | Auto-play smallest legal card |
| Thread Panic | Catch at spawn, clean up resources |

### Client-side (Python)

| Error Type | Handling |
|------------|----------|
| Connection Failed | Retry 3 times, then exit |
| Invalid Server Response | Log and continue |
| Gemini API Error | Use fallback strategy |
| Timeout | Reconnect or exit gracefully |


