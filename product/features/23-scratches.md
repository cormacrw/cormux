# 23 · Scratches

## Summary

A scratch is a titled, one-off conversation against a single repo. It has no worktree and no branch. On Homebase, scratches sit in a row of thin cards above the workspace cards. Opening a card shows that scratch on its own page: a title, the repo path, one conversation, and a composer. The prompt is optional. Starting with a prompt gets a short read-only answer. Starting blank opens an empty conversation with the cursor in the message box.

The word **scratch** is the user-facing name. The prototype's code still says `session` (`kind: 'session'`, ids like `sess-webhook`, view `view-session`). Those identifiers are internal. Visible copy says scratch.

## Why it exists

Some questions do not deserve a branch. "Why does this webhook fail?" or "draft a changelog, don't publish it" should not provision a worktree, pick an engine per thread, or show Changes, Output, and Create PR. A scratch is the short trip: one title, one repo, one conversation, thrown away with End or Delete.

It sits next to workspaces on purpose. **New scratch** is a secondary button beside **New Workspace**, so the heavier path stays the primary action and the lighter path is one shortcut away (`⌘S`).

## Where it lives

### Homebase

- Section `<section data-od-id="home-sessions">`, first section under the Homebase header, above Workspaces and Open pull requests.
- **New scratch** is not in that section. It is in the header action row, immediately left of **New Workspace**.

### Scratch page

- `<section class="view" id="view-session" hidden aria-label="Scratch" data-od-id="session-view">`.
- There is no back button. Sidebar **Homebase** (and palette **Go to Homebase**) is how you leave.

### Dialogs

- New scratch: `<dialog id="session-dialog">`.
- End and Delete reuse the shared confirm dialog (`#confirm-dialog`).

### Entry points

| Entry | What it does |
| --- | --- |
| **New scratch** on Homebase | Opens the dialog. Hook `new-session`. |
| `⌘S` / `Ctrl+S` | Opens the dialog from anywhere, including while a field is focused. |
| Palette **New scratch** | Opens the dialog. Key hint `⌘S`. |
| A scratch card | Opens that scratch. Hook `session-card-<id>`. |
| Palette **Open &lt;title&gt;** | Opens that scratch. Group **Scratches**. |
| Trash button on a card | Asks to delete it. Hook `session-delete-<id>`. |
| **End scratch** on the scratch page | Asks to end it. Hook `session-end`. |

## Anatomy

### Homebase header

The action row, left to right:

1. **New scratch**, secondary button, plus icon, label **New scratch**, key hint `⌘S`.
2. **New Workspace**, the only solid primary button on Homebase, key hint `⌘N`.

### Scratches section

- Title **Scratches** with a count pill of `state.sessions.length`, including zero.
- No filter. The workspace Running / Idle filter does not apply here.
- **With scratches:** a grid, `gap` of 8px, columns `repeat(auto-fill, minmax(min(100%, 320px), 1fr))`. Cards are one line tall (`min-height: 52px`), not the tall workspace cards (those start at 380px wide and stack title, meta, summary, and actions).
- **With none:** the grid class is removed and a single dashed box reads `No scratches. Start one for a question that doesn’t need its own branch.`

### Scratch card

An `<article class="sess-card">`. Inside, two controls side by side. The delete button is not inside the open button, so activating the card does not delete it.

1. **Open button** (`.sess-open`), the rest of the row:
   - **Title** in the regular sans at the small size, weight 500, single line, ellipsis if it overflows.
   - **Repo id** under the title, in monospace, muted, single line, ellipsis. This is the repo's id (`my-app`), not its folder path.
   - **Status badge** on the right: a dot plus one of **Working**, **Idle**, **Paused**, **Needs attention**. See States.
   - Accessible name: `<title>, <repo id>, <status>`.
2. **Delete button**, 44×44px, trash icon, `title="Delete scratch"`, accessible name `Delete <title>`.

Hover (fine pointer only) strengthens the card border and fills it with the raised surface. Pressing the open button fills the card with the next surface step. A card that was just added plays the same fade-and-rise as a new workspace card, once. Later re-renders do not replay it (`_shown`).

