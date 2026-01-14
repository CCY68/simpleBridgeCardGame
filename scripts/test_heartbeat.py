import socket
import threading
import json
import time
import sys
import os

# Add project root to path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from clients.common.heartbeat import HeartbeatClient

HOST = '127.0.0.1'
PORT = 8889

def mock_udp_server(stop_event):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((HOST, PORT))
    sock.settimeout(1.0)
    print(f"[MockServer] UDP Listening on {HOST}:{PORT}")

    while not stop_event.is_set():
        try:
            data, addr = sock.recvfrom(1024)
            msg = json.loads(data.decode())
            
            if msg.get("type") == "HB_PING":
                # print(f"[MockServer] Received PING seq={msg['seq']}")
                
                # Simulate server processing time? Nah, let's just reply
                reply = {
                    "type": "HB_PONG",
                    "seq": msg['seq'],
                    "t_client_ms": msg['t_client_ms'],
                    "t_server_ms": int(time.time() * 1000)
                }
                sock.sendto(json.dumps(reply).encode(), addr)
        except socket.timeout:
            continue
        except Exception as e:
            print(f"[MockServer] Error: {e}")
    
    sock.close()
    print("[MockServer] Stopped")

def main():
    stop_event = threading.Event()
    server_thread = threading.Thread(target=mock_udp_server, args=(stop_event,), daemon=True)
    server_thread.start()
    
    # Give server time to start
    time.sleep(1)
    
    print("\n[Test] Starting HeartbeatClient...")
    client = HeartbeatClient(HOST, PORT, interval=0.5) # Fast interval for testing
    client.start()
    
    try:
        for i in range(6):
            time.sleep(1)
            metrics = client.get_metrics()
            print(f"[Test] T={{i+1}}s Metrics: {metrics}")
            
            # Simple assertions
            if i > 2:
                 assert metrics['sent'] > 0, "Should have sent packets"
                 assert metrics['received'] > 0, "Should have received packets"
                 # assert metrics['loss_rate'] < 10.0, "Loss rate should be low on localhost" 
                 # (First few might be lost if server not ready, but usually fine)

    except KeyboardInterrupt:
        pass
    finally:
        print("\n[Test] Stopping...")
        client.stop()
        stop_event.set()
        server_thread.join()
        print("[Test] Done.")

if __name__ == "__main__":
    main()
