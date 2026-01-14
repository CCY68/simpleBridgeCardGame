import socket
import threading
import time
import json
import select
from typing import Optional, Dict

class HeartbeatClient:
    """
    UDP Heartbeat Client for monitoring RTT and packet loss.
    Protocol:
        Client -> Server: {"type": "HB_PING", "seq": N, "t_client_ms": ...}
        Server -> Client: {"type": "HB_PONG", "seq": N, "t_client_ms": ..., "t_server_ms": ...}
    """

    def __init__(self, server_ip: str, server_udp_port: int, interval: float = 1.0):
        self.server_addr = (server_ip, server_udp_port)
        self.interval = interval
        self.sock: Optional[socket.socket] = None
        self.running = False
        self._thread: Optional[threading.Thread] = None
        
        # Stats
        self.seq = 0
        self.sent_count = 0
        self.recv_count = 0
        self.last_rtt = 0.0
        self.avg_rtt = 0.0
        self._lock = threading.Lock()

    def start(self):
        """Start the heartbeat loop in a background thread."""
        if self.running:
            return
        
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            self.sock.setblocking(False)  # Non-blocking for select()
            self.running = True
            self._thread = threading.Thread(target=self._loop, daemon=True)
            self._thread.start()
            print(f"[Heartbeat] Started monitoring {self.server_addr}")
        except Exception as e:
            print(f"[Heartbeat] Failed to start: {e}")

    def stop(self):
        """Stop the heartbeat loop."""
        self.running = False
        if self._thread:
            self._thread.join(timeout=1.0)
        if self.sock:
            self.sock.close()
            self.sock = None

    def get_metrics(self) -> Dict[str, float]:
        """Return current network metrics."""
        with self._lock:
            loss_rate = 0.0
            if self.sent_count > 0:
                loss_rate = 1.0 - (self.recv_count / self.sent_count)
            
            return {
                "rtt_ms": round(self.last_rtt, 2),
                "avg_rtt_ms": round(self.avg_rtt, 2),
                "loss_rate": round(loss_rate * 100, 1), # Percentage
                "sent": self.sent_count,
                "received": self.recv_count
            }

    def _loop(self):
        next_ping_time = time.time()

        while self.running and self.sock:
            now = time.time()
            
            # 1. Send PING if it's time
            if now >= next_ping_time:
                self._send_ping()
                next_ping_time = now + self.interval
            
            # 2. Calculate timeout for select()
            # Wait only until next ping is due, or max 0.1s to allow checking 'running' flag
            wait_time = min(next_ping_time - time.time(), 0.1)
            if wait_time < 0:
                wait_time = 0

            # 3. Wait for PONG
            try:
                ready = select.select([self.sock], [], [], wait_time)
                if ready[0]:
                    data, _ = self.sock.recvfrom(1024)
                    self._handle_pong(data)
            except OSError:
                break # Socket closed
            except Exception as e:
                print(f"[Heartbeat] Error in loop: {e}")

    def _send_ping(self):
        try:
            self.seq += 1
            msg = {
                "type": "HB_PING",
                "seq": self.seq,
                "t_client_ms": int(time.time() * 1000)
            }
            payload = json.dumps(msg).encode('utf-8')
            self.sock.sendto(payload, self.server_addr)
            
            with self._lock:
                self.sent_count += 1
                
        except Exception as e:
            print(f"[Heartbeat] Send error: {e}")

    def _handle_pong(self, data: bytes):
        try:
            msg = json.loads(data.decode('utf-8'))
            if msg.get("type") != "HB_PONG":
                return
            
            t_client_sent = msg.get("t_client_ms", 0)
            seq = msg.get("seq", 0)
            
            # Basic validation
            # Note: We don't strictly check seq match because UDP can reorder,
            # but we assume the latest one is good enough for RTT.
            # A more robust impl might track pending seqs.
            
            now_ms = time.time() * 1000
            rtt = now_ms - t_client_sent
            
            with self._lock:
                self.recv_count += 1
                self.last_rtt = rtt
                
                # Simple moving average
                if self.avg_rtt == 0:
                    self.avg_rtt = rtt
                else:
                    self.avg_rtt = 0.7 * self.avg_rtt + 0.3 * rtt
                    
        except json.JSONDecodeError:
            pass
        except Exception as e:
            print(f"[Heartbeat] Process error: {e}")