### New scratch dialog

A small dialog (`dialog-sm`, max 480px). Title **New scratch**. No description line. Close (×) on the right.

Fields, top to bottom:

1. **Title**
   - Single-line text input, `maxlength="64"`, placeholder `Why the webhook signature fails`.
   - Hint: `Shown on the card and at the top of the scratch`.
   - Error, hidden until submit with an empty title: `Give this scratch a title.` The hint hides while the error shows, so the two never stack.
2. **Repository**
   - Select of every repo from Settings, option label and value both the repo id.
   - Hint: `The agent reads this repo. No worktree or branch is created.`
   - Preselected: `my-app` if that repo still exists, otherwise the first repo.
3. **Prompt**
   - Textarea, 4 rows, same field chrome as Initial Prompt on New workspace.
   - Placeholder: `Ask something that doesn’t need its own branch`.
   - Hint: `Optional. Leave it blank and write the first message in the scratch.`
   - Not required. There is no error state for this field.

Footer:

- Left: `Esc to cancel`.
- **Cancel** (ghost) and **Start scratch** (primary) with a `⌘↵` hint. **Start scratch** is the only solid button.

### Scratch page

Full height under the app sidebar. Two rows: header, then the conversation.

**Header** (`.ws-top`, hook `session-header`):

- **Title** as `h1` (`#sess-heading`). Unlike a workspace name, it wraps (`text-wrap: pretty`) instead of truncating to one line. It is focused when the page opens, except when the scratch was started with an empty prompt (then the composer is focused).
- **Repo path** under the title, monospace, muted, one line with ellipsis. The `title` attribute holds the full path, for example `~/code/my-app`. If the repo was removed from Settings, the id is shown instead.
- Right side: the same status badge as the card, then a ghost **End scratch** button.

Not on this page: thread tabs, a `+` for new threads, a branch picker, Changes, Output, Run / Restart / Stop, Create PR, Pull, Rebase, or Teardown.

**Conversation** (hook `session-thread`):

The same renderer as a workspace thread ([11](11-agent-conversation.md)), with two scratch-specific differences:

- The speaker line is the engine name only (`Claude Code`), with its two-letter mark. A workspace thread also shows the role (`Lead`).
- The screen-reader live region (`#sess-announce`) announces the newest agent item, not the whole thread. See Keyboard and accessibility.

The column is the same centered reading width as a workspace thread. User messages are right-aligned bubbles. Agent text is plain. Tool rows group into one compact card. The sample tool rows carry the chip `Read-only, no worktree`.

**Composer** (hook `session-composer`), pinned to the bottom:

- Visually hidden label: `Message this scratch`.
- Textarea, one row to start, grows with the text up to 200px.
- Placeholder `Write the first message…` until the timeline contains a user message, then `Ask a follow-up…`.
- Foot: engine mark and engine name; **Pause** or **Resume** only while `status === 'running'`; hint `↵ send` and `⇧↵ new line`; send button (arrow up, accessible name **Send**, hook `session-send`).
- Send is disabled while the box is empty or whitespace.

## States

`sessionStatus` picks the badge in this order. The first match wins.

| Condition | Badge | Tone |
| --- | --- | --- |
| Any pending approval in the timeline | **Needs attention** | Amber (`st-paused`, the same amber class as Paused) |
| `status === 'running'` and not paused | **Working** | Green, pulsing dot |
| `paused` | **Paused** | Amber |
| Anything else | **Idle** | Grey |

The prototype never creates an approval on a scratch, so **Needs attention** is reachable only if a timeline item of `kind: 'approval'` with `state: 'pending'` is added later. The check is live.

`status` itself is only `running` or `idle`. There is no `provisioning` state. `activity` is stored (`Reading the repo…`, `Drafting the entries…`, `Answered`, `Idle`) and drives the live row's title while a run is on screen. The card and the header badge do not show `activity`.

**Pause** is offered only while `status === 'running'`. An idle scratch has no Pause button. Pausing a running scratch keeps `status` as `running` and sets `paused`, so the badge becomes **Paused** and the live row title becomes `Paused by you` with the line `Resume to let the agent continue.`

