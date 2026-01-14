# Development Notes

> 記錄技術決策、討論、review 結果等。

---

## Project Status Summary

**Date**: 2026-01-14
**Status**: Client-side Core Development Complete

### Current Progress
- **Client Side (@Gemini)**:
    - EPIC 5 (CLI) & EPIC 7 (GUI) are fully implemented.
    - S4.2 (UDP Heartbeat Client) is implemented and verified.
    - Gemini AI integration for AI CLI is active.
- **Server Side (@Claude)**:
    - EPIC 1 (TCP Core) & EPIC 2 (Lobby) are reported as completed.
- **Next Step**: Integration testing between Server and Client using the established protocol.

---

## Technical Decisions

### TD-001: Why socket2 instead of tokio?

**Date**: 2025-01-13
**Decision**: 使用 `socket2` + `std::thread` 而非 `tokio`

**Rationale**:
1. 課程要求展示 POSIX Socket API 對應
2. `socket2` 可直接控制 `listen(backlog)`, `SO_REUSEADDR` 等
3. Thread-per-connection 模型更易解釋
4. 4 人遊戲規模不需要 async I/O

**Trade-offs**:
- (+) 程式碼與 C/C++ POSIX API 一對一對應
- (+) 容易 debug，每個 thread 獨立
- (-) 不適合大量連線（但本專案只有 4 人）

---

### TD-002: Why NDJSON instead of Length-Prefixed?

**Date**: 2025-01-13
**Decision**: 使用 NDJSON (newline-delimited JSON)

**Rationale**:
1. CLI 可直接觀察封包內容
2. 使用 `BufReader::read_line()` 即可 framing
3. 對 debug 友善

**Trade-offs**:
- (+) Human readable
- (+) 容易用 `nc` 測試
- (-) 無法傳送含 newline 的資料（但 JSON 會 escape）

---

## Code Reviews

<!--
Template:

### Review: S1.1 TCP listener setup
**Reviewer**: Gemini CLI
**Date**: YYYY-MM-DD
**Status**: APPROVED / NEEDS_CHANGES

#### Findings
- [ ] Issue: xxx
- [x] Good: yyy

#### Suggestions
1. ...

#### Action Items
- [ ] Fix xxx
- [ ] Add test for yyy
-->

---

## Test Results

### 2026-01-14: QA Verification Run

**Scope**:
- Server unit tests
- Client unit tests (codec/heartbeat/fallback)
- UDP heartbeat integration script

**Commands**:
- `cd server && cargo test`
- `pytest clients/`
- `python scripts/test_heartbeat.py`

**Results**:
- Server: 37 passed (warnings: unused imports/variables)
- Clients: 5 passed
- Heartbeat script: HB_PING/HB_PONG exchange OK, RTT/loss metrics reported

---

## Meeting Notes

<!--
Template:

### 2025-01-13: Initial Planning
**Attendees**: Human, Claude, Gemini

#### Discussed
- ...

#### Decisions
- ...

#### Action Items
- [ ] @Claude: ...
- [ ] @Gemini: ...
-->
