# 27 · Plan mode

## Summary

A Claude thread can be switched to **plan mode**. The agent reads the code and writes a plan, but edits nothing. When it's done, the plan opens in a **Plan** tab as a document. The user highlights parts of it, comments, and sends the comments back for a revision, as many rounds as it takes. Approving the plan sends the agent off to carry it out.

## Why it exists

For anything bigger than a small fix, agreeing on the approach before code is written is cheaper than reviewing a finished diff that went the wrong way. Commenting on the exact sentence that's wrong is quicker and clearer than writing the feedback out in the composer. Serves **Drafted, never presumed**: the agent proposes, the user decides when work starts.

## Where it lives

- **Plan** toggle in the composer footer, after the model picker (`composer-plan-mode`). Claude threads only; scratches are always read-only, so they don't show it.
- Tab `thread-tab-plan` in the tab bar, right after the thread tabs and before Findings (see [10](10-thread-tabs.md)).
- Panel `#plan-panel` (`data-od-id="plan"`), with body `#plan-body` and action bar `#plan-bar`.
- In the conversation, the plan shows as a slim row like the findings one (see [13](13-approvals.md)): `Plan ready for review · <plan title>` with a **Review plan** link that opens the tab. Once decided it reads `Plan approved` or `Plan sent back`, with **Open plan**.
- Hooks: `plan-markdown`, `plan-status`, `plan-thread`, `plan-comment-start`, `plan-comment-form`, `plan-comments`, `plan-comment`, `plan-note`, `plan-send-feedback`, `plan-approve`.

## Anatomy

### Composer toggle
A ghost button with a clipboard icon and `Plan`. It's a toggle (`aria-pressed`), tinted when on. While on, the composer placeholder reads `Ask <agent> to plan something…`. Switching posts a step in the conversation: `Switched to plan mode` or `Left plan mode`.

### Tab
Clipboard icon and `Plan`. An amber `Review` badge while a plan is waiting. The tab appears once any thread in the workspace has handed over a plan, and stays so the approved plan can be read later.

### Panel
Two columns on wide windows: the plan, and a comment rail on the right. On narrow windows the rail drops below the plan.

- **Meta line:** `Plan from <agent> with Claude Code`, plus `· revision 2` from the second plan on. When more than one thread has a plan, a **Showing** select picks the thread; waiting ones say `· waiting`.
- **Heading** by state:
  - Waiting: `Review the plan` / `Nothing is edited until you approve. Highlight any part of the plan to comment on it, then send your comments back or approve it.`
  - Approved: a green check, `Plan approved` / `<agent> is carrying out this plan.`
  - Sent back: `Feedback sent` / `<agent> is revising the plan.` (with a spinner while it works), and a collapsed **What you sent** showing the message.
- **The plan:** the agent's markdown, rendered with headings, lists, tables and code blocks. Its text is selectable.

### Commenting
1. Selecting text in a waiting plan shows a **Comment** button just below the selection.
2. **Comment** opens a small card in the same place: `New comment`, a textarea (`What should change here?`), **Cancel** (`Esc`) and **Add comment** (`⌘↵`).
3. Saved comments keep their text highlighted in the plan. A comment card in the rail shows the quoted text and the note; clicking it scrolls to the highlight and flashes it. A trash button deletes the comment.
4. Rail heading: `2 comments, not sent yet`, or `No comments yet` / `Select text in the plan, then choose Comment.`

### Action bar (waiting plans)
- **Overall feedback (optional)** textarea, for notes that aren't about one passage. `⌘↵` sends.
- **Send 2 comments** (or **Send feedback** when there's only a note): outline button, disabled with nothing to send.
- **Approve plan**: green, disabled while there's unsent feedback (`Send or delete your feedback before approving`).

When the plan isn't waiting, the bar says `Follow the work in <agent>'s thread.` or `The revised plan opens here when <agent> is done.`, with **Go to <agent>**.

## Behaviour

- **Plan mode is per thread** and survives restarts. A running agent switches straight away; otherwise it starts in plan mode next time.
- **A plan arriving** while the app is open switches the workspace to the Plan tab, if the user is in that workspace. Plans already waiting at launch don't switch tabs; the badge and the approval card point to them.
- **Waiting, not working.** While a plan waits, the agent's turn is on hold: the thread's status is `waiting` (amber dot, `Plan ready for review`, no working spinner) and the workspace needs attention. Answering the plan puts it back to working. The agent's raw `ExitPlanMode` step is hidden; the plan's row stands for it.
- **Replying from the composer.** While a plan waits, the composer reads `Reply to <agent>'s plan, or review it in the Plan tab…`, and a message sent there goes back as feedback on the plan, like an overall note.
- **Leaving plan mode.** Turning the composer's **Plan** toggle off, or closing the Plan tab (its x, `Close the plan and leave plan mode`), switches the agent back to normal mode at any point. A plan still waiting is dropped: the agent is told the user left plan mode and carries on normally, and its row reads `Left plan mode before reviewing`. Closing the tab also hides the plans it showed; the tab comes back with the next plan.
- **The toggle follows the agent.** Handing over a plan means the agent is in plan mode, so the toggle turns on then, even if the agent entered plan mode by itself.
- **No approval pill.** A waiting plan has its own status and the Plan tab's `Review` badge, so it doesn't add to the approval count pills on the agent tab or in the sidebar.
- **Never auto-approved.** A plan waits for the user even with **Run everything** on.
- **Send comments** denies the agent's request to start, with one message: an instruction to revise, then each comment as the quoted plan text followed by the note, in plan order, then `Overall: <note>`. The agent stays in plan mode and hands over a revised plan, which becomes the next revision. The conversation gets a slim `Sent feedback on the plan` step; the full message stays under **What you sent**. Toast: `Sent 2 comments to <agent>`.
- **Approve plan** lets the agent start and takes the thread out of plan mode (the composer toggle turns off), back to the mode it normally works in, such as Claude's auto mode. The agent then works like any other thread. The view returns to the agent's thread. Toast: `Plan approved. The agent is carrying it out`.
- Draft comments belong to one plan. They're kept while switching tabs, and cleared once sent.

## Data model

```
approval (tool: execute_plan) = {
  id, threadId, status: 'pending' | 'approved' | 'denied',
  payload: { title, what: <plan title>, plan: <markdown>, effect: 'execute_plan', denyReason?, resolvedAtMs? }
}
setting thread:<threadId>/planMode = 'true' | 'false'
draft comment (in memory) = { id, start, end, quote, body }   // start/end: offsets into the rendered plan's text
```

## How it maps to Claude Code

- Plan mode is Claude's `plan` permission mode: `--permission-mode plan` at spawn, or a `set_permission_mode` control request on a live session.
- The plan arrives as Claude's `ExitPlanMode` tool call (`input.plan`), which asks permission through the host.
- Approve answers `allow` with `updatedPermissions: [{ type: 'setMode', mode: <normal mode>, destination: 'session' }]`. The normal mode is the one the thread's sessions start in outside plan mode (Claude reports it in its `init` event, for example `auto`), else `permissions.defaultMode` from Claude's settings for the worktree (local, project, then user), else `default`. Leaving plan mode goes back to the same mode.
- Send comments answers `deny` with the feedback as the message.

## Known gaps and open questions

- Claude only. Codex and Gemini have their own plan or ask modes over ACP; they aren't wired up yet.
- No diff between revisions, and earlier revisions can't be opened.
- Comments can't be edited, only deleted and re-added.
- Highlighting text needs a pointer; keyboard users can still send overall feedback.
- No choice of mode after approval, such as auto-accepting edits for just this plan.
