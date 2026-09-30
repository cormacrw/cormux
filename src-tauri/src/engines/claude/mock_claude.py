#!/usr/bin/env python3
"""NDJSON stand-in for `claude -p` used by the COR-42 spawn test."""

import json
import sys


def emit(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


SESSION = "sess-1"
emit({"type": "system", "subtype": "init", "session_id": SESSION})

for raw in sys.stdin:
    line = raw.strip()
    if not line:
        continue
    msg = json.loads(line)
    kind = msg.get("type")
    if kind == "user":
        text = msg["message"]["content"][0]["text"]
        if text.startswith("first"):
            emit(
                {
                    "type": "control_request",
                    "request_id": "req_bash",
                    "request": {
                        "subtype": "can_use_tool",
                        "tool_name": "Bash",
                        "input": {"command": "echo spike-ok"},
                        "tool_use_id": "toolu_1",
                    },
                }
            )
        elif text.startswith("second"):
            emit(
                {
                    "type": "assistant",
                    "message": {"content": [{"type": "text", "text": "counting"}]},
                }
            )
        else:
            emit(
                {
                    "type": "result",
                    "subtype": "success",
                    "session_id": SESSION,
                    "is_error": False,
                    "result": "resumed",
                }
            )
    elif kind == "control_response":
        emit(
            {
                "type": "result",
                "subtype": "success",
                "session_id": SESSION,
                "is_error": False,
                "result": "ok",
            }
        )
    elif kind == "control_request" and msg.get("request", {}).get("subtype") == "interrupt":
        emit(
            {
                "type": "control_response",
                "response": {"subtype": "success", "request_id": msg["request_id"]},
            }
        )
        emit(
            {
                "type": "result",
                "subtype": "error",
                "is_error": True,
                "session_id": SESSION,
            }
        )