## Behaviour

### Opening the dialog

`openSessionDialog`:

1. Clears the "user edited the title" flag.
2. Clears Title and Prompt. Does not touch a previous error until the next line.
3. Hides the title error, shows the hint, clears `aria-invalid`.
4. Rebuilds the Repository menu from `REPOS` and selects `my-app` or the first repo.
5. `showModal()`, then focuses Title.

Opening does not check whether the dialog is already open. Pressing `⌘S` while it is open resets the fields and calls `showModal()` again.

### Drafting the title from the prompt

On each Prompt input, if the user has not edited Title:

- An empty prompt clears Title.
- A non-empty prompt sets Title to `nameFor(prompt)`, the same helper New workspace uses for the workspace name: strip trailing spaces and full stops, take whole words up to 40 characters, capitalise the first character, leave the rest as typed.
- Example: `draft a short changelog for september` becomes `Draft a short changelog for september`.

Editing Title sets the flag, including back to empty. The flag is "the field is currently non-empty", so clearing Title hands it back to the draft on the *next* Prompt keystroke. Clearing Title does not immediately refill from the prompt that is already there.

A drafted or typed title that is non-empty clears the error.

The 64-character `maxlength` applies to typing. `nameFor` stops at 40, so a draft never hits the cap. A hand-typed title can be 64.

### Submitting the dialog

Triggered by **Start scratch**, by Enter in Title (it is a single-line input in the form), or by `⌘↵` / `Ctrl+↵` in Prompt. Plain Enter in Prompt inserts a newline.

1. Title is trimmed. If it is empty, the error shows, the hint hides, `aria-invalid` is set, and focus returns to Title. Prompt is not checked.
2. The dialog closes immediately. There is no spinner and no `Creating…` label. Unlike New workspace, nothing is provisioned.
3. `startSession(title, repoId, prompt)` runs.

### Starting a scratch

A new object is inserted at the front of `state.sessions` (newest first) and the app navigates to it. Toast, success tone: `Started <title>`.

The engine is `state.settings.defaultEngine` (Claude Code unless Settings changed it). The dialog does not ask.

**With a prompt** (`createdMin: 0`, `status: 'running'`, `activity: 'Reading the repo…'`):

Timeline, in order:

1. User message, the prompt, time `now`.
2. Tool row: title `Opened <repo id>`, detail the repo path (or the id if the repo is gone), chip `Read-only, no worktree`, folder icon, time `now`.
3. A live row. Its title is the activity. Its subtitle cycles `Finding the files this question names` and `Reading them in place`.

Focus stays on the page title (set by `openSession`).

After 1400ms, if the scratch still exists and is not paused:

- The live row is removed.
- `status` becomes `idle`, `activity` becomes `Answered`.
- An agent message is appended: `I read <path> in place. Nothing was checked out and nothing was edited. Ask a follow-up here, or start a workspace if this should land on a branch.`

If it is paused when the timer fires, the finish retries every 800ms until the scratch is resumed or deleted. The answer is not written while paused.

**Without a prompt** (`status: 'idle'`, `activity: 'Idle'`, timeline `[]`):

The page opens empty. After the title is focused, focus moves to the composer. Placeholder is `Write the first message…`. No timer runs. No tool row is written until the user sends a message, and a later message uses the follow-up reply, not the "I read … in place" reply.

### Follow-ups

Submit is the send button or Enter in the composer. Shift+Enter inserts a newline. IME composition Enter is ignored. Empty and whitespace-only submits do nothing, and the button is already disabled.

1. The trimmed text is appended as a user message at `now`, inserted before a live row if one exists.
2. The box is cleared and shrunk.
3. After 900ms, if the scratch still exists, an agent message is appended:
   - Paused: `Noted. I’ll pick that up when you resume me.`
   - Otherwise: `Noted. I’ll answer from this repo, and I won’t check out a branch.`

This timer does not set `status` to `running`, does not add a live row, and does not wait out a pause the way the first answer does. A follow-up sent while a first-run is paused still arrives after 900ms, using the paused sentence. Several follow-ups in a row each schedule their own reply.

