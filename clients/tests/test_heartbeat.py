import json
import time

from clients.common.heartbeat import HeartbeatClient


def test_handle_pong_updates_metrics():
    client = HeartbeatClient("127.0.0.1", 9999)
    start_ms = int(time.time() * 1000) - 50
    payload = json.dumps(
        {
            "type": "HB_PONG",
            "seq": 1,
            "t_client_ms": start_ms,
            "t_server_ms": start_ms + 5,
        }
    ).encode("utf-8")

    client._handle_pong(payload)
    metrics = client.get_metrics()

    assert metrics["received"] == 1
    assert metrics["rtt_ms"] >= 0.0
    assert metrics["avg_rtt_ms"] >= 0.0
