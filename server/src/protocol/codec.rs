use super::messages::{ClientMessage, ErrorCode, ServerMessage};
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;

/// NDJSON Codec - 處理訊息的序列化與反序列化
pub struct Codec {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl Codec {
    /// 從 TcpStream 建立 Codec
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        let writer = stream.try_clone()?;
        let reader = BufReader::new(stream);
        Ok(Self { reader, writer })
    }

    /// 讀取一行並解析為 ClientMessage
    pub fn read_message(&mut self) -> io::Result<Option<ClientMessage>> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line)?;

        if bytes_read == 0 {
            // EOF - 連線關閉
            return Ok(None);
        }

        let line = line.trim();
        if line.is_empty() {
            // 空行，繼續讀取
            return self.read_message();
        }

        match serde_json::from_str(line) {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JSON parse error: {}", e),
            )),
        }
    }

    /// 發送 ServerMessage
    pub fn send_message(&mut self, msg: &ServerMessage) -> io::Result<()> {
        let json = serde_json::to_string(msg).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("JSON serialize error: {}", e))
        })?;
        writeln!(self.writer, "{}", json)?;
        self.writer.flush()?;
        Ok(())
    }

    /// 發送錯誤訊息
    pub fn send_error(&mut self, code: ErrorCode, message: impl Into<String>) -> io::Result<()> {
        let msg = ServerMessage::Error {
            code,
            message: message.into(),
        };
        self.send_message(&msg)
    }

    /// 取得 peer address
    pub fn peer_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.writer.peer_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_codec_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Server thread
        let server_handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut codec = Codec::new(stream).unwrap();

            // 讀取訊息
            let msg = codec.read_message().unwrap().unwrap();
            assert!(matches!(msg, ClientMessage::Ping));

            // 發送回應
            codec.send_message(&ServerMessage::Pong).unwrap();
        });

        // Client
        let client_stream = TcpStream::connect(addr).unwrap();
        let mut client_codec = Codec::new(client_stream).unwrap();

        // 發送 (模擬 client 直接寫入)
        writeln!(client_codec.writer, r#"{{"type":"PING"}}"#).unwrap();
        client_codec.writer.flush().unwrap();

        // 讀取回應 (直接讀 line)
        let mut response = String::new();
        client_codec.reader.read_line(&mut response).unwrap();
        assert!(response.contains("PONG"));

        server_handle.join().unwrap();
    }
}