The composer draft is one shared textarea for every scratch. Switching scratches does not save or clear it.

### Pause and resume

While `status === 'running'`, the composer shows **Pause** (pause icon) or **Resume** (play icon).

Toggling writes a tool row, `You paused the agent` or `You resumed the agent`, and flips `paused`. The live subtitle keeps cycling only while running and not paused (every 3200ms, in `liveSteps` order, wrapping). A full re-render replaces the live subtitle from `liveIdx`.

The same `toggle-pause` action serves the workspace composer. On a scratch page it targets the scratch. On a workspace it targets the current thread.

### Opening an existing scratch

`openSession`:

1. Sets `view` to `session` and `sessionId`.
2. Shows `#view-session` and hides Homebase, the workspace, and Settings.
3. Document title becomes `Harness · <scratch title>`.
4. Scrolls the thread to the bottom (`tlToEnd`).
5. Focuses the title.

Existing messages do not replay their entrance animation. Only items added after this page was already showing the same scratch animate in. If the last row is the live row, the row just above it animates with it.

Scroll position is kept across re-renders except when `tlToEnd` is set (open, or a new item pushed while this scratch is on screen).

### Delete from Homebase

Trash runs `endSession(id, true)`.

Confirm dialog:

- Title: `Delete this scratch?`
- Body: `Discards the conversation for <title>. <path> is not changed.` The path is in `<code>`.
- Confirm button: danger **Delete scratch**. While the 900ms confirm timer runs, the button is disabled and reads `Deleting…` with a spinner.
- Cancel, Esc, or the backdrop dismisses with no change.

On confirm:

1. The scratch is removed from `state.sessions`.
2. Toast, default tone: `Deleted <title>`.
3. If that scratch was the open page, the app goes to Homebase and focuses the Homebase title.
4. If Homebase was already showing, it re-renders and focuses the open button of the card that slid into that index, or **New scratch** if the list is empty.

The repo, its files, and its branches are not modified. There is no "also delete" checkbox.

### End from the scratch page

**End scratch** runs `endSession(id)` with the home flag unset.

Same body. Title: `End this scratch?` Confirm label: **End scratch**. Busy label: `Ending…`. Toast: `Ended <title>`.

Because this button only exists on the open scratch, confirm always then calls `goHome()` (the open `sessionId` matches). Focus lands on the Homebase title. The card-focus step does not run.

End and Delete remove the same record. The two labels exist so the card action reads as delete and the page action reads as leaving the conversation.

### While a scratch is running

Every 3200ms, each running, unpaused scratch advances `liveIdx` and, if its live subtitle is in the DOM, updates that node without re-rendering the page.

Every 4000ms the sidebar memory figure counts unpaused running scratches the same as unpaused running agents: base 0.9 GB plus 0.24 GB each, plus a small random term, capped at a full bar of 4 GB.

Every 60s, `createdMin` increments on every scratch. If Homebase or a scratch page is showing, it re-renders. Cards do not display `createdMin`. The sample timestamps (`26m`, `4m`, `2h`) are the `t` strings on timeline items, not `createdMin`, and they do not tick.

### Scratch macros

A macro is a saved name and prompt, managed in Settings › Scratch macros ([21](21-settings.md)). Each macro with a name and a prompt appears in the palette under **Macros** (right after Actions), labelled with its name, lightning icon, and a muted second line with the prompt's first 20 characters (whitespace collapsed, `…` when cut).

Typing a macro's full name (any case) then Space or Tab turns it into a chip, like the TODO chip ([24](24-todos.md)). Whatever is typed after the chip is appended to the saved prompt after a blank line. The list shows one row, `Start <name>`, with the preview plus `+ “<text>”`. Enter starts it; the footer hint reads `start scratch`. Backspace on an empty field turns the chip back into the name. Enter on the bare name (no chip) runs the macro as saved.

Running one starts a scratch titled with the macro's name, in the Default repository, with the prompt as its first message. It does **not** navigate: you stay on whatever view you were on. A success toast `Started <name>` confirms it, and clicking the toast opens the scratch. Errors toast `Could not start the scratch`.

