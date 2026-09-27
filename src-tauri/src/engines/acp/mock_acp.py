#!/usr/bin/env python3
"""Minimal ACP agent for COR-43: permission round trip + session/load."""

import json
import sys

session_id = "sess-acp-1"
perm_req_id = 9000
pending_prompt_id = None


def send(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


for raw in sys.stdin:
    line = raw.strip()
    if not line:
        continue
    msg = json.loads(line)
    method = msg.get("method")
    req_id = msg.get("id")

    if method == "initialize":
        send(
            {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": 1,
                    "agentCapabilities": {"loadSession": True},
                },
            }
        )
    elif method == "session/new":
        send({"jsonrpc": "2.0", "id": req_id, "result": {"sessionId": session_id}})
    elif method == "session/prompt":
        pending_prompt_id = req_id
        send(
            {
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": {
                    "sessionId": session_id,
                    "update": {
                        "sessionUpdate": "agent_message_chunk",
                        "content": {"type": "text", "text": "working"},
                    },
                },
            }
        )
        send(
            {
                "jsonrpc": "2.0",
                "id": perm_req_id,
                "method": "session/request_permission",
                "params": {
                    "sessionId": session_id,
                    "toolCall": {
                        "toolCallId": "call-1",
                        "title": "Run echo",
                    },
                    "options": [
                        {
                            "optionId": "allow-once",
                            "name": "Allow once",
                            "kind": "allow_once",
                        },
                        {
                            "optionId": "reject-once",
                            "name": "Reject once",
                            "kind": "reject_once",
                        },
                    ],
                },
            }
        )
    elif method == "session/cancel":
        if pending_prompt_id is not None:
            send(
                {
                    "jsonrpc": "2.0",
                    "id": pending_prompt_id,
                    "result": {"stopReason": "cancelled"},
                }
            )
            pending_prompt_id = None
    elif method == "session/load":
        send({"jsonrpc": "2.0", "id": req_id, "result": {}})
    elif pending_prompt_id is not None and "result" in msg and msg.get("id") == perm_req_id:
        send(
            {
                "jsonrpc": "2.0",
                "id": pending_prompt_id,
                "result": {"stopReason": "end_turn"},
            }
        )
        pending_prompt_id = None
