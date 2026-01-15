# CardArena Client Development Stories

> 本文件專門追蹤 Client 端 (Python) 的開發進度。
> Server 端與整合進度請參閱 `stories.md`。

---

## Status Legend

| Status | Meaning |
|--------|---------|
| `TODO` | 尚未開始 |
| `IN_PROGRESS` | 進行中 |
| `DONE` | 已完成 |
| `BLOCKED` | 被阻擋 |

---

## EPIC 4 - UDP Heartbeat

### S4.2 Client heartbeat loop (Python) `[P1]` `DONE` `@Gemini`

**依賴**: S5.1
**檔案**: `clients/common/heartbeat.py`
**驗收指令**: CLI 顯示 RTT 和 loss rate

**DoD**:
- [x] UDP socket 建立
- [x] 每秒發送 HB_PING (seq 遞增)
- [x] 計算 RTT = now - t_client_ms
- [x] 計算 loss rate = missed_seq / total_sent
- [x] CLI 顯示 metrics

---

## EPIC 5 - Clients (Core)

### S5.1 Human CLI client - connection `[P0]` `DONE` `@Gemini`

**依賴**: S2.1
**檔案**: `clients/human_cli/main.py`, `clients/common/codec.py`
**驗收指令**: 能連線並完成 handshake

**DoD**:
- [x] `socket.socket(AF_INET, SOCK_STREAM)`
- [x] NDJSON codec (encode/decode)
- [x] 送 HELLO，收 WELCOME
- [x] 顯示 player_id 和 room

### S5.2 Human CLI client - game loop `[P0]` `DONE` `@Gemini`

**依賴**: S5.1, S3.2
**檔案**: `clients/human_cli/game.py`
**驗收指令**: 能完成一局遊戲

**DoD**:
- [x] 顯示手牌
- [x] 收到 YOUR_TURN 時提示輸入
- [x] 從 stdin 讀取選擇的牌
- [x] 送 PLAY message
- [x] 處理 PLAY_REJECT (重新輸入)
- [x] 顯示 TRICK_RESULT 和分數
- [x] 顯示 GAME_OVER 結果

### S5.3 AI CLI client - fallback mode `[P0]` `DONE` `@Gemini`

**依賴**: S5.1
**檔案**: `clients/ai_cli/main.py`, `clients/ai_cli/fallback.py`
**驗收指令**: AI 能用 rule-based 完成一局

**DoD**:
- [x] 連線並 handshake (role=AI, auth)
- [x] 收到 YOUR_TURN 時自動選牌
- [x] Fallback 策略：出最小合法牌
- [x] 確保永遠不送非法牌
- [x] 處理 PLAY_REJECT (換牌)

### S5.4 AI CLI client - Gemini integration `[P1]` `DONE` `@Gemini`

**依賴**: S5.3
**檔案**: `clients/ai_cli/gemini.py`
**驗收指令**: AI 能用 Gemini API 決策

**DoD**:
- [x] 安裝 `google-generativeai` SDK
- [x] 組裝 prompt (當前牌局狀態)
- [x] 呼叫 Gemini API
- [x] 解析回應 (預期 JSON 格式)
- [x] 驗證選牌是否合法
- [x] API 失敗/timeout → fallback
- [x] Rate limit 處理

---

## EPIC 7 - Python GUI Client (Tkinter)

### S7.1 GUI Scaffold & Threaded Bridge `[P0]` `DONE` `@Gemini`
**依賴**: S5.1
**檔案**: `clients/human_gui/app.py`
**DoD**:
- [x] 實作 Tkinter `App` 類別與主迴圈
- [x] 整合 `NetworkClient` 與 `queue.Queue`
- [x] 實作 `.after()` 輪詢機制處理網路訊息
- [x] 基礎 Login 介面 (IP, Port, Name)

### S7.2 Card Rendering Engine `[P1]` `DONE` `@Gemini`
**依賴**: S7.1
**檔案**: `clients/human_gui/components/card.py`
**DoD**:
- [x] 在 Canvas 上繪製向量撲克牌 (非圖片)
- [x] 支援顯示花色、點數、背面狀態
- [x] 實作手牌佈局演算法 (扇形或直線排列)