## Rules and edge cases

- **Title is the only required field.** Repo always has a selection because Settings will not remove the last repo. Prompt may be blank.
- **Titles are not unique.** Two scratches may share a title. The card, the confirm copy, and the palette label all use the title string, so they look the same. Ids differ (`sess-` plus a base36 timestamp).
- **No branch, no worktree, no setup commands.** Repo setup lines and the run command are not executed. The scratch page has no Output tab.
- **No engine picker.** New scratches take the Settings default. The three samples are hardcoded (`claude`, `claude`, `cursor`) and do not follow a later default change.
- **Removing a repo does not remove its scratches.** The card still shows the id. The page falls back to the id when `repoById` misses. The hint on the card is the id either way.
- **`⌘S` is global.** It fires with no dialog guard, including while New scratch, New workspace, confirm, or the palette is open, and while typing in the composer. It always `preventDefault`, so the browser save dialog does not appear while the prototype is focused. Shift+S and Alt+S are not bound.
- **`⌘S` and `⌘N` do not collide.** `⌘N` still opens New workspace. The earlier `⌘⇧N` binding is gone.
- **Palette will not open** while New scratch, New workspace, or confirm is open. `⌘K` in those dialogs is still swallowed by the palette shortcut's `preventDefault` before that guard, same as `⌘N`.
- **Needs attention uses the Paused color class.** The words differ. The amber treatment does not.
- **A follow-up during the first read does not replace the planned answer.** Both timers can fire. The first-read answer still arrives at 1400ms (or after resume). The follow-up answer arrives at 900ms.
- **Deleting during a timer** makes the timer return at the `getSession` check. No message is added to a removed scratch.
- **Confirm's delete-branch checkbox** belongs to Teardown. Scratch confirm replaces the dialog body and does not include it.

## Copy

### Chrome

| Surface | String |
| --- | --- |
| Header button | `New scratch` |
| Section title | `Scratches` |
| Empty section | `No scratches. Start one for a question that doesn’t need its own branch.` |
| Card delete tooltip | `Delete scratch` |
| Card delete accessible name | `Delete <title>` |
| Dialog title | `New scratch` |
| Title label | `Title` |
| Title placeholder | `Why the webhook signature fails` |
| Title hint | `Shown on the card and at the top of the scratch` |
| Title error | `Give this scratch a title.` |
| Repo label | `Repository` |
| Repo hint | `The agent reads this repo. No worktree or branch is created.` |
| Prompt label | `Prompt` |
| Prompt placeholder | `Ask something that doesn’t need its own branch` |
| Prompt hint | `Optional. Leave it blank and write the first message in the scratch.` |
| Dialog footer | `Esc to cancel` |
| Dialog buttons | `Cancel`, `Start scratch` |
| Page accessible name | `Scratch` |
| End button | `End scratch` |
| Composer label | `Message this scratch` |
| Composer placeholder, empty thread | `Write the first message…` |
| Composer placeholder, after a user message | `Ask a follow-up…` |
| Composer hint | `↵ send` `⇧↵ new line` |
| Send accessible name | `Send` |
| Pause / Resume | `Pause`, `Resume` |
| Badges | `Working`, `Idle`, `Paused`, `Needs attention` |
| Tool chip | `Read-only, no worktree` |
| Document title | `Harness · <scratch title>` |

### Confirm

| From | Title | Confirm | Busy | Toast |
| --- | --- | --- | --- | --- |
| Homebase trash | `Delete this scratch?` | `Delete scratch` | `Deleting…` | `Deleted <title>` |
| Scratch page | `End this scratch?` | `End scratch` | `Ending…` | `Ended <title>` |

Shared body: `Discards the conversation for <title>. <path> is not changed.`

Start toast, success tone: `Started <title>`.

### Generated thread copy

