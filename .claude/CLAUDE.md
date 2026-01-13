# CardArena - Claude Code Development Guide

> 本文件提供 Claude Code 開發此專案時的必要資訊與指引。

---

## Your Role: Server-Side Development

**Claude Code 負責 Server (Rust) 開發**，Gemini CLI 負責 Client (Python) 開發。

### 你的開發範圍
```
server/                 ← 你負責這裡
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── net/            # TCP listener, connection handler, UDP heartbeat
│   ├── protocol/       # Message types, NDJSON codec
│   ├── lobby/          # Handshake, room management
│   └── game/           # Game engine, scoring
└── tests/
```

### 協作契約
- **Protocol 規格**: `protocol/protocol.md` (已固定，雙方共同遵守)
- **整合測試**: `protocol/integration_tests.md` (驗收標準)
- **進度追蹤**: `progress/stories.md`

---

## Project Overview

CardArena 是一個 LAN 環境的回合制紙牌對戰遊戲，作為 Socket Programming 期末作業。

- **Server**: Rust (`socket2` + `std::net` + `std::thread`)
- **Clients**: Python (`socket` 直連)
- **Protocol**: TCP (NDJSON) + UDP (Heartbeat)

---

## Current Status

| Milestone | Status | Description |
|-----------|--------|-------------|
| M0 - Repo scaffold | IN_PROGRESS | 目錄結構、文件 |
| M1 - TCP server skeleton | TODO | socket2 listener + accept |
| M2 - Lobby + 4-player rule | TODO | 配對邏輯 |
| M3 - Game engine MVP | TODO | 發牌、出牌、計分 |
| M4 - UDP heartbeat | TODO | RTT/loss 展示 |
| M5 - AI client (Gemini) | TODO | LLM 決策 + fallback |
| M6 - Demo scripts | TODO | 一鍵 demo |

**當前任務**: 請參考 `progress/stories.md` 中標記為 `IN_PROGRESS` 的 Story。

---

## Development Conventions

### Language & Encoding
- 文件、commit message、程式註解：使用**繁體中文**或**英文**
- 避免使用簡體中文

### Rust (Server)
- Formatter: `cargo fmt`
- Linter: `cargo clippy`
- Test: `cargo test`
- 檔案位置: `server/src/`

### Python (Clients)
- Formatter: `black`
- Linter: `ruff` 或 `flake8`
- Test: `pytest clients/`
- 檔案位置: `clients/`

### Git Workflow
- Branch: `main` (可交付) / `dev` (整合) / `feature/*`
- 每個 Story 對應一個 commit 或 PR
- Commit message 格式: `[S1.2] Implement accept loop`

---

## Directory Structure

```
.
├── .claude/CLAUDE.md      # 本文件 (Claude Code 指引)
├── README.md              # 專案說明
├── PROJECT.md             # 架構設計與計畫
├── protocol/
│   ├── protocol.md        # 完整協議規格
│   └── posix_mapping.md   # POSIX Socket 對照表
├── server/                # Rust Host Node
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── net/           # listener_tcp, conn_tcp, heartbeat_udp
│   │   ├── protocol/      # message types, codec
│   │   ├── lobby/         # matchmaking
│   │   └── game/          # engine, scoring
│   └── tests/
├── clients/
│   ├── common/            # codec.py, models.py
│   ├── human_cli/         # 真人 CLI client
│   └── ai_cli/            # AI client (Gemini + fallback)
├── progress/
│   └── stories.md         # 開發任務追蹤
└── scripts/
    ├── run_local_demo.sh
    └── run_local_demo.ps1
```

---

## Key Technical Decisions

### Why socket2 instead of tokio?
- 課程要求展示 POSIX Socket API 對應
- `socket2` 可直接設定 `listen(backlog)`, `SO_REUSEADDR` 等
- 易於解釋，適合教學目的

### Why NDJSON instead of Length-Prefixed?
- CLI 可直接觀察封包內容 (human readable)
- 每行一個 JSON，framing 簡單 (`\n` delimiter)
- 對 debug 友善

### Threading Model
- Main thread: TCP accept loop
- Per-connection threads: Client handler
- Game loop thread: 從 mpsc 收事件、更新狀態、broadcast
- UDP thread: Heartbeat 處理

---

## How to Develop a Story

1. **找到目標 Story**: 查看 `progress/stories.md`，找到標記 `@Claude` 且狀態為 `TODO` 的 Story
2. **標記開始**: 將狀態改為 `IN_PROGRESS`
3. **實作**: 依照 Story 的 DoD (Definition of Done) 完成實作
4. **測試**: 執行驗收指令，確認通過
5. **整合測試**: 用 `protocol/integration_tests.md` 的 mock client 測試
6. **標記完成**: 將狀態改為 `DONE`，勾選所有 checklist

### Server Stories (你負責)
- EPIC 1: TCP Networking Core (S1.1 - S1.5)
- EPIC 2: Lobby & Matchmaking (S2.1 - S2.3)
- EPIC 3: Game Engine MVP (S3.1 - S3.5)
- EPIC 4: UDP Heartbeat - Server side (S4.1, S4.3)

---

## Verification Commands

```bash
# Rust 編譯檢查
cd server && cargo check

# Rust 測試
cd server && cargo test

# Rust 格式化
cd server && cargo fmt

# Python 測試
pytest clients/

# 啟動 Server (開發完成後)
cd server && cargo run -- --port 8888

# 啟動 Human Client (開發完成後)
python clients/human_cli/main.py --host 127.0.0.1 --port 8888
```

---

## Common Pitfalls

1. **WSL 網路**: 若 Host 在 WSL，Windows Client 連線需注意 IP (使用 `ip addr` 查看 WSL IP)
2. **TCP 黏包**: NDJSON 用 `\n` 分隔，讀取時要用 `BufReader::read_line`
3. **Blocking I/O**: 每個 connection 一個 thread，避免 block 整個 server
4. **AI Fallback**: Gemini API 可能 timeout，必須有 fallback 邏輯

---

## LLM Collaboration Roles

| Tool | Role |
|------|------|
| Claude Code | 主要開發、實作、測試、commit |
| Gemini CLI | Code review、QA、架構討論 |
| Web LLM | Brainstorming、規劃、文件撰寫 |
