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
   - **Meta** (left): the engine mark, `<Engine> · <role>` (for example `Claude Code · Lead`), the **model picker**, and, when the thread is running, a ghost **Pause** button (pause icon) or **Resume** button (play icon).
   - **Model picker** (`composer-model`): a ghost button showing the thread's model (`Default` until one is picked) that opens a menu of `Default` plus the engine's models. Claude lists Fable, Opus, Sonnet and Haiku (passed as `--model` aliases). ACP engines (Cursor, Codex, Gemini) list the models their session advertises as its `model` config option, so the picker appears once the session starts; an engine that advertises none shows no picker. The pick is saved per thread, used on every later spawn, and applied to a live engine straight away (Claude's `set_model` control request, ACP's `session/set_config_option`; mid-turn on ACP, it applies when the turn ends).
   - **Usage meter** (`composer-usage`): once the engine reports usage, a small mono line such as `22.3k · 11% · $1.23`: tokens in the latest turn's context (cached ones included), how full the context window is, and what the thread has cost so far. Parts the engine doesn't report are left out, and the tooltip says so (`cost not reported`) instead of showing zero. The thread tab's tooltip has the same sentence, or `Usage not reported`.
   - **Attach** (`composer-attach`, paperclip): opens the file picker for images and files. Pasting (`⌘V`) or dropping files on the composer attaches them too. Disabled while a plan waits for review, since a reply then is plan feedback.
- **Attachments** (`composer-attachments`), above the input once there are any. An image is a 64px thumbnail with no caption, its name in the tooltip (a pasted screenshot is named `pasted-<date-time>.png`) and a round x on its corner to remove it; images picked or dropped from Finder get their thumbnail from the core. Any other file is a chip with a file icon, its name and an x. Each pasted or browser-dropped file is limited to 20 MB.
- **Repo aliases:** typing `@` at the start of a word opens a **Repositories** picker of the repos set up in Cormux, each as `@<name>` (spaces become dashes) with its folder. Arrows move, `↵` or `⇥` picks, `Esc` closes. Picking inserts `@cormux ` in the draft. When the message is sent, each `@alias` of a configured repo is replaced by that repo's absolute path, so the agent can read or change another repo; emails (`me@example.com`), scoped packages (`@tauri-apps/api`) and unknown names stay as typed. Works for every engine, and the hint line shows `@ repos` once a repo is set up.
- Sending passes every attachment to the agent by path: picked and dropped files keep theirs, pasted ones are saved under the app's data folder (never the worktree). Images up to 5 MB also go to the model directly (Claude, and ACP engines that accept images). A message can be attachments alone. The conversation shows the message with `Attached: <names>` under it.
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

## Focus

The composer is where focus lives in a conversation, so typing always has somewhere to go.

- Opening a workspace on a thread, switching threads, opening a scratch, coming back from another tab (Plan, Git) or with `Ctrl+Tab`, and returning to the app with nothing focused all put focus in the input.
- Clicking a footer control (Plan, attach, New session, pause), picking a model, stopping the agent, and answering an approval card all leave focus in the input.
- Typing a character while focus has drifted somewhere that isn't a field (after clicking in the conversation, say) moves focus to the input and types it there.
- It never takes focus from another field, an open dialog, menu or list, or from tabs the user is moving through with the arrow keys.

