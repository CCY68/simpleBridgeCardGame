# CardArena (LAN Socket Programming Final Project)

> 期末作業目標：做出「可被解釋的網路應用程式」
> 技術重點：可控制 TCP/UDP 連線建立與資料交換，並能在 CLI 清楚觀察通訊過程

CardArena 是一個在 **LAN 環境**中運作的回合制紙牌對戰網路應用：
- **Host Node（Server）**：負責 lobby、配桌、發牌、回合、驗證出牌、計分（權威狀態機）
- **Clients（Python / C++）**：
  - Human client (Python)：真人加入、輸入動作、顯示狀態
  - AI client (Python)：透過 Gemini LLM API 決策（含 rule-based fallback）
  - C++ Client：使用 `sys/socket.h` 實作的 CLI/GUI client
- **TCP**：遊戲控制與狀態同步（NDJSON）
- **UDP**：heartbeat（RTT / loss rate 展示，非關鍵狀態）

---

## Quick Links

| Document | Description |
|----------|-------------|
| [PROJECT.md](PROJECT.md) | 架構設計與開發計畫 |
| [protocol/protocol.md](protocol/protocol.md) | 完整協議規格 |
| [protocol/posix_mapping.md](protocol/posix_mapping.md) | POSIX Socket API 對照表 |
| [docs/user_manual.md](docs/user_manual.md) | 使用者操作手冊 |
| [docs/integration_test_manual.md](docs/integration_test_manual.md) | 整合測試手冊 |
| [docs/term_report.md](docs/term_report.md) | Socket 程式碼報告 |
| [progress/stories.md](progress/stories.md) | 開發任務追蹤 |
| [.claude/CLAUDE.md](.claude/CLAUDE.md) | Claude Code 開發指引 |
| [GEMINI.md](GEMINI.md) | Gemini CLI 開發指引 |

---

## 1. Features

### 1.1 LAN-only Client–Server
- Server 在 LAN 指定 IP/Port 上 `bind/listen/accept`
- Clients 透過 IP 連線到 Server
- 支援 WSL host + Windows/WSL clients + Web（可選，非主線）

### 1.2 Lobby 與開局條件（必要）
每局固定 4 位玩家才能開始：  
- `n` 位真人 + `(4-n)` 位 AI，`n ∈ {1,2,3,4}`
- 例如固定作業 demo：`n=2` 真人 + `2` AI

### 1.3 TCP 遊戲協議（NDJSON）
每個 TCP 訊息是一行 JSON（newline-delimited JSON），便於 CLI 觀察與 framing：
```json
{"type":"HELLO","role":"HUMAN","nickname":"W1","proto":1}
{"type":"WELCOME","player_id":"P1","room":"R001"}
{"type":"DEAL","hand":["AS","7H","9D"],"n":10}
{"type":"YOUR_TURN","trick":1,"legal":["AS","7H","9D"]}
{"type":"PLAY","card":"9D"}
{"type":"PLAY_RESULT","trick":1,"winner":"P3","score":{"HUMAN":0,"AI":1}}
```

### 1.4 UDP Heartbeat（展示 connectionless）

* Client → Server：`HB_PING(seq, t_client_ms)`
* Server → Client：`HB_PONG(seq, t_client_ms, t_server_ms)`
  CLI 顯示：
* last RTT / avg RTT
* loss rate（依 seq gap 估算）

### 1.5 AI 決策（Gemini LLM + fallback）

* AI client 透過 Gemini API 決策出牌
* 若 API timeout / 回覆格式錯誤 / 非法牌 → fallback（rule-based）
  例：出「最小合法牌」或「最大合法牌」（可配置）

> Gemini API 是否會產生額外費用：
> 我們的設計已內建「可關閉 LLM」與「fallback」模式；實際費用取決於你在 Google AI Studio / Cloud 的 API 計費與用量設定，請以你帳號的 billing/usage 顯示為準。

---

## 2. Tech Stack（定案）

### Host Node（Networking Core）

* Rust
* `socket2`：建立 TCP listener、設定 socket options、`listen(backlog)`、`accept`
* `std::net::TcpStream / UdpSocket`：實際 I/O
* `std::thread + std::sync::mpsc`：threading management / event queue

### Clients

* Python
  * `socket`：TCP 直連（human / ai）
  * AI client：Gemini API 呼叫 + fallback
* C++
  * `sys/socket.h`：POSIX socket 直連
  * Standard Library (`iostream`, `string`, `thread`)

---

## 3. POSIX Socket 對照（作業報告用）

