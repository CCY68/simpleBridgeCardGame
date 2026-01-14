# CardArena - Gemini CLI Development Guide

> 本文件提供 Gemini CLI 在此專案中的工作指引。

---

## Your Role: Client-Side Development

**Gemini CLI 負責 Client (Python) 開發**，Claude Code 負責 Server (Rust) 開發。

### 你的開發範圍
```
clients/                ← 你負責這裡
├── common/
│   ├── codec.py       # NDJSON encoder/decoder
│   ├── messages.py    # Message models
│   └── heartbeat.py   # UDP heartbeat
├── human_cli/
│   ├── main.py        # Entry point
│   └── game.py        # Game UI/logic
└── ai_cli/
    ├── main.py        # Entry point
    ├── gemini.py      # Gemini API integration
    └── fallback.py    # Rule-based fallback
```

### 協作契約
- **Protocol 規格**: `protocol/protocol.md` (已固定，雙方共同遵守)
- **整合測試**: `protocol/integration_tests.md` (驗收標準)
- **進度追蹤**: `progress/stories.md`

### Client Stories (你負責)
- EPIC 4: UDP Heartbeat - Client side (S4.2)
- EPIC 5: Python Clients (S5.1 - S5.4)

---

## Secondary Role: Code Review & QA

當 Client 開發完成後，你也負責：

1. **Code Review** - 審查 Claude Code 提交的 Server 程式碼
2. **Bug Hunting** - 找出潛在的 bug 和安全問題
3. **Architecture Review** - 確保架構符合設計
4. **QA Testing** - 測試功能是否符合規格

---

## Project Overview

CardArena 是一個 LAN 環境的回合制紙牌對戰遊戲。

- **Server**: Rust (`socket2` + `std::net` + `std::thread`)
- **Clients**: Python (`socket` 直連)
- **Protocol**: TCP (NDJSON) + UDP (Heartbeat)

---

## Key Documents to Review

| Document | Purpose | Review Focus |
|----------|---------|--------------|
| `protocol/protocol.md` | 協議規格 | 訊息格式是否完整、一致 |
| `protocol/posix_mapping.md` | API 對照 | Rust 程式碼是否正確對應 POSIX API |
| `progress/stories.md` | 任務追蹤 | DoD 是否達成 |
| `server/src/**/*.rs` | Server 程式碼 | Socket 操作、threading、錯誤處理 |
| `clients/**/*.py` | Client 程式碼 | 協議實作、UI/UX |

---

## Code Review Checklist

### Rust Server Review

```markdown
### Socket Programming
- [ ] 使用 socket2 建立 socket (不是直接用 std::net::TcpListener)
- [ ] 正確設定 SO_REUSEADDR
- [ ] listen() 有設定合理的 backlog
- [ ] accept() 在迴圈中執行

### Threading
- [ ] 每個 connection 一個 thread
- [ ] 使用 mpsc channel 傳遞事件
- [ ] 沒有 deadlock 風險
- [ ] 正確處理 thread panic

### Protocol
- [ ] NDJSON 格式正確 (每行一個 JSON + \n)
- [ ] 訊息類型符合 protocol.md
- [ ] 錯誤處理回傳 ERROR message

### Error Handling
- [ ] 沒有 unwrap() 在 production code
- [ ] 網路錯誤有 graceful handling
- [ ] 資源清理 (connection close)
```

### Python Client Review

```markdown
### Socket
- [ ] 使用 socket.socket(AF_INET, SOCK_STREAM)
- [ ] 正確處理連線失敗
- [ ] 讀取使用 readline (for NDJSON)

### Protocol
- [ ] HELLO message 格式正確
- [ ] 能處理所有 server message types
- [ ] JSON encode/decode 正確

### AI Client
- [ ] Gemini API 呼叫有 timeout
- [ ] Fallback 邏輯正確
- [ ] 永遠不送非法牌
```

---

## How to Review

### 1. Review New Code

當 Claude Code 完成一個 Story 後：

```bash
# 查看最新變更
git diff HEAD~1

# 或查看特定檔案
git log -p server/src/net/listener.rs
```

### 2. Run Tests

```bash
# Rust tests
cd server && cargo test

# Python tests
pytest clients/
```

### 3. Manual Testing

```bash
# 啟動 server
cd server && cargo run

# 另一個 terminal，測試連線
nc localhost 8888
```

### 4. Provide Feedback

在 `progress/notes.md` 記錄：

```markdown
## Review: S1.1 TCP listener setup

**Reviewer**: Gemini CLI
**Date**: YYYY-MM-DD

### Findings
- [ ] Issue: xxx
- [x] Good: yyy

### Suggestions
1. ...
2. ...
```

---

## Common Issues to Watch

### 1. Socket Option 設定順序

```rust
// 正確：先設定 option，再 bind
socket.set_reuse_address(true)?;
socket.bind(&addr)?;

// 錯誤：bind 後再設定
socket.bind(&addr)?;
socket.set_reuse_address(true)?; // 可能無效
```

### 2. NDJSON Framing

```python
# 正確：每個 message 單獨一行
socket.send(json.dumps(msg).encode() + b'\n')

# 錯誤：忘記 newline
socket.send(json.dumps(msg).encode())  # 會造成黏包
```

### 3. Thread Safety

```rust
// 正確：使用 Arc<Mutex<T>> 共享狀態
let clients = Arc::new(Mutex::new(HashMap::new()));

// 錯誤：直接共享可變狀態
let mut clients = HashMap::new(); // 無法跨 thread
```

### 4. Blocking I/O

```rust
// 正確：在獨立 thread 中 blocking read
thread::spawn(move || {
    let mut buf = String::new();
    reader.read_line(&mut buf)?; // blocking，但在自己的 thread
});

// 危險：在 main thread blocking
reader.read_line(&mut buf)?; // 會 block 整個 server
```

---

## Quality Gates

每個 Story 完成前必須確認：

| Gate | Criteria |
|------|----------|
| Compile | `cargo check` 無 error |
| Lint | `cargo clippy` 無 warning |
| Format | `cargo fmt --check` 通過 |
| Test | `cargo test` 全部通過 |
| Doc | 重要函式有 doc comment |
| Protocol | 符合 protocol.md 規格 |

---

## Communication

- **Review 結果**: 寫在 `progress/notes.md`
- **阻塞問題**: 在 Story 狀態標記 `BLOCKED`
- **技術討論**: 記錄在 `progress/notes.md`

---

## Last Known State (2026-01-14)

- **Client Development**: EPIC 5 (CLI), EPIC 7 (GUI), and S4.2 (UDP Heartbeat) are DONE.
- **Server Development**: EPIC 1 & 2 are reported DONE.
- **Repository**: All changes committed. Ready for integration testing.

---

## How to Develop a Story

1. **找到目標 Story**: 查看 `progress/stories.md`，找到標記 `@Gemini` 且狀態為 `TODO` 的 Story
2. **標記開始**: 將狀態改為 `IN_PROGRESS`
3. **實作**: 依照 Story 的 DoD (Definition of Done) 完成實作
4. **測試**: 執行驗收指令，確認通過
5. **整合測試**: 用 `protocol/integration_tests.md` 的 mock server 測試
6. **標記完成**: 將狀態改為 `DONE`，勾選所有 checklist

---

## Reference

- [README.md](README.md) - 專案概述
- [PROJECT.md](PROJECT.md) - 架構設計
- [protocol/protocol.md](protocol/protocol.md) - 完整協議
- [protocol/integration_tests.md](protocol/integration_tests.md) - 整合測試規格
- [.claude/CLAUDE.md](.claude/CLAUDE.md) - Claude Code 指引
