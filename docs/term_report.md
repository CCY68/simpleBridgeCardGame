# CardArena Socket Programming Report

## Repository
- GitHub: https://github.com/CCY68/simpleBridgeCardGame

## NDJSON 通訊說明
- 遊戲流程以 TCP NDJSON 交換訊息驅動
- 每行一個 JSON 物件，`\n` 作為 frame boundary
- Client/Server 皆使用 NDJSON codec 進行編解碼

## 範圍與重點
- 範圍: Server (Rust) + Client (Python CLI, C++ CLI) 的 socket 程式碼
- AI 玩家: 改由 Server side 執行，Client 僅負責連線與出牌
- 遊戲簡述: 四人回合制牌局，NoKing 規則

## Server (Rust) Socket 程式碼
- TCP listener: `server/src/net/listener.rs`
  - `create_tcp_listener()` 以 `socket2` 完成 `socket/bind/listen` 後轉為 `TcpListener`
- 連線處理: `server/src/net/handler.rs`
  - `ConnectionHandler::run()` 讀取 NDJSON、傳遞事件、回送訊息
  - `spawn_handler()` 為每條 TCP 連線建立 thread
- NDJSON codec: `server/src/protocol/codec.rs`
  - `read_message()` 以 `BufRead::read_line()` 取一行 JSON
  - `send_message()` 序列化後加換行傳送
- UDP heartbeat: `server/src/net/heartbeat.rs`
  - `create_udp_socket()` 綁定 UDP port
  - `handle_ping()` 回覆 `HB_PONG`

## Client (Python CLI) Socket 程式碼
- TCP 連線與接收執行緒: `clients/common/connection.py`
  - `NetworkClient.connect()` 建立 `socket` 並啟動接收 thread
  - `_receive_loop()` 持續讀取 NDJSON 並進 queue
- NDJSON codec: `clients/common/codec.py`
  - `send()` 送出 JSON + `\n`
  - `recv()` 以 buffer 等到 `\n` 後解析
- UDP heartbeat: `clients/common/heartbeat.py`
  - `HeartbeatClient.start()` 建立 UDP socket 並在背景 thread 送/收 ping/pong

## Client (C++ CLI) Socket 程式碼
- 平台抽象: `clients/cpp_cli/src/net/socket_wrapper.hpp`
  - Windows/Linux 的 `socket_t`、`CLOSE_SOCKET`、初始化與清理
- TCP 連線與收發: `clients/cpp_cli/src/net/tcp_client.cpp`
  - `connect_to()` 建立 socket、`connect()`、開啟接收 thread
  - `send_message()` NDJSON framing（補 `\n`）
  - `receive_loop()` `recv()` 後以 `\n` 拆分訊息
- UDP heartbeat: `clients/cpp_cli/src/net/udp_heartbeat.cpp`
  - `start()` 建立 UDP socket 並啟動 send/recv thread
  - `send_loop()` 送出 `HB_PING`
  - `recv_loop()` 接收 `HB_PONG` 計算 RTT

## Rust / C++ / Python Socket 功能對照表

| 功能 | Rust (Server) | C++ (Client) | Python (Client) |
| --- | --- | --- | --- |
| 建立 TCP socket | `socket2::Socket::new()` `server/src/net/listener.rs` | `socket()` `clients/cpp_cli/src/net/tcp_client.cpp` | `socket.socket()` `clients/common/connection.py` |
| bind/listen | `bind()` + `listen()` `server/src/net/listener.rs` | N/A (client) | N/A (client) |
| accept | `TcpListener::accept()` 於主 loop | N/A (client) | N/A (client) |
| connect | N/A (server) | `connect()` `clients/cpp_cli/src/net/tcp_client.cpp` | `sock.connect()` `clients/common/connection.py` |
| send | `Codec::send_message()` `server/src/protocol/codec.rs` | `send()` `clients/cpp_cli/src/net/tcp_client.cpp` | `sendall()` `clients/common/codec.py` |
| recv | `Codec::read_message()` `server/src/protocol/codec.rs` | `recv()` `clients/cpp_cli/src/net/tcp_client.cpp` | `sock.recv()` `clients/common/codec.py` |
| NDJSON framing | `read_line()` + `\n` | `\n` split + append | buffer + `\n` split |
| UDP heartbeat | `UdpSocket::bind()` `server/src/net/heartbeat.rs` | `socket(AF_INET, SOCK_DGRAM)` `clients/cpp_cli/src/net/udp_heartbeat.cpp` | `socket.AF_INET, SOCK_DGRAM` `clients/common/heartbeat.py` |
