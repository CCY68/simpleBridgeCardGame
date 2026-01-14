import json
import socket
from typing import Optional, Any

class NDJsonCodec:
    """NDJSON encoder/decoder for TCP socket."""

    def __init__(self, sock: socket.socket):
        self.sock = sock
        self.buffer = b""

    def send(self, msg: Any) -> None:
        """將 dict 編碼為 JSON 並加上換行符號送出。"""
        data = json.dumps(msg).encode('utf-8') + b'\n'
        self.sock.sendall(data)

    def recv(self) -> Optional[dict]:
        """從 socket 讀取資料並解析出一個完整的 JSON 物件。"""
        try:
            while b'\n' not in self.buffer:
                data = self.sock.recv(4096)
                if not data:
                    return None
                self.buffer += data
            
            line, self.buffer = self.buffer.split(b'\n', 1)
            return json.loads(line.decode('utf-8'))
        except (socket.error, json.JSONDecodeError):
            return None
