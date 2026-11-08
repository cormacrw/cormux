"""A stand-in for `gh` in the stack tests.

Everything lives in $FAKE_GH_DIR. Each call is appended to `calls.jsonl` as its argv and
its GIT_CONFIG_* env. `replies.json` maps a `gh stack` subcommand to the reply for it:
{"code", "stdout", "stderr", "run"}, where `run` is an argv to run in the cwd first,
standing in for what gh-stack does to the worktree.
"""

import json
import os
import subprocess
import sys

root = os.environ["FAKE_GH_DIR"]
args = sys.argv[1:]

call = {
    "args": args,
    "env": {k: v for k, v in os.environ.items() if k.startswith("GIT_CONFIG_")},
}
with open(os.path.join(root, "calls.jsonl"), "a") as log:
    log.write(json.dumps(call) + "\n")

with open(os.path.join(root, "replies.json")) as file:
    replies = json.load(file)
subcommand = args[1] if len(args) > 1 and args[0] == "stack" else None
reply = replies.get(subcommand)
if reply is None:
    sys.stderr.write(f"fake gh: no reply for {args}\n")
    sys.exit(97)

if reply.get("run"):
    subprocess.run(reply["run"], check=True)
sys.stdout.write(reply.get("stdout", ""))
sys.stderr.write(reply.get("stderr", ""))
sys.exit(reply.get("code", 0))