我們使用 Rust + socket2 來逐行對應 POSIX socket lifecycle（socket/bind/listen/accept/setsockopt），並將 accept 後的連線轉為 `std::net::TcpStream` 進行 I/O（等價於 send/recv 的 loop）。

### C++ socket.h ↔ Rust (socket2 + std::net) 對照表

| POSIX / C++ `socket.h`            | Rust `socket2` / `std::net`                                                         | 說明                                          |
| --------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------- |
| `socket(AF_INET, SOCK_STREAM, 0)` | `Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))`                      | 一對一建立 TCP socket                            |
| `setsockopt(SO_REUSEADDR)`        | `socket.set_reuse_address(true)`                                                    | 一對一設定 option                                |
| `bind(fd, addr)`                  | `socket.bind(&addr.into())`                                                         | 一對一 bind                                    |
| `listen(fd, backlog)`             | `socket.listen(backlog)`                                                            | **std::net 無 backlog**，socket2 可一對一         |
| `accept(fd, ...)`                 | `socket.accept()`                                                                   | 回 `(Socket, SockAddr)`；可轉 `TcpStream`       |
| `connect(fd, addr)`               | `socket.connect(&addr.into())` 或 `TcpStream::connect()`                             | 若要逐行展示用 socket2；簡化可用 std::net               |
| `send(fd, ...)`                   | `TcpStream.write()/write_all()`                                                     | `write_all` 等價於自行 loop send                 |
| `recv(fd, ...)`                   | `TcpStream.read()` / `BufRead::read_line()`                                         | `read_line` 用於 NDJSON framing（等價 buffer 拆包） |
| `socket(AF_INET, SOCK_DGRAM, 0)`  | `Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))` 或 `UdpSocket::bind()` | UDP socket                                  |
| `sendto/recvfrom`                 | `UdpSocket.send_to/recv_from`                                                       | 一對一                                         |
| `fcntl(O_NONBLOCK)`               | `set_nonblocking(true)`                                                             | 一對一                                         |
| `select/poll/epoll`               | （std 無）threads + mpsc（主線）/ `mio`（加分）                                                | 我們主線採 threads（規模小、易解釋）                      |

---

## 4. Repository Structure

```text
.
├─ README.md
├─ PROJECT.md
├─ .gitignore
├─ protocol/
│  ├─ protocol.md
│  └─ posix_mapping.md
├─ server/                      # Rust host node
│  ├─ src/
│  ├─ tests/
│  └─ Cargo.toml
├─ clients/
│  ├─ human_gui/                # Python (Tkinter GUI)
│  ├─ ai_cli/                   # Python (Gemini + fallback)
│  ├─ cpp_cli/                  # C++ (POSIX socket)
│  └─ common/                   # codec, message models
├─ progress/
│  ├─ stories.md
│  └─ notes.md
└─ scripts/
   ├─ run_local_demo.sh
   └─ run_local_demo.ps1
```

---

## 5. How to Run (Planned)

> 初版先定義操作方式；實作完成後補上實際指令與參數。

### 5.1 Start Server (WSL)

* bind 到 `0.0.0.0:<TCP_PORT>`（LAN 可連）
* UDP heartbeat bind 到 `0.0.0.0:<UDP_PORT>`

### 5.2 Start Clients

* Human client xN：指定 nickname
* AI client x(4-N)：指定 AI token / API key / fallback mode

---

## 6. Development Workflow (Git Flow)

Branches:

* `main`：可交付版本
* `dev`：整合分支
* `feature/*`：功能開發分支
* `release/*`：release stabilization

---

## 7. LLM Collaboration Guide

本專案採用多 LLM 協作開發模式：

### Role Distribution

| LLM Tool | Primary Role | Guidelines |
|----------|--------------|------------|
| **Claude Code** | 主要開發 | 實作功能、撰寫測試、commit。參考 [.claude/CLAUDE.md](.claude/CLAUDE.md) |
| **Gemini CLI** | Code Review / QA | 審查程式碼、找 bug、架構建議。參考 [GEMINI.md](GEMINI.md) |
| **Web LLM** | Brainstorming | 規劃、文件撰寫、複雜問題討論 |

### How to Start Development

**For Claude Code:**
```bash
# 進入專案目錄
cd /path/to/networkCardGame

# 查看目前進度
cat progress/stories.md | grep "IN_PROGRESS\|TODO"

# 開始開發（Claude 會讀取 .claude/CLAUDE.md）
claude
```

**For Gemini CLI:**
```bash
# 進入專案目錄
cd /path/to/networkCardGame

# 開始開發（Gemini 會讀取 GEMINI.md）
gemini
```

> 詳細工作流程請參考各自的指引文件。

---

## 8. License

TBD (for course project)

