# 11 · Agent conversation

## Summary

The agent conversation is the main surface of a workspace. It shows everything a thread has done, in order: the user's messages, the agent's reasoning, grouped runs of tool calls and file edits, plans, approval requests, review summaries and a live indicator of what the agent is doing right now. It sits in a centred reading column with the composer pinned underneath.

## Why it exists

Principle 1: **The conversation is the product.** The user supervises by reading what the agent is doing and why, answering its questions and approving risky steps. Code, diffs and output are available on demand but don't take over.

## Where it lives

- `<section class="thread" id="thread-panel" role="tabpanel" data-od-id="thread">`.
- Scroll container `#thread-scroll`, reading column `.thread-col` (about 760px wide), intro `#thread-intro`, message list `<ol id="timeline" aria-label="Conversation">`.
- Hooks: `thread-intro`, `approval-<id>`, `findings-card`, `findings-card-open`.

## Anatomy

### Thread intro (top of the conversation)

- The engine's two-letter mark in a small tile.
- The thread's role as a heading, for example `Lead`.
- A subtitle: `<Engine> · <relationship>`, where relationship is:
  - Lead with other threads: `leads <N other threads> in this worktree`
  - Lead alone: `owns this worktree`
  - Secondary thread: `shares the worktree with Lead`
- **Lead thread only:** the workspace summary block (same as the Homebase card), for example `Summarized by Haiku 4.5, 2m ago`.

The old status badge in the intro was removed; the tab's dot carries status.

### Speaker headers

When the speaker changes from the user to the agent, a small header row appears: the engine mark, the role in bold, the engine name, and the time (`· 8m`). Consecutive agent items share one header.

### Message types

| Kind | Rendering |
| --- | --- |
| **User message** | Right-aligned bubble. Under it: `You · <time>`. |
| **Agent thought** | Plain paragraph text, no bubble. |
| **Tool call / edit run** | Consecutive tool calls and edits are grouped into one compact card (a list of steps). See below. |
| **Plan** | Card headed `Proposed plan` with a numbered list of steps. |
| **Approval** | Card with a warning icon, title, a `Needs approval` chip, what and why, and two buttons. See [13](13-approvals.md). |
| **Review findings** | Card headed `Review findings` showing counts per severity and an **Open findings** button. See [20](20-review-findings.md). |
| **Live row** | Last item while the agent is working: a spinner (or pause icon), a title and a rotating subtitle. |

### Steps inside a tool run

Each step row: an icon, a title, an optional detail line, optional chips, and a time on the right.

- **Tool steps**: icon from the tool (file, tool, branch, check-circle, terminal, database, pr, download, list, pause, play). Success steps have a green tone. Examples:
  - `Read 6 files` · `src/lib/auth/**, src/hooks.server.ts` · chip `Read-only, auto-approved`
  - `Parsed AST` · `src/lib/auth/session.ts` · chip `3 call sites of getSession()`
  - `Ran tests: 12 passed` · `vitest · src/lib/auth` · chips `12 passed`, `0 failed`, `1.8s`
- **Edit steps**: pencil icon (trash for deletions). Title `Edited` / `Created` / `Deleted` followed by the file name as a link button, then `+N −N`. Detail is the full path. Clicking the file name opens the Changes panel on that file's diff.

### Live row

- Title: the thread's activity (for example `Analyzing AST…`), `Paused by you` when paused, or the provisioning activity.
- Subtitle:
  - Paused: `Resume to let the agent continue.`
  - Provisioning: `Setting up the worktree` (Lead) or `Joining the worktree` (other threads).
  - Running: one of the thread's live steps, rotating every **3.2 seconds**. Example for the auth Lead: `Walking getSession() call sites` → `Checking cookie refresh in hooks.server.ts` → `Looking for other TTL constants`.
- Icon: spinner, or a pause icon when paused.

## Behaviour

### Scrolling
- Opening a workspace, switching threads and sending a message scroll to the end.
- New items pushed to the thread you're viewing scroll to the end.
- Other re-renders (approvals elsewhere, ticks) preserve your scroll position.

### Animation
Only newly added items fade in. Re-rendering the thread doesn't replay animations on existing items. When the live row is last, the item inserted just before it animates.

### Screen reader announcements
A hidden polite live region (`#thread-announce`) announces only the newest agent item when the viewed thread grows:

| New item | Announcement |
| --- | --- |
| Thought | `<role>: <text>` |
| Edit | `<role> edited <path>` |
| Approval | `<role> needs approval: <title>, <what>` |
| Findings | `Review findings are ready` |
| Plan | `<role> proposed a plan` |
| Tool | `<role>: <title>` |

User messages are not announced.

## Sample conversations

- **Auth session timeout, Lead** (Claude Code): created worktree → reasoning about idle timeouts → read 6 files → parsed AST → edited three files → ran 12 passing tests → reasoning about the legacy cookie helper → two approvals (Delete file, Run database migration) → live.
- **Auth session timeout, Test writer**: user asks for boundary tests → reads 2 files → edits the test file → reasoning → live.
- **Auth session timeout, Reviewer** (Cursor CLI, idle): reads the Lead's diff → two notes → approval **Apply review suggestions** (Send to Lead / Dismiss).
- **Stripe webhook parsing, Planner** (Cursor CLI, idle): reads files → diagnosis → **Proposed plan** (4 steps) → approval **Execute plan**.
- **Lazy-load dashboard, Lead**: user prompt → created worktree → live (provisioning).

## Keyboard and accessibility

- The conversation is an ordered list labelled "Conversation".
- Speaker headers are `aria-hidden` (the announcement and list structure carry who said what).
- File name links are real buttons with `aria-label="View diff for <path>"`.
- The thread panel is a tabpanel labelled by its tab.

## Data model

Each thread has `timeline[]` of items:

```
{ kind: 'user',     text, t }
{ kind: 'thought',  text, t }
{ kind: 'tool',     icon, title, detail?, chips?: [[label, tone]], tone?, t }
{ kind: 'edit',     path, t }                 // counts come from workspace.files
{ kind: 'plan',     t }                       // steps come from workspace.plan
{ kind: 'approval', id, title, what, why, ok, no, state, doneAt?, t }
{ kind: 'findings', t }
{ kind: 'live' }                              // always last while working
```

New items are inserted before the `live` item so it stays last.

## Simulated in the prototype

- All agent output is scripted. A real build must translate each engine's native event stream (Claude Code's stream-JSON, Cursor CLI, Codex, Gemini) into these item kinds. That adapter layer is the core of the product.
- Times are static strings (`12m`, `now`) and don't update.
- The live subtitle rotates through a fixed list; a real build should show the current tool call.

## Known gaps and open questions

- Tool runs can't be expanded to show full input and output (the exact command, file contents read, test output).
- Agent thoughts render as plain text; no markdown, code blocks or links.
- No copy, retry, edit-and-resend or branch-from-here on messages.
- No search within a thread.
- Very long threads will need virtualisation or collapsing of older runs.
- Times should be relative and update, with absolute time on hover.
- No way to see token usage or cost per thread.
