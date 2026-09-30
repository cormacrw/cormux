#!/usr/bin/env python3
import os
import time

os.setpgrp()
child = os.fork()
if child == 0:
    while True:
        time.sleep(1)

print("Local: http://127.0.0.1:8765/", flush=True)
time.sleep(120)