| When | Text |
| --- | --- |
| Prompted start, tool row | `Opened <repo id>` / detail `<path>` |
| Prompted start, after 1400ms | `I read <path> in place. Nothing was checked out and nothing was edited. Ask a follow-up here, or start a workspace if this should land on a branch.` |
| Follow-up, not paused | `Noted. I’ll answer from this repo, and I won’t check out a branch.` |
| Follow-up, paused | `Noted. I’ll pick that up when you resume me.` |
| Pause tool row | `You paused the agent` |
| Resume tool row | `You resumed the agent` |
| Live subtitle while paused | `Resume to let the agent continue.` |
| First-run live subtitles | `Finding the files this question names`, then `Reading them in place` |

### Palette

| Group | Label | Meta | Key hint |
| --- | --- | --- | --- |
| Actions | `New scratch` | | `⌘S` |
| Scratches | `Open <title>` | repo id | |

The Scratches group is omitted when there are no scratches. It is rendered after Workspaces and before App.

### Sample scratches

These three ship in `state.sessions`, in this order. They are placeholder copy, same as the sample workspaces.

**Why the webhook signature fails** (`sess-webhook`, `my-app`, Claude Code, idle, `createdMin` 26)

- User, `26m`: `Stripe webhooks are failing signature checks on /api/stripe/webhook. Why?`
- Tool, `26m`: `Read 2 files` / `src/routes/api/stripe/webhook/+server.ts, src/lib/stripe.ts` / chip `Read-only, no worktree`
- Agent, `25m`: `The handler calls request.json() before constructEvent. Stripe signs the raw body, and parsing it then serializing it again changes the bytes, so the signature never matches. Verify against request.text() first. I didn’t edit anything.`

**Draft the September changelog** (`sess-changelog`, `marketing-site`, Claude Code, running, `createdMin` 4)

- User, `4m`: `Draft a short changelog for what shipped in September. Don’t publish it.`
- Tool, `4m`: `Read the changelog and recent commits` / `marketing-site, on the current checkout` / chip `Read-only, no worktree`
- Live, activity `Drafting the entries…`, cycling `Reading the September commits`, `Grouping them by what shipped`, `Writing the draft in the scratch`

This one is still running when the app boots, so Homebase shows **Working** and the page shows Pause.

**Where sessions are stored** (`sess-redis`, `my-app-api`, Cursor CLI, idle, `createdMin` 90)

- User, `2h`: `Where does the API keep session records?`
- Tool, `2h`: `Read the auth client` / `app/auth/sessions.py` / chip `Read-only, no worktree`
- Agent, `1h`: `In Redis, keyed as sess:<id>, written by the web app. This API only reads them. The TTL is 30 minutes and nothing refreshes it on read, which is why active users still get signed out.`

The word "session" in that third scratch is about login sessions in the sample repo. It is not the scratch feature. The workspace **Auth session timeout** is also unchanged.

## Keyboard and accessibility

| Key | Where | Effect |
| --- | --- | --- |
| `⌘S` / `Ctrl+S` | Anywhere, no Shift or Alt | Open the New scratch dialog and reset it. Browser save is suppressed. |
| `⌘↵` / `Ctrl+↵` | Prompt field | Submit the dialog. |
| `↵` | Title field | Submit the dialog (native single-line submit). |
| `↵` | Prompt field | New line. |
| `Esc`, ×, backdrop, **Cancel** | Dialog | Close. Native dialog cancel. |
| `↵` | Composer, not composing, no Shift | Send. |
| `⇧↵` | Composer | New line. |
| `⌘K` | New scratch dialog open | Prevented, and the palette stays closed. |

- The dialog is named by `#session-dlg-title`.
- Title's `aria-describedby` points at both the hint and the error, including while the error is hidden.
- Invalid title sets `aria-invalid="true"`.
- Prompt's `aria-describedby` points at its hint.
- The scratch page's conversation list is `aria-label="Conversation"`.
- `#sess-announce` is `aria-live="polite"`. It updates only when the timeline grows while this scratch is already rendered, and only for the newest item that is not `live` and not `user`:
  - thought: `<engine name>: <text>`
  - tool or edit: `<engine name>: <title or path>`
  - anything else: `<engine name> updated the scratch`
