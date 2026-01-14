# CardArena 使用說明書

本文件提供一般使用者啟動與操作 CardArena 的步驟，適用於 LAN 環境。

---

## 1. 環境需求

- Rust toolchain (stable)
- Python 3.10+
- 端口預設：TCP 8888、UDP 8889

---

## 2. 啟動 Server

在專案根目錄執行：

```bash
cd server && cargo run
```

預期看到 server 綁定訊息，例如：

```
Listening on 0.0.0.0:8888
```

---

## 3. 啟動 Human CLI

在專案根目錄執行：

```bash
python clients/human_cli/app.py --host 127.0.0.1 --port 8888 --name Player1
```

參數說明：
- `--host`：Server IP
- `--port`：Server TCP port
- `--name`：玩家暱稱 (1-16 字元)

---

## 4. 啟動 AI CLI

AI client 需要 auth token，Server 端使用環境變數 `AI_AUTH_TOKEN` 驗證。

先設定 Server 端環境變數：

```bash
export AI_AUTH_TOKEN=secret
```

再啟動 AI Client：

```bash
python clients/ai_cli/app.py --host 127.0.0.1 --port 8888 --name Bot1 --token secret
```

參數說明：
- `--token`：必須與 server 的 `AI_AUTH_TOKEN` 相同

---

## 5. 啟動 Human GUI (Tkinter)

```bash
python clients/human_gui/app.py
```

GUI 內填寫：
- IP
- Port
- Nickname

點擊 Connect 後進入遊戲畫面。

---

## 6. Heartbeat 顯示

Client 會用 UDP 心跳監控 RTT / loss rate：
- UDP port = TCP port + 1（預設 8889）
- CLI 顯示 RTT、loss rate

---

## 7. 基本遊戲流程

1. 連線後會收到 WELCOME
2. 未滿 4 人時收到 ROOM_WAIT
3. 滿 4 人自動開始，收到 ROOM_START
4. Server 發 DEAL (手牌)
5. 輪流 YOUR_TURN → PLAY
6. 每圈 TRICK_RESULT
7. 13 圈後 GAME_OVER

---

## 8. 常見問題

**Q: AI 連不上？**  
A: 檢查 Server 是否設定 `AI_AUTH_TOKEN`，以及 client `--token` 是否一致。

**Q: GUI 沒反應？**  
A: 確認 Tkinter 有安裝，且 server 已啟動。

