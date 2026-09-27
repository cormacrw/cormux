# 15 · Run app and Output tab

## Summary

Each workspace can run its project (for example `pnpm dev`) straight from the worktree. Run, Restart and Stop live in a compact control in the workspace header, and the app's output (plus the worktree setup log) shows in an **Output** tab pinned to the right of the tab bar. Ports shift automatically when another workspace already holds one.

## Why it exists

To check an agent's work, the user needs to see it running, and parallel workspaces make running several copies of the same app routine. Harness knows each repo's run command, so running a workspace is one click, and the output is a tab rather than a separate terminal. It's deliberately *not* a terminal: there's no shell prompt, just the app's output.

## Where it lives

- **Header run controls:** `<div class="run-ctl" role="group" aria-label="App" data-od-id="ws-run-controls">`.
- **Output tab:** `#out-tabs`, pinned right in the tab bar (`thread-tab-output`).
- **Output panel:** `<section class="term" id="output-panel" role="tabpanel" data-od-id="output">` with header `#term-head` (`output-head`) and log `#term-body` (`output-log`).
- **Palette** App commands (see [02](02-command-palette-and-shortcuts.md)).
- The run command itself is configured per repo in Settings → Repos (see [22](22-repos.md)).

## Anatomy

### Header run controls

One joined, bordered control with two parts.

**Left part (Output toggle)**, content depends on app state:

| App state | Content |
| --- | --- |
| Stopped | terminal icon and `Output` |
| Starting | small spinner and `Starting…` |
| Running | green pulsing dot and `localhost:5173` in monospace |

Clicking it switches to the Output tab; clicking again goes back to the tab you came from. Tooltip: `Show output (⌃\`)` or `Back to thread (⌃\`)`. `aria-pressed` reflects whether Output is showing.

**Right part (actions):**

| App state | Buttons |
| --- | --- |
| Stopped | **Run** (play icon). Disabled while provisioning. Tooltip: `Run <command>`, `No run command set for <repo>`, or `Available once the worktree is set up`. |
| Starting | **Restart** (disabled) and **Stop** icon buttons |
| Running | **Restart** and **Stop** icon buttons. Stop turns red on hover. |

### Output tab

Terminal icon, `Output`, and a status mark: spinner while setting up or starting, a green dot and `:<port>` while running, nothing when stopped.

### Output panel header

Left to right:

1. **Status chip**: `Setting up` (spinner), `Starting` (spinner), `Running` (green dot, success chip), or `Stopped` (neutral).
2. **Command** in monospace (if the repo has one), truncated with a tooltip.
3. **URL link** while running: `localhost:5173` with an external-link icon, opening `http://localhost:5173/` in a new window.
4. Flexible space.
5. **Actions:**
   - Stopped: secondary **Run** (disabled while setting up).
   - Starting/Running: ghost **Restart** (disabled while starting) and ghost red **Stop**.
   - Always: ghost **Clear** (disabled when the log is empty).
   - On small screens the button labels collapse to icons only.

There's no close button and no "Terminal" title; it's just a tab.

### Output log

Monospace, 12.5px on 20px lines, wrapping long lines. Line styles:

| Style | Used for |
| --- | --- |
| `cmd` | A command being run (brighter). |
| `out` | Normal output. |
| `dim` | Low-importance output (`^C`, reloader info, HMR updates). |
| `ok` | Success lines (`Done in 6.4s`, `VITE v5.4.8 ready in 612 ms`). |
| `warn` | Port-in-use warnings. |
| `sep` | A centred rule with a label: `Worktree ready`, `App stopped`, `Restarting`. |
| `blank` | Spacer line. |

### Empty states

| Situation | Heading | Body | Action |
| --- | --- | --- | --- |
| Repo has no run command (and not provisioning) | `No run command for <repo>` | `Add the command that starts this project, like pnpm dev. Every workspace on <repo> will use it.` | Secondary **Add run command** (sliders icon). Opens Settings with that repo's config expanded and the cursor in its Run command field. |
| Has a run command but nothing logged | `Nothing running` | `Run starts <command> in this worktree. Its output shows up here.` | |

If there's setup output but the app is stopped, the setup log is shown with no empty state.

## Behaviour

### Run
1. Only works if the repo has a run command and the workspace has finished provisioning.
2. Harness picks the app type from the command:
   - contains `storybook` → Storybook, base port 6006
   - contains `uv`, `uvicorn`, `fastapi` or `python` → Python/Uvicorn, base port 8000
   - otherwise → Vite, base port 5173
