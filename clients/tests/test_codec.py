import json
import socket

from clients.common.codec import NDJsonCodec


def test_codec_send_adds_newline():
    left, right = socket.socketpair()
    try:
        codec = NDJsonCodec(left)
        codec.send({"type": "PING"})

        data = right.recv(1024)
        assert data.endswith(b"\n")
        payload = json.loads(data.decode("utf-8").strip())
        assert payload["type"] == "PING"
    finally:
        left.close()
        right.close()


def test_codec_recv_parses_multiple_messages():
    left, right = socket.socketpair()
    try:
        codec = NDJsonCodec(left)
        right.sendall(b'{"type":"PING"}\n{"type":"PONG"}\n')

        first = codec.recv()
        second = codec.recv()

        assert first == {"type": "PING"}
        assert second == {"type": "PONG"}
    finally:
        left.close()
        right.close()


def test_codec_recv_invalid_json_returns_none():
    left, right = socket.socketpair()
    try:
        codec = NDJsonCodec(left)
        right.sendall(b"{invalid json\n")

        assert codec.recv() is None
    finally:
        left.close()
        right.close()
