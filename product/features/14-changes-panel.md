# 14 · Changes panel and diff review

## Summary

The Changes panel shows everything the agents have changed in the worktree: a list of changed files with line counts, and a syntax-highlighted diff of the selected file in unified or split layout. The user can leave comments on lines and send them to the agent as one message. It is a tab pinned to the right of the thread bar, beside Output, and fills the workspace body when selected.

## Why it exists

The user must be able to check the agent's work before it becomes a PR, without leaving the workspace or opening an editor. Keeping it closed by default respects **The conversation is the product**: code is on demand.

## Where it lives

- `<div role="tabpanel" id="changes-panel" aria-labelledby="thread-tab-changes" data-od-id="changes-panel">` in the workspace body, in place of the conversation.
- Entry points:
  - **Git** tab in the thread bar, right-aligned before Output (shows `+N −N`).
  - Clicking a file name in an edit step in the conversation (opens the tab on that file).
- The file tree and in-app editor are deliberately out of scope for now.

## Anatomy

### Stack and toolbar
Two panes: the stack (25) on the left and the diff on the right. The stack picks what the diff shows:
- **Uncommitted changes** (the top level, and the default): worktree vs `HEAD`, including untracked files.
- **A branch**: its committed changes since it left its base, the branch below it in the stack or the trunk (`git diff <base>...<branch>`). The base is `origin/<base>` when that has everything the local branch has, else the local branch, so a rebased but unpushed stack still diffs cleanly. Review workspaces start on `HEAD` against their PR base.

The diff's toolbar is one row: what it shows (`Uncommitted changes` or `feat/b vs feat/a`), then (when there are files) the **Unified** / **Split** toggle and total `+N −N`; on the right, **Collapse all** / **Expand all**, **Discard all** (uncommitted changes only) and **Send N comments to <thread>** (when there are draft comments).

### Files (accordion)
One scrolling list with a section per changed file, in the order files were changed. Every file starts expanded; any number can be open at once.

- **Header** (sticky while its diff scrolls past; a button with `aria-expanded`): chevron, path with the directory in a subtler colour and the file name emphasised (full path in the tooltip), comment count (speech-bubble icon, if any), `+N` / `−N` (zero counts omitted), and the status letter `M` / `A` / `D` / `R`, colour-coded (screen readers hear the word). A moved or renamed file is one entry, `old → new`, not a deletion plus an addition. On uncommitted changes, a discard button (shown on hover or focus) sits at the end of the header.
- **Body**: the file's diff, when expanded.

### Diff body
Rendered by [`@git-diff-view/svelte`](https://github.com/MrWangJustToDo/git-diff-view) (patched in `patches/` so comments show in unified mode), recoloured to the app's surfaces.
- **Unified** or **split** layout, line numbers, syntax highlighting (highlight.js via lowlight) and word-level change highlights.
- **Comments:** hovering a line shows a `+`; clicking it opens a comment box under the line (`⌘↵` adds, `Esc` cancels). Added comments show under their line with a delete (×) button. Unsent text in a box survives a diff refresh.

### Empty state (no changes)
- Diff header: `No changes`.
- Body: mop-sparkles icon, `Clean diff!`, `Go make some changes`

## Behaviour

- **Opening:** the Git tab, or a file link in the conversation. Picking another tab leaves it.
- **Clicking a file header** expands or collapses that file.
- **A file link in the conversation** opens the tab, expands that file and scrolls it to the top.
- **Switching layout** keeps the vertical scroll position.
- **Send N comments** posts every draft comment in the workspace to the open thread's agent as one message (each as `path:line`, the quoted line and the note; removed lines are marked), clears them and switches to that thread so its reply is in view. If sending fails the comments are restored. Drafts live in memory only.
- **Discard** (a file, or **Discard all**) asks first, since it can't be undone, then puts tracked files back to `HEAD` (index included) and deletes new files; ignored files are left alone. A branch level has no discard: its changes are commits.
- Opening a different workspace expands every file again and returns to the thread tab.
- New files added by agents (for example after approving a plan or a delete) appear in the list and update the header counts.

## Layout

- Full width of the workspace body: an 18rem stack pane, then the diff. The toolbar wraps on narrow windows.

## Keyboard and accessibility

- The panel is a `tabpanel` labelled by the Git tab; the file list is labelled "Changed files".
- The scrolling file list is focusable (`tabindex="0"`) and labelled "Proposed changes", so it can be scrolled with the keyboard.
- File headers are buttons with `aria-expanded` / `aria-controls`; status letters have their full words.
- The layout toggle uses `aria-pressed`.
- Diff lines themselves come from git-diff-view and have no screen-reader text for `+` / `−` (a regression from the prototype).

## Data model

Per workspace, `files[]`:

```
{ path, oldPath, added, deleted, hunks: [{ header, body }] }
```

Draft comments, per workspace: `{ id, path, side: 'old' | 'new', line, code, body }`.

`state.changesOpen`, `state.fileIdx`, `state.diffMode` (`unified` | `split`).

## Simulated in the prototype

- Diffs are hard-coded. A real build derives them from `git diff <base>...HEAD` plus uncommitted changes in the worktree, updating live as agents write files.

## Known gaps and open questions

- **The diff doesn't change after a branch switch;** it still shows the old branch's files (see [16](16-branch-switching.md)).
- Review workspaces show an empty Changes panel, so the user can't see the PR's diff or the diff of fixes made from findings.
- Comments are one line at a time; no range selection, and they can't be drafted as PR review comments yet.
- Comments keep their line number when the file changes underneath them; the quoted code in the message keeps them meaningful.
- No "whole-file" view, no expanding context around hunks.
- No per-file filter or search; no grouping by directory for large diffs.
- No indication of which thread made which change.
- No way to discard a single hunk.
