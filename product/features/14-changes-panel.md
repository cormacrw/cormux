# 14 · Changes panel and diff review

## Summary

The Changes panel shows everything the agents have changed in the worktree: a list of changed files with line counts, and a syntax-highlighted diff of the selected file in unified or split layout. The user can approve or reject each file. The panel is closed by default and opens beside the conversation (or over it on narrow windows).

## Why it exists

The user must be able to check the agent's work before it becomes a PR, without leaving the workspace or opening an editor. Keeping it closed by default respects **The conversation is the product**: code is on demand.

## Where it lives

- `<aside class="changes" id="changes" aria-label="Changes" data-od-id="changes-panel">` inside the workspace body.
- Entry points:
  - **Changes** button in the header (toggle, shows `+N −N`).
  - Clicking a file name in an edit step in the conversation (opens the panel on that file).
- The file tree and in-app editor are deliberately out of scope for now.

## Anatomy

### Panel header
- `Changes` title, total `+N −N` (if there are files), and a close (×) icon button.

### File list
One row per changed file, in the order files were changed:

- **Path**, with the directory in a subtler colour and the file name emphasised; full path in the tooltip.
- **Review mark**: a check (approved) or x (rejected), if reviewed.
- **Counts**: `+N` and/or `−N` (zero counts are omitted).
- **Status letter**: `M` (Modified), `A` (Added), `D` (Deleted), colour-coded; screen readers hear the word.
- The selected file is marked `aria-current="true"`.

Hidden when there are no changes.

### Diff header
- File icon, the full path, a status chip (`Modified` amber, `Added` green, `Deleted` red), and `+N −N`.
- **Layout toggle**: segmented **Unified** / **Split**.
- **Review controls**:
  - Unreviewed: a red **Reject** pill and a green **Approve file** pill (check icon).
  - Approved: `Approved` green chip and an **Undo** pill.
  - Rejected: `Rejected` red chip and an **Undo** pill.

### Diff body
- **Unified:** each line has old and new line numbers, a sign column (`+`, `−` or blank), and the code. Each hunk starts with a header like `@@ -1,11 +1,18 @@ getSession`.
- **Split:** two side-by-side columns labelled "Before" and "After", with a striped fill where one side has no line. Horizontal scrolling of the two sides is synced.
- **Syntax highlighting** for comments, strings, keywords, numbers, type names and function calls.
- **Word-level highlights:** when a removed line is directly replaced by an added line, the changed middle portion is highlighted on both lines, unless it covers more than 80% of the line (then the whole line is treated as changed).

### Empty state (no changes)
- Diff header: `No changes`.
- Body, depending on the workspace:
  - Has an unapproved plan and is idle: list icon, `Plan ready, nothing edited yet`, `Review the plan in the thread. Once you approve it, proposed edits stream in here for file-by-file review.`
  - Otherwise: clock icon, `Waiting for the first edit`, `The agent is still reading the repository. Diffs appear here as soon as it proposes a change.`

## Behaviour

- **Toggle:** the header button or the panel's × toggles the panel. `Esc` closes it when no dialog is open.
- **Selecting a file** (in the list or from a conversation link) opens the panel if needed, shows that file, and scrolls the diff to the top.
- **Switching layout** keeps the vertical scroll position.
- **Approve / Reject / Undo** set the file's review state; the list's review mark updates.
- Opening a different workspace resets the selection to the first file. The panel's open/closed state carries across workspaces.
- New files added by agents (for example after approving a plan or a delete) appear in the list and update the header counts.

## Layout

- **Wider than 1100px:** the workspace body splits into two columns: conversation on the left, Changes on the right at `minmax(380px, 44%)`.
- **1100px and below:** the panel slides over the conversation from the right, up to 520px wide, with a shadow, instead of squeezing the conversation.

## Keyboard and accessibility

- The panel is an `<aside>` labelled "Changes"; the file list is labelled "Changed files".
- The diff body is focusable (`tabindex="0"`) and labelled "Proposed changes", so it can be scrolled with the keyboard.
- Signs have screen-reader text (`Added:`, `Removed:`); status letters have their full words.
- The layout toggle uses `aria-pressed`.
- Split columns are labelled "Before" and "After".

## Data model

Per workspace, `files[]`:

```
{ path, status: 'M' | 'A' | 'D', review: 'pending' | 'approved' | 'rejected',
  hunks: [{ oldStart, newStart, ctx, lines: ['+added', '-removed', ' context'] }] }
```

Counts, hunk headers, line numbers and word ranges are derived and cached per file.

`state.changesOpen`, `state.fileIdx`, `state.diffMode` (`unified` | `split`).

## Simulated in the prototype

- Diffs are hard-coded. A real build derives them from `git diff <base>...HEAD` plus uncommitted changes in the worktree, updating live as agents write files.
- **Approve / Reject** only change a label. Decide what they mean for real: does Reject revert the file, or tell the agent to redo it? Does Approve stage it? Does Create PR require every file approved?

## Known gaps and open questions

- **The diff doesn't change after a branch switch;** it still shows the old branch's files (see [16](16-branch-switching.md)).
- Review workspaces show an empty Changes panel, so the user can't see the PR's diff or the diff of fixes made from findings.
- No inline comments on lines (which would be the natural way to send feedback to an agent or draft PR review comments).
- No "whole-file" view, no expanding context around hunks.
- No per-file filter or search; no grouping by directory for large diffs.
- No indication of which thread made which change.
- No way to discard a single hunk.
- Review marks don't feed into anything (Create PR, findings, agent instructions).
