#!/usr/bin/env python3
"""Bounded OpenAI-compatible provider for the ForgeOS AGENT-100 witness."""

from __future__ import annotations

import argparse
import http.server
import json
from typing import Any


class Handler(http.server.BaseHTTPRequestHandler):
    server_version = "ForgeOSRemoteAgentFixture/1.0"
    protocol_version = "HTTP/1.1"

    def log_message(self, format: str, *args: object) -> None:
        del format, args

    def write_json(self, status: int, payload: dict[str, Any]) -> None:
        raw = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/v1/models":
            self.write_json(
                200,
                {
                    "object": "list",
                    "data": [
                        {
                            "id": "fixture.complete.v1",
                            "object": "model",
                            "owned_by": "forgeos-fixture",
                        }
                    ],
                },
            )
            return
        self.write_json(404, {"error": {"code": "not_found"}})

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/v1/chat/completions":
            self.write_json(404, {"error": {"code": "not_found"}})
            return
        length = int(self.headers.get("content-length", "0"))
        try:
            body = json.loads(self.rfile.read(length).decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError):
            self.write_json(400, {"error": {"code": "invalid_json"}})
            return
        messages = body.get("messages", [])
        prompt = ""
        if isinstance(messages, list) and messages:
            last = messages[-1]
            if isinstance(last, dict) and isinstance(last.get("content"), str):
                prompt = last["content"]
        self.write_json(
            200,
            {
                "id": "forgeos-remote-agent-fixture-response",
                "object": "chat.completion",
                "created": 0,
                "model": body.get("model", "fixture.complete.v1"),
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": f"fixture-provider:{prompt}",
                        },
                        "finish_reason": "stop",
                    }
                ],
                "usage": {
                    "prompt_tokens": 11,
                    "completion_tokens": 7,
                    "total_tokens": 18,
                },
                "nyx_cost": {
                    "currency": "USD",
                    "monetary_microunits": 1234,
                    "energy_millijoules": 55,
                    "memory_bytes": 4096,
                    "device_use": ["cpu"],
                },
            },
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18089)
    args = parser.parse_args()
    server = http.server.ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"FORGE_REMOTE_AGENT_FIXTURE=http://{args.host}:{args.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
