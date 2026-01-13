# POSIX Socket API Mapping

> 本文件展示 C/C++ POSIX Socket API 與 Rust (socket2 + std::net) 的對應關係。
> 這是本專案的核心教學目標之一。

---

## 1. Overview

我們使用 Rust 的 `socket2` crate 來精確對應 POSIX Socket API，這讓我們可以：

- 直接控制 `listen(backlog)` 參數
- 設定各種 socket options (`SO_REUSEADDR`, `TCP_NODELAY` 等)
- 展示 socket lifecycle 的每一步

`std::net` 則用於實際的 I/O 操作，因為 `socket2::Socket` 可以轉換為 `std::net::TcpStream`。

---

## 2. TCP Server Lifecycle

### 2.1 Create Socket

| C/C++ (POSIX) | Rust (socket2) |
|---------------|----------------|
| `int fd = socket(AF_INET, SOCK_STREAM, 0);` | `let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;` |

```rust
use socket2::{Socket, Domain, Type, Protocol};

let socket = Socket::new(
    Domain::IPV4,      // AF_INET
    Type::STREAM,      // SOCK_STREAM
    Some(Protocol::TCP)
)?;
```

### 2.2 Set Socket Options

| C/C++ (POSIX) | Rust (socket2) |
|---------------|----------------|
| `setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));` | `socket.set_reuse_address(true)?;` |
| `setsockopt(fd, SOL_SOCKET, SO_REUSEPORT, &opt, sizeof(opt));` | `socket.set_reuse_port(true)?;` |
| `setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &opt, sizeof(opt));` | `socket.set_nodelay(true)?;` |
| `setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));` | `socket.set_read_timeout(Some(Duration::from_secs(30)))?;` |

```rust
// 允許 port 重用 (server 重啟時避免 "Address already in use")
socket.set_reuse_address(true)?;

// Linux 下允許多個 socket bind 同一 port
#[cfg(target_os = "linux")]
socket.set_reuse_port(true)?;
```

### 2.3 Bind

| C/C++ (POSIX) | Rust (socket2) |
|---------------|----------------|
| `bind(fd, (struct sockaddr*)&addr, sizeof(addr));` | `socket.bind(&addr.into())?;` |

```rust
use std::net::SocketAddr;

let addr: SocketAddr = "0.0.0.0:8888".parse()?;
socket.bind(&addr.into())?;
```

### 2.4 Listen

| C/C++ (POSIX) | Rust (socket2) |
|---------------|----------------|
| `listen(fd, 128);` | `socket.listen(128)?;` |

```rust
// backlog = 128，決定 pending connection queue 大小
socket.listen(128)?;

// 注意: std::net::TcpListener 沒有 backlog 參數
// 這是我們選擇 socket2 的主要原因之一
```

### 2.5 Accept

| C/C++ (POSIX) | Rust (socket2) |
|---------------|----------------|
| `int client_fd = accept(fd, (struct sockaddr*)&client_addr, &len);` | `let (client_socket, client_addr) = socket.accept()?;` |

```rust
loop {
    let (client_socket, client_addr) = socket.accept()?;

    // 轉換為 std::net::TcpStream 以便使用標準 I/O trait
    let stream: TcpStream = client_socket.into();

    // 為每個連線 spawn 一個 thread
    std::thread::spawn(move || {
        handle_client(stream, client_addr);
    });
}
```

---

## 3. TCP I/O Operations

### 3.1 Send/Write

| C/C++ (POSIX) | Rust (std::net) |
|---------------|-----------------|
| `send(fd, buf, len, 0);` | `stream.write(buf)?;` |
| `write(fd, buf, len);` (loop until all sent) | `stream.write_all(buf)?;` |

```rust
use std::io::Write;

// write_all 會自動 loop 直到所有 bytes 都送出
stream.write_all(b"Hello\n")?;
stream.flush()?;
```

### 3.2 Recv/Read

| C/C++ (POSIX) | Rust (std::net) |
|---------------|-----------------|
| `recv(fd, buf, len, 0);` | `stream.read(buf)?;` |
| 逐行讀取 (需自己實作 buffer) | `BufReader::read_line(&mut line)?;` |

```rust
use std::io::{BufRead, BufReader};

let reader = BufReader::new(&stream);
for line in reader.lines() {
    let line = line?;
    // 每行是一個 NDJSON message
    let msg: Message = serde_json::from_str(&line)?;
}
```

---

## 4. UDP Operations

### 4.1 Create & Bind

| C/C++ (POSIX) | Rust |
|---------------|------|
| `socket(AF_INET, SOCK_DGRAM, 0);` | `Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;` |
| - | `UdpSocket::bind("0.0.0.0:8889")?;` (std::net 簡化版) |

```rust
// 使用 socket2 (完整控制)
let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
socket.bind(&addr.into())?;
let udp_socket: UdpSocket = socket.into();

// 或使用 std::net (簡化版)
let udp_socket = UdpSocket::bind("0.0.0.0:8889")?;
```

