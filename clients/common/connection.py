import socket
import threading
import queue
from typing import Optional, Dict, Any
from .codec import NDJsonCodec

class NetworkClient:
    """處理 TCP 連線與背景接收訊息的類別。"""

    def __init__(self):
        self.sock: Optional[socket.socket] = None
        self.codec: Optional[NDJsonCodec] = None
        self.msg_queue = queue.Queue()
        self.running = False
        self._recv_thread: Optional[threading.Thread] = None

    def connect(self, host: str, port: int) -> bool:
        """建立連線並啟動接收執行緒。"""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((host, port))
            self.codec = NDJsonCodec(self.sock)
            self.running = True
            
            self._recv_thread = threading.Thread(target=self._receive_loop, daemon=True)
            self._recv_thread.start()
            return True
        except Exception as e:
            print(f"Connection failed: {e}")
            return False

    def _receive_loop(self):
        """背景執行緒：不斷讀取封包並放入 Queue。"""
        while self.running and self.codec:
            msg = self.codec.recv()
            if msg is None:
                self.running = False
                break
            self.msg_queue.put(msg)
        
        if self.sock:
            self.sock.close()

    def send(self, msg: Dict[str, Any]):
        """送出訊息。"""
        if self.codec and self.running:
            self.codec.send(msg)

    def get_message(self) -> Optional[Dict[str, Any]]:
        """從 Queue 中取得一則訊息（非阻塞）。"""
        try:
            return self.msg_queue.get_nowait()
        except queue.Empty:
            return None

    def close(self):
        """關閉連線。"""
        self.running = False
        if self.sock:
            self.sock.close()
