# 10 · Thread tabs

## Summary

Directly under the header, a tab bar holds one tab per agent thread in the workspace, a **+** to start another thread, a **Findings** tab once a review has finished, and an **Output** tab pinned to the right for the running app. Each thread tab shows live status and a count of approvals waiting, so the user can see which thread needs them without opening it.

## Why it exists

The conversation is the product, and a workspace can have several agents working in the same worktree (a Lead, a Test writer, a Reviewer). Tabs make each conversation one click away while keeping the one in front full-width.

## Where it lives

- `<div class="thread-bar" data-od-id="thread-tabs">` below the workspace header.
- Contains `#thread-tabs` (thread and Findings tabs), `#thread-add` (the +), and `#out-tabs` (Output).
- Hooks: `thread-tab-<thread id>`, `thread-tab-findings`, `thread-tab-output`, `thread-add`.

## Anatomy

Left to right:

1. **Thread tabs**, one per thread, in creation order. The first is always the workspace's own thread (Lead, Planner or Reviewer). Each tab has:
   - A status dot (same colours as the sidebar: green pulsing running, amber paused, provisioning, grey idle).
   - The thread's role name, for example `Lead`, `Test writer`, `Reviewer`, `Agent 4`.
   - An amber count badge if the thread has pending approvals.
   - Tooltip: `<role>: <activity>` (or `Paused`).
   - A close (×) button on every tab but the first, shown on the selected tab and on hover.
2. **Findings tab** (review workspaces, only after the review finishes): a list icon, `Findings`, and a count of findings not yet sent. See [20](20-review-findings.md).
3. **+ (New thread)** icon button.
4. Flexible space.
5. **Changes tab**, pinned right: a file icon, `Changes`, and `+N −N` when there are changed files. See [14](14-changes-panel.md).
6. **Output tab**, pinned right: a terminal icon, `Output`, and a status mark:
   - Spinner while the worktree is setting up or the app is starting.
   - Green dot and the port (for example `:5173`) while the app is running.
   - Nothing when stopped.
   See [15](15-run-app-and-output.md).

## Behaviour

### Selecting tabs
- Clicking a thread tab shows that thread's conversation and scrolls it to the end.
- Clicking Findings shows the findings panel, scrolled to the top, focusing the tab.
- Clicking Changes shows the Changes panel in place of the conversation.
- Clicking Output shows the output panel. The tab you came from is remembered so the header's Output toggle (and `Ctrl+\``) can take you back.
- Opening a workspace always starts on the first thread tab, unless it was opened from a specific agent row in the sidebar, in which case that thread's tab is selected.

### Starting a new thread (+)
Also available as **New thread** in the ⋯ menu.

1. A thread is added with role `Agent <N>` (N = new total thread count), engine Claude Code, status provisioning, activity `Joining worktree…`.
2. Its tab is selected immediately and appears at the end of the thread tabs.
3. Toast: `Started a new thread in <workspace name>`.
4. After 2.4 seconds it becomes running with activity `Waiting for instructions…` and posts: `I’m in the same worktree as the Lead. Tell me which part to take on and I’ll coordinate so we don’t edit the same files.`

If started from a workspace that isn't open (not currently possible from the UI, but supported by the action), the app opens that workspace on the new tab.

### Closing a thread
- The × on a tab, a middle-click on it, or `Delete` / `Backspace` while it's focused closes it.
- Closing stops the thread's agent and hides the tab; its history stays in the database (`threads.closed_at`). There's no way to reopen it yet.
- The workspace's own thread (the first tab) can't be closed; the backend refuses too.
- Closing the selected tab selects the tab before it, or after it if it was first.

### Live updates
Tabs re-render on every change: status dots, approval counts, the Findings count and the Output port all stay current.

## Keyboard and accessibility

- Thread and Findings tabs are in a `role="tablist"` labelled "Agent threads"; Output is in a separate `role="tablist"` labelled "App".
- Each tab is `role="tab"` with `aria-selected`, `aria-controls` pointing at its panel, and a roving `tabindex` (only the selected tab is in the Tab order).
- **Arrow keys** (`←` `→`) move between *all* tabs in the bar, including Findings and Output, wrapping at the ends. `Home` and `End` jump to the first and last. Moving selects the tab immediately (automatic activation) and keeps focus on it.
- Thread tab `aria-label`: `<role>, <activity>[, <N approvals> waiting]`. Findings: `Review findings, <N open findings>`. Output: `App output, <state>`, where state is `setting up`, `starting`, `running on localhost:<port>` or `stopped`.
- The thread panel is labelled by the selected thread's tab.

## Data model

- `state.agentIdx`: index of the selected thread in `[workspace, ...workspace.extra]`.
- `state.wsTab`: `thread` | `findings` | `output`.
- `state.lastTab`: tab to return to from Output.
- New thread object: `id`, `role`, `engine`, `status`, `activity`, `paused`, `liveSteps`, `liveIdx`, `timeline`.

## Simulated in the prototype

- Threads are data objects; no agent processes. A real build starts a new engine process attached to the same worktree directory.
- Coordination between threads sharing a worktree (who edits which file) is only described in agent copy. Nothing prevents two agents editing the same file.

## Known gaps and open questions

- **New threads ignore the default engine** and always use Claude Code. There's no way to choose the engine or role when adding a thread.
- Threads can't be renamed or reordered, and a closed thread can't be reopened.
- Closing a running agent stops it without asking.
- No limit on the number of threads, and no overflow handling when tabs exceed the width.
- Should threads be able to run on separate worktrees (sub-branches) rather than sharing one, to avoid file conflicts?
- No keyboard shortcut to jump to thread N (for example `⌘1`–`⌘9`).
- No unread indicator: a thread that posted something new while you were on another tab looks the same unless it's waiting on an approval.