3. **Port selection:** starting from the base port, skip any port held by another workspace's app that's starting or running. Each skipped port writes a warning line in that tool's style:
   - Vite: `Port 5173 is in use, trying another one...`
   - Storybook: `Port 6006 is not available, using the next free port`
   - Python: `ERROR:    [Errno 48] Address already in use (8000), trying the next port`
4. Status becomes **starting**. The command line, any warnings and the tool's boot lines stream into the log: first after 400ms, then one every 160ms. Vite example:
   ```
   > vite dev

     VITE v5.4.8  ready in 612 ms

     ➜  Local:   http://localhost:5173/
     ➜  Network: use --host to expose
   ```
5. When the last line lands, status becomes **running** and a toast says `<name> is running on localhost:<port>`.
6. **Run does not switch tabs.** You stay where you are; the Output tab's spinner, then dot and port, show progress. Focus moves to the Stop button (or to **Add run command** if there's no command).

### Stop
- Pending boot lines are cancelled, `^C` and a `App stopped` separator are written, status becomes **stopped**, the port is released, and a toast says `Stopped the app in <name>`. Focus moves to Run.

### Restart
- Only while running. Stops quietly (writes `^C`, no toast or "App stopped"), writes a `Restarting` separator, then runs again (possibly on a different port if another workspace grabbed it). Toast: `Restarting the app in <name>`, followed by the running toast when it's up.

### Clear
- Empties the log and moves focus to the log.

### Live output while running
Every 5 seconds, each running app has a 50% chance to write a line:
- **Python apps:** a request log, for example `INFO:     127.0.0.1:52431 - "GET /v1/invoices?limit=50 HTTP/1.1" 200 OK`.
- **Vite apps, only while an agent in the workspace is running:** a hot-reload line naming a file the agents touched, for example `12:41:07 PM [vite] (client) hmr update /src/lib/auth/session.ts`.

### Scrolling
The log follows new output while you're at the bottom (within 24px). If you scroll up, it stops following until you scroll back down. Switching to a different workspace's output resets to following.

### Log limit
The log keeps the last **500 lines**; older lines are dropped from the top.

### Switching tabs
- Header toggle and `Ctrl+\`` (no dialog open): go to Output, or back to the previous tab (Findings if you came from there and it still exists, otherwise the thread, scrolled to the end).
- Clicking the Output tab or arrowing onto it also remembers the previous tab.

### Across the app
- The Auth session timeout sample workspace launches with its app already running on 5173.
- Teardown stops the app and mentions it in the confirmation (see [08](08-teardown.md)).
- Setup commands stream into the same log during provisioning (see [07](07-worktree-provisioning.md)).

## Keyboard and accessibility

- The log is `role="log"`, labelled "App output", focusable for keyboard scrolling, with `aria-live="off"` so chatty output isn't read aloud.
- Every icon-only button has an `aria-label` (`Restart app`, `Stop app`, `Run app`).
- The header toggle's label includes state: `Output, app running on localhost:5173`.
- `Ctrl+\`` toggles; see [10](10-thread-tabs.md) for arrow-key tab navigation.

## Data model

Per workspace, `app` (created on first access):

```
status   'stopped' | 'starting' | 'running'
port     number | null
kind     'vite' | 'storybook' | 'python'
log      [[style, text], ...]   // max 500
ver      incrementing version, used to render incrementally
timers   pending timeouts (boot lines, setup lines)
```

Reads the repo's `run` and `setup`. `state.wsTab`, `state.lastTab`.

## Simulated in the prototype

All output is canned per app type. A real build must:

- Spawn the run command in the worktree with the user's login shell environment, as a process group so Stop kills children.
- Stream stdout/stderr with ANSI colour support.
- Detect the actual listening port (parse output, or inspect the process's sockets) rather than predicting it. Consider injecting `PORT` so each workspace gets a predictable port.
- Handle crashes: an exit with a non-zero code should show a `Crashed` state with the exit code and a Restart button. There's no crashed state today.
- Decide whether apps keep running when Harness quits.

## Known gaps and open questions

- **No crashed/exited state.**
- One run command per repo. Many projects need several processes (web, API, worker). Consider multiple named run targets per repo, each with its own Output tab.
- No input to the process (it's not a terminal); some dev servers accept keypresses (`r` to restart, `o` to open). Decide whether that matters.
- No per-workspace override of the run command or environment variables.
- No search or filter in the log; no copy-all or save.
- The URL always uses `localhost:<port>/`; apps served on a sub-path or HTTPS aren't handled.
- No in-app browser preview.
- Should an agent be able to read the Output log (for example to see a runtime error) or start/restart the app itself?
