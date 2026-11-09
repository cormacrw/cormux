#!/usr/bin/env python3
"""NDJSON stand-in for `claude -p`, for the session and engine registry tests."""

import json
import sys


def emit(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


SESSION = "sess-1"
REPLAY = "--replay-user-messages" in sys.argv


def result(text):
    return {
        "type": "result",
        "subtype": "success",
        "session_id": SESSION,
        "is_error": False,
        "result": text,
    }


# A "slow" turn waits for the next prompt. Like the real CLI with no tool call left in it, it
# answers that turn first, then starts one of its own on the prompt.
slow_turn = False

for raw in sys.stdin:
    line = raw.strip()
    if not line:
        continue
    msg = json.loads(line)
    kind = msg.get("type")
    if kind == "user":
        text = msg["message"]["content"][0]["text"]
        if slow_turn:
            slow_turn = False
            emit(result("slow"))
            emit({"type": "system", "subtype": "init", "session_id": SESSION})
        if REPLAY:
            emit({**msg, "isReplay": True})
        if text.startswith("slow"):
            slow_turn = True
            emit(
                {
                    "type": "assistant",
                    "message": {"content": [{"type": "text", "text": "working"}]},
                }
            )
        elif text.startswith("first"):
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
        elif text.startswith("ask"):
            emit(
                {
                    "type": "control_request",
                    "request_id": "req_ask",
                    "request": {
                        "subtype": "can_use_tool",
                        "tool_name": "AskUserQuestion",
                        "input": {
                            "questions": [
                                {
                                    "question": "Which colour?",
                                    "header": "Colour",
                                    "options": [
                                        {"label": "Red", "description": "Warm"},
                                        {"label": "Blue", "description": "Cool"},
                                    ],
                                    "multiSelect": False,
                                }
                            ]
                        },
                        "tool_use_id": "toolu_ask",
                    },
                }
            )
        elif text.startswith("background"):
            # Shaped like the real CLI: the turn's result lands first, then the finished
            # agent starts a turn of its own.
            agent = {"task_id": "agent-1", "task_type": "local_agent"}
            emit({"type": "system", "subtype": "background_tasks_changed", "tasks": [agent]})
            result = {
                "type": "result",
                "subtype": "success",
                "session_id": SESSION,
                "is_error": False,
                "result": "launched",
            }
            emit(result)
            emit({"type": "system", "subtype": "background_tasks_changed", "tasks": []})
            emit({"type": "system", "subtype": "init", "session_id": SESSION})
            emit(result)
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
        # Echo the decision so tests can see what the host sent back.
        emit(
            {
                "type": "assistant",
                "message": {
                    "content": [
                        {"type": "text", "text": json.dumps(msg["response"]["response"])}
                    ]
                },
            }
        )
        emit(
            {
                "type": "result",
                "subtype": "success",
                "session_id": SESSION,
                "is_error": False,
                "result": "ok",
            }
        )
    elif kind == "control_request" and msg.get("request", {}).get("subtype") == "initialize":
        emit(
            {
                "type": "control_response",
                "response": {"subtype": "success", "request_id": msg["request_id"]},
            }
        )
        emit({"type": "system", "subtype": "init", "session_id": SESSION})
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
