# 12 · Composer

## Summary

The composer is the message box pinned to the bottom of every thread. The user types instructions to the selected thread's agent; Enter sends. It shows which engine and thread you're talking to and holds the thread's Pause/Resume control.

## Why it exists

Steering is the user's main job while agents work: clarifying scope, redirecting, answering questions. The composer is always in the same place, always addressed to the thread in front of you, and makes it obvious who will receive the message.

## Where it lives

- `<form class="composer" id="composer" data-od-id="composer">` inside `.composer-wrap` at the bottom of the thread panel.
- Hooks: `composer`, `composer-send`, `composer-pause`.

## Anatomy

1. **Textarea** (`#composer-input`), one row tall to start, placeholder `Message <role>…` (for example `Message Lead…`). Visually labelled only by its placeholder; has a screen-reader label `Message the agent`.
2. **Footer row:**
   - **Meta** (left): the engine mark, `<Engine> · <role>` (for example `Claude Code · Lead`), and, when the thread is running, a ghost **Pause** button (pause icon) or **Resume** button (play icon).
   - **Hint** (right of meta): `↵ send  ⇧↵ new line`.
   - **Send** button: a square primary button with an up-arrow icon, accessible name `Send to agent`.

## States

| State | Appearance |
| --- | --- |
| Empty | Send disabled. |
| Has text | Send enabled. |
| Thread running | Pause shown. |
| Thread paused | Resume shown. |
| Thread idle or provisioning | No Pause/Resume. |

## Behaviour

### Growing
The textarea grows with its content up to **200px**, then scrolls internally. It shrinks back after sending.

### Sending
- `↵` sends (unless an IME composition is in progress). `⇧↵` inserts a new line. Clicking Send also sends.
- Whitespace-only messages are ignored.
- On send:
  1. The input clears and shrinks; Send disables.
  2. The message is appended to the selected thread as a user bubble with time `now`, and the thread scrolls to the end.
  3. After 900ms the agent acknowledges:
     - Running: `Noted. I’ll factor that in before the next edit and flag anything it changes.`
     - Paused: `Noted. I’ll apply that as soon as you resume me.`

Messages go to the *selected thread only*, never to the whole workspace.

### Pause and Resume (interrupt and hold)
- **Pause** is not a true freeze mid-request. It **cancels the current turn** (ACP `session/cancel` or Claude interrupt), then holds the thread: status `paused`, step `You paused the agent` (pause icon), live row `Paused by you` / `Resume to let the agent continue.`, amber tab and sidebar dots, and the workspace stops counting the thread as working.
- Messages typed while paused are **queued** (not sent to the engine until resume).
- **Resume** sets the thread back to `running`, adds `You resumed the agent` (play icon), and sends the queued messages as the next turn (or `continue` if the queue is empty).
- Pausing unlocks the branch picker when no other thread is still running.
- A paused Reviewer delays finishing its review until resumed.

## Keyboard and accessibility

- The textarea has a proper (visually hidden) label.
- The hint is `aria-hidden` since the behaviour follows platform conventions.
- Send is a real submit button with an accessible name, disabled (not just inert) when empty.

## Data model

- Writes to the selected thread (`curAgent()`): `timeline`, `paused`.
- Reads `engine`, `role`, `status`, `paused`.

## Simulated in the prototype

- The agent's reply is a canned acknowledgement. A real build writes the message to the engine process's input (or its API) and streams the response back as conversation items.
- Pause is a flag. A real build must actually suspend the engine (signal the process or use the engine's interrupt), and define what happens to an in-flight tool call.

## Known gaps and open questions

- No attachments: files, images, screenshots, or links to lines in the diff.
- No `@` mentions for files, other threads or Skills.
- No slash commands (for example `/plan`, `/test`, `/review`).
- No way to interrupt and replace the agent's current step (as opposed to pausing).
- No message history (`↑` to recall).
- ~~Drafts aren't kept per thread~~ **Fixed:** each thread has its own composer draft when switching tabs.
- No "Stop" (end the task) distinct from Pause.
- Should the composer be available in idle threads that have finished? It is today, and that's probably right, but the reply copy assumes active work.
