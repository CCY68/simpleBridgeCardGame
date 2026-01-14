use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::{SocketAddr, TcpListener};

const DEFAULT_BACKLOG: i32 = 128;

/// 使用 socket2 建立 TCP listener，展示 POSIX socket API 對應
pub fn create_tcp_listener(addr: SocketAddr) -> io::Result<TcpListener> {
    // socket() - 建立 socket
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;

    // setsockopt(SO_REUSEADDR) - 允許重複使用位址
    socket.set_reuse_address(true)?;

    // bind() - 綁定位址
    socket.bind(&addr.into())?;

    // listen() - 開始監聽，設定 backlog
    socket.listen(DEFAULT_BACKLOG)?;

    // 轉換為標準庫的 TcpListener
    let listener: TcpListener = socket.into();

    Ok(listener)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[test]
    fn test_create_listener() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let listener = create_tcp_listener(addr).expect("Failed to create listener");
        let local_addr = listener.local_addr().expect("Failed to get local addr");
        assert!(local_addr.port() > 0);
    }
}