- The speaker line above agent text is `aria-hidden`. The live region is the announcement.
- The open control's accessible name includes title, repo id, and status, so the badge text is not required for the name.
- Focus rings use the app's `:focus-visible` ring. The open button and the delete button draw it inset (`outline-offset: -2px`) so it stays inside the card.
- After a re-render, if focus was on an element with `data-action` inside the scratch page (or Homebase) and that node was replaced, focus returns to the matching action and arg via `preventScroll`.
- Opening a scratch moves focus to the title, which has `tabindex="-1"` and no visible focus outline (same as the workspace and Homebase titles). A blank start then moves focus to the composer.
- Delete from Homebase moves focus to the next card or to **New scratch**. End from the page moves focus to the Homebase title.
- Reduced motion cuts the new-card animation with the rest of the app.

The sidebar does not list scratches, and **Homebase** is not `aria-current="page"` while a scratch is open. Nothing in the sidebar is current on that page.

## Data model

`state.sessions[]`, newest first. `state.sessionId` is the open scratch, or `null` on every other view. `state.view === 'session'` selects the page.

```
id           'sess-' + base36 timestamp, or a sample id
kind         'session'          // internal; UI says scratch
title        string, required, drafted up to 40 chars, typed up to 64
repo         repo id
engine       'claude' | 'cursor' | 'codex' | 'gemini'
status       'running' | 'idle'
activity     string shown as the live-row title
paused       boolean
createdMin   number, ticks every 60s, not displayed
liveSteps    string[]
liveIdx      number
files        always []
timeline     same item kinds as a workspace thread
_shown       set true the first time the card renders, so the entrance animation plays once
```

Reads: `REPOS` (id, path), `state.settings.defaultEngine`, `ENGINES`.

Does not read or write: branches, worktrees, `behind`, app/terminal state, findings, PRs, or the workspace list.

Local UI flag `sessNameEdited` lives outside the scratch object and resets each time the dialog opens.

## Simulated in the prototype

- **No process reads the repo.** The tool rows and the 1400ms answer are fixed strings. A real build would run the chosen engine against the repo's current checkout, read-only, with no `git worktree add` and no run of the repo's setup commands.
- **Follow-ups are one canned sentence**, chosen by the paused flag, after 900ms. A real build sends the message to that scratch's engine session and streams the reply.
- **Pause** only flips a flag and a timeline row. A real build signals the engine to stop generating and to resume the same session.
- **Live subtitles** rotate on a 3200ms timer. A real build would show the engine's actual current step.
- **The memory meter** adds 0.24 GB per running scratch. That number is decorative.
- **Sample threads** are authored. They must not be presented as real agent output.

## Known gaps and open questions

- **A scratch cannot become a workspace.** The first-run answer tells the user to start a workspace if the work should land on a branch. There is no "Start workspace from this scratch" action that would carry the title, repo, and transcript.
- **The engine cannot be chosen or changed** on the dialog or the page. New scratches always take the Settings default.
- **The title and repo cannot be edited** after creation.
- **The composer draft is global.** A half-written message survives switching scratches and is what Send will submit on the scratch you switched to. The workspace composer has the same leak across threads.
- **Follow-ups ignore the first-run pause gate.** They reply after 900ms even while the first answer is waiting for Resume, and they never show a live row of their own.
- **`⌘S` while the dialog is open** clears the form and calls `showModal()` on an already-open dialog.
- **`⌘S` while typing** always opens a new scratch. There is no "save" and no way to dismiss the shortcut inside a field.
- **Needs attention and Paused share a color.** Only the label distinguishes them, and nothing in the current scratch flow creates an approval.
- **Sidebar and memory.** Scratches are absent from the sidebar list and from the Agents list. They do count toward the memory figure. Homebase is not marked current while one is open.
- **Removing a repo** leaves its scratches in place with a dangling id.
- **No search** over scratch transcripts. The palette matches the title and the repo id only.
- **`createdMin` is unused in the UI.** Cards do not show age. Timeline times are static strings.
- **End and Delete are the same operation** with different verbs. There is no archive, and no undo after the 900ms confirm.