### 4.2 Send/Receive

| C/C++ (POSIX) | Rust (std::net) |
|---------------|-----------------|
| `sendto(fd, buf, len, 0, &dest_addr, sizeof(dest_addr));` | `socket.send_to(buf, dest_addr)?;` |
| `recvfrom(fd, buf, len, 0, &src_addr, &len);` | `let (len, src_addr) = socket.recv_from(&mut buf)?;` |

```rust
// Server 端接收 PING
let mut buf = [0u8; 1024];
let (len, src_addr) = udp_socket.recv_from(&mut buf)?;

// 回覆 PONG
let pong = format!(r#"{{"type":"HB_PONG","seq":{},"t_client_ms":{},"t_server_ms":{}}}"#,
    seq, t_client, now());
udp_socket.send_to(pong.as_bytes(), src_addr)?;
```

---

## 5. Non-blocking & Timeout

### 5.1 Set Non-blocking

| C/C++ (POSIX) | Rust |
|---------------|------|
| `fcntl(fd, F_SETFL, O_NONBLOCK);` | `socket.set_nonblocking(true)?;` |

### 5.2 Set Timeout

| C/C++ (POSIX) | Rust |
|---------------|------|
| `setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, ...);` | `socket.set_read_timeout(Some(Duration::from_secs(30)))?;` |
| `setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, ...);` | `socket.set_write_timeout(Some(Duration::from_secs(30)))?;` |

---

## 6. Multi-threading Model

我們使用 `std::thread` + `std::sync::mpsc` 實作 thread-per-connection 模型：

| C/C++ (POSIX) | Rust (std) |
|---------------|------------|
| `pthread_create()` | `std::thread::spawn()` |
| `pthread_mutex_t` | `std::sync::Mutex<T>` |
| `pthread_cond_t` | `std::sync::Condvar` |
| pipe/socketpair (IPC) | `std::sync::mpsc::channel()` |

```rust
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

// 建立 channel
let (tx, rx): (Sender<GameEvent>, Receiver<GameEvent>) = channel();

// 每個 client handler 持有 tx clone
let tx_clone = tx.clone();
thread::spawn(move || {
    // 讀取 client 訊息後送到 game loop
    tx_clone.send(GameEvent::PlayerMove { player_id, card }).unwrap();
});

// Game loop 從 rx 接收事件
thread::spawn(move || {
    for event in rx {
        match event {
            GameEvent::PlayerMove { player_id, card } => { /* 處理出牌 */ }
            GameEvent::PlayerDisconnect { player_id } => { /* 處理斷線 */ }
        }
    }
});
```

---

## 7. What About select/poll/epoll?

| C/C++ | Rust |
|-------|------|
| `select()` | 無直接對應 |
| `poll()` | 無直接對應 |
| `epoll` (Linux) | `mio` crate (可選) |

**本專案選擇不使用 epoll/mio**，原因：
1. Thread-per-connection 對 4 個玩家的規模完全足夠
2. 程式碼更容易理解和解釋
3. 展示經典的 blocking I/O 模型

若要使用 event-driven 模型，可參考 `mio` 或 `tokio`（作為 Bonus 項目）。

---

## 8. Summary Table

| Operation | C/C++ `<sys/socket.h>` | Rust `socket2` | Rust `std::net` |
|-----------|------------------------|----------------|-----------------|
| Create TCP socket | `socket(AF_INET, SOCK_STREAM, 0)` | `Socket::new(IPV4, STREAM, TCP)` | `TcpListener::bind()` (間接) |
| Set SO_REUSEADDR | `setsockopt(..., SO_REUSEADDR, ...)` | `set_reuse_address(true)` | N/A |
| Bind | `bind(fd, &addr, len)` | `socket.bind(&addr)` | (內建於 TcpListener::bind) |
| Listen | `listen(fd, backlog)` | `socket.listen(backlog)` | N/A (無法控制 backlog) |
| Accept | `accept(fd, &addr, &len)` | `socket.accept()` | `listener.accept()` |
| Read | `recv(fd, buf, len, 0)` | - | `stream.read(buf)` |
| Write | `send(fd, buf, len, 0)` | - | `stream.write(buf)` |
| Close | `close(fd)` | `drop(socket)` | `drop(stream)` |
| Create UDP socket | `socket(AF_INET, SOCK_DGRAM, 0)` | `Socket::new(IPV4, DGRAM, UDP)` | `UdpSocket::bind()` |
| Send UDP | `sendto(fd, buf, len, 0, &addr, len)` | - | `socket.send_to(buf, addr)` |
| Recv UDP | `recvfrom(fd, buf, len, 0, &addr, &len)` | - | `socket.recv_from(buf)` |
| Non-blocking | `fcntl(fd, F_SETFL, O_NONBLOCK)` | `set_nonblocking(true)` | `set_nonblocking(true)` |