### S7.3 Game Board & Interaction `[P1]` `DONE` `@Gemini`
**依賴**: S7.2, S3.2
**檔案**: `clients/human_gui/views/table.py`
**DoD**:
- [x] 渲染遊戲桌面、其他玩家位置、出牌區
- [x] 實作卡牌點擊事件 -> 送出 `PLAY` 訊息
- [x] 只有在 `YOUR_TURN` 且為 `legal` 的卡牌才可點擊 (視覺提示)

### S7.4 State Synchronization & Logging `[P2]` `DONE` `@Gemini`
**依賴**: S7.3
**檔案**: `clients/human_gui/app.py`
**DoD**:
- [x] 收到 `STATE` / `PLAY_BROADCAST` 時更新畫面
- [x] 右側 Side Panel 顯示遊戲日誌 (Log)
- [x] 即時更新分數看板

---

## EPIC 8 - C++ Client (POSIX Socket)

### S8.1 C++ Scaffold & Makefile `[P1]` `DONE` `@Gemini`
**依賴**: S0.1
**檔案**: `clients/cpp_cli/Makefile`
**DoD**:
- [x] 專案目錄結構 `clients/cpp_cli/`
- [x] Makefile 支援 `make clean`, `make all`
- [x] 支援 Linux (`g++`) 環境

### S8.2 TCP Connection & Threading `[P1]` `DONE` `@Gemini`
**依賴**: S8.1
**檔案**: `clients/cpp_cli/net/connection.cpp`
**DoD**:
- [x] 使用條件編譯處理 `sys/socket.h` (Linux) 與 `winsock2.h` (Windows)
- [x] Windows 環境下正確呼叫 `WSAStartup` / `WSACleanup`
- [x] 實作 Reader Thread 處理 `recv()`
- [x] 主執行緒處理 `cin` 輸入
- [x] 處理 `Ctrl+C` graceful shutdown

### S8.3 NDJSON Protocol & Game Logic `[P1]` `TODO` `@Gemini`
**依賴**: S8.2
**檔案**: `clients/cpp_cli/game/protocol.cpp`
**DoD**:
- [ ] 實作 NDJSON framing (`\n` 分隔)
- [ ] JSON 序列化/反序列化 (建議使用 `nlohmann/json` 或輕量級 parser)
- [ ] 處理 `HELLO`, `DEAL`, `YOUR_TURN`, `PLAY_RESULT`, `GAME_OVER`
- [ ] CLI 顯示卡牌與遊戲狀態

### S8.4 UDP Heartbeat (C++) `[P2]` `TODO` `@Gemini`
**依賴**: S8.2
**檔案**: `clients/cpp_cli/net/heartbeat.cpp`
**DoD**:
- [ ] 建立 UDP socket (`SOCK_DGRAM`)
- [ ] 實作 Ping Loop (1s 間隔)
- [ ] 統計與顯示 RTT / Loss Rate

---

## Client Progress Summary

| EPIC | Total | Done | In Progress | TODO |
|------|-------|------|-------------|------|
| EPIC 4 - UDP Heartbeat | 1 | 1 | 0 | 0 |
| EPIC 5 - Clients (Core) | 4 | 4 | 0 | 0 |
| EPIC 7 - GUI Client | 4 | 4 | 0 | 0 |
| EPIC 8 - C++ Client | 4 | 2 | 0 | 2 |
| **Total** | **13** | **11** | **0** | **2** |

---

## Changelog

### 2026-01-14 (Code Review Fix by @Claude)
- **AI CLI 修正** (`clients/ai_cli/app.py`):
  - 新增 UDP Heartbeat 支援 (原本缺失)
  - 新增 PLAY_REJECT 處理邏輯
  - 新增 ROOM_WAIT, ROOM_START, PLAY_BROADCAST 訊息處理
  - 修正 TRICK_RESULT 分數更新邏輯
