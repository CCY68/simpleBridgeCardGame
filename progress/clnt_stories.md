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

### S8.3 NDJSON Protocol & Game Logic `[P1]` `DONE` `@Gemini`
**依賴**: S8.2
**檔案**: `clients/cpp_cli/game/game_manager.cpp`
**DoD**:
- [x] 實作 NDJSON framing (`\n` 分隔)
- [x] 實作輕量級 JSON parser (json_helper.hpp) 避免外部依賴
- [x] 處理 `HELLO`, `DEAL`, `YOUR_TURN`, `PLAY_RESULT`, `GAME_OVER`
- [x] CLI 顯示卡牌與遊戲狀態，支援輸入 Index 出牌

### S8.4 UDP Heartbeat (C++) `[P2]` `DONE` `@Gemini`
**依賴**: S8.2
**檔案**: `clients/cpp_cli/net/heartbeat.cpp`
**DoD**:
- [x] 建立 UDP socket (`SOCK_DGRAM`)
- [x] 實作 Ping Loop (1s 間隔)
- [x] 統計與顯示 RTT / Loss Rate (整合至 CLI UI)

---

## EPIC 9 - Advanced AI Strategy

### S9.4 AI Card Strategy & Prompt Fix `[P1]` `DONE` `@Gemini`
**依賴**: S5.4
**檔案**: `clients/ai_cli/gemini_bridge.py`
**DoD**:
- [x] 修正 Prompt 產生時的 `KeyError: 'player'`
- [x] 確保 `TablePlay` 結構 (player_id, card) 正確傳遞給 LLM
- [x] 驗證 Human CLI 與 AI Client 處理 Table 訊息的一致性

---

## EPIC 11 - Admin GUI Tool (Python/Tkinter)

> **目標**: 提供圖形化的遠端管理介面，連線至 Server 的 Admin Port (8890)，
> 方便管理員監控伺服器狀態、查看訊息記錄、重設遊戲、及踢除玩家。

### S11.1 Admin GUI Scaffold & Connection `[P0]` `TODO` `@Claude`

**依賴**: S10.1 (Server Admin)
**檔案**: `clients/admin_gui/app.py`, `clients/admin_gui/connection.py`
**驗收指令**: 能連線並通過認證

**DoD**:
- [ ] 建立 `clients/admin_gui/` 目錄結構
- [ ] 實作 Tkinter 主視窗 (AdminApp)
- [ ] Login 介面: IP, Port, Auth Token
- [ ] TCP 連線與認證 (`AUTH <token>`)
- [ ] 連線狀態顯示 (Connected/Disconnected)

---

### S11.2 Dashboard & Status View `[P1]` `TODO` `@Claude`

**依賴**: S11.1
**檔案**: `clients/admin_gui/views/dashboard.py`
**驗收指令**: 顯示伺服器狀態、房間列表、玩家列表

**DoD**:
- [ ] Dashboard 主頁面佈局
- [ ] 伺服器狀態卡片 (uptime, connections)
- [ ] 房間列表 (TreeView) - ID, 狀態, 玩家數
- [ ] 玩家列表 (TreeView) - ID, 暱稱, 房間, 角色
- [ ] 定時自動更新 (5秒) 或手動重新整理

---

### S11.3 Message Log Viewer `[P1]` `TODO` `@Claude`

**依賴**: S11.1, S10.2
**檔案**: `clients/admin_gui/views/logs.py`
**驗收指令**: 顯示伺服器訊息記錄

**DoD**:
- [ ] 訊息記錄面板 (ScrolledText)
- [ ] 載入最近 N 條訊息 (`LOGS` 指令)
- [ ] 按事件類型篩選 (下拉選單)
- [ ] 搜尋功能 (關鍵字過濾)
- [ ] 自動捲動到最新訊息

---

### S11.4 Admin Actions (Reset & Kick) `[P0]` `TODO` `@Claude`

**依賴**: S11.2, S10.3, S10.4
**檔案**: `clients/admin_gui/views/actions.py`
**驗收指令**: 能執行 RESET 和 KICK 操作

**DoD**:
- [ ] 重設遊戲按鈕 (選擇房間 -> 確認對話框 -> RESET)
- [ ] 踢除玩家按鈕 (選擇玩家 -> 確認對話框 -> KICK)
- [ ] 操作結果顯示 (成功/失敗訊息)
- [ ] 右鍵選單支援 (在列表上右鍵執行操作)
- [ ] 操作後自動更新狀態

---

## Client Progress Summary

| EPIC | Total | Done | In Progress | TODO |
|------|-------|------|-------------|------|
| EPIC 4 - UDP Heartbeat | 1 | 1 | 0 | 0 |
| EPIC 5 - Clients (Core) | 4 | 4 | 0 | 0 |
| EPIC 7 - GUI Client | 4 | 4 | 0 | 0 |
| EPIC 8 - C++ Client | 4 | 4 | 0 | 0 |
| EPIC 9 - AI Strategy | 1 | 1 | 0 | 0 |
| EPIC 11 - Admin GUI | 4 | 0 | 0 | 4 |
| **Total** | **18** | **14** | **0** | **4** |

---

## Changelog

### 2026-01-14 (Code Review Fix by @Claude)
- **AI CLI 修正** (`clients/ai_cli/app.py`):
  - 新增 UDP Heartbeat 支援 (原本缺失)
  - 新增 PLAY_REJECT 處理邏輯
  - 新增 ROOM_WAIT, ROOM_START, PLAY_BROADCAST 訊息處理
  - 修正 TRICK_RESULT 分數更新邏輯
