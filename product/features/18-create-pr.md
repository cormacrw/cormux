# 18 · Create PR

## Summary

**Create PR** is the primary action in a build workspace's header. It opens a small dialog with one field, "Why is this change necessary?", which a small model drafts automatically from the thread and diff. The user edits the draft and creates the PR. The new PR appears on Homebase and the header button changes to `PR #N opened`.

## Why it exists

Getting agent work into review is the end of the build loop. The hardest part of a PR description is the *why*; the agent has that context in its thread, so Harness drafts it. The user stays in control of what's sent. Serves **Drafted, never presumed**.

## Where it lives

- Header primary button `data-od-id="ws-create-pr"` (build workspaces only; review workspaces show Submit review instead).
- Dialog `<dialog id="pr-dialog" data-od-id="pr-dialog">`.
- Hooks: `pr-why-field`, `pr-redraft`, `pr-submit`.

## Anatomy

### Header button
- Default: primary **Create PR** with a PR icon.
- After creating: disabled `PR #<N> opened`.

### Dialog
1. **Title:** `Create pull request`.
2. **Route line** under the title: `<branch> → <base> · <N files> +A −D`, for example `feat/auth → main · 3 files +32 −4`. With no changes: `· No file changes yet`.
3. **Close** (×) icon button.
4. **Field:** label `Why is this change necessary?` with a ghost **Redraft** button on the same row.
   - Textarea, 5 rows, placeholder `The problem this solves and why it matters now`.
   - While drafting: a shimmering three-line placeholder overlays the textarea.
   - Hint below (live region): `Drafting from the thread and diff…` then `Drafted from the thread and diff. Edit it before creating.`
   - Error: `Add a reason so reviewers know what this fixes.`
5. **Footer:** hint `⌘ ↵ to create`, ghost **Cancel**, primary **Create PR**.

There's only one field. Title, body and reviewers aren't asked for.

## Behaviour

### Opening
1. Only opens if the workspace hasn't already created a PR. Any open popover closes.
2. The route line is filled in, the submit label reset, the dialog opens with focus in the textarea, and drafting starts.

### Drafting
1. The textarea clears and becomes read-only (`aria-busy`), the shimmer shows, and **Redraft** and **Create PR** are disabled.
2. After **1.2 seconds** the draft appears, the field becomes editable, the buttons re-enable, and the hint updates.
3. **Redraft** repeats this, replacing whatever the user typed.
4. Closing the dialog cancels a pending draft.

### Draft content
- The three sample workspaces have written reasons. For example, Auth session timeout: `Sessions never expire while the cookie is valid, so a forgotten or stolen session stays usable indefinitely. This adds a 30-minute sliding idle timeout and a 12-hour absolute cap: stale sessions are rejected in getSession() and the user is sent back to /login.`
- Other workspaces: `This change is needed to <goal, first letter lowercased>. <first sentence of the summary>.`

### Validation
- Editing the field clears the error once it has text.
- Submitting with an empty field shows the error and focuses the field.

### Creating
Triggered by **Create PR** or `⌘↵` / `Ctrl+↵` in the field.

1. The button disables and shows a spinner with `Creating PR…`.
2. After 900ms:
   - A PR number is assigned (highest existing number + 1).
   - The workspace records it, so the header shows `PR #<N> opened`.
   - A PR is added to the top of Homebase's Open pull requests: title = workspace name, author you, `Opened by you`, head/base from the workspace, updated just now, `Checks running`, `Awaiting review`, and the diff's file and line counts.
   - A step is added to the Lead's thread: **Opened PR #<N>** (PR icon, success tone), detail `<branch> → <base>`, chip `Checks running`.
   - The dialog closes. Toast: `Opened PR #<N> for <branch>`.

### Cancelling
Cancel, ×, `Esc` or backdrop click.

## Keyboard and accessibility

- Dialog labelled by its title.
- Textarea described by the hint and error; `aria-invalid` when empty on submit; `aria-busy` while drafting.
- The hint is a polite live region, so "Drafting…" and "Drafted…" are announced.
- The shimmer is `aria-hidden`.

## Data model

- Reads: workspace `id`, `title`, `branch`, `base`, `files`, `goal`, `summary`.
- Writes: workspace `prNum`; prepends to `state.prs`; timeline step.

## Simulated in the prototype

- **The model call is a timer with canned text.** A real build sends a small model the thread (user prompt, key agent reasoning), the diff stats and file list, and asks for a two- to three-sentence problem statement. Keep it fast (under ~2 seconds) and cheap.
- **PR creation is local.** A real build must: commit any uncommitted work (or ask), push the branch (`git push -u origin <branch>`), then create the PR via the GitHub API with a title (the workspace name?) and body (the reason, plus perhaps a generated summary of changes and test results). It must surface push and API failures in the dialog.

## Known gaps and open questions

- **No title field.** The PR title silently becomes the workspace name. Decide whether to show it, editable.
- Only the "why" is drafted. Should the body also include "what changed" and "how it was tested" (the agent knows both)?
- No draft-PR option, no reviewers, labels or linked issues.
- No check that the work is committed or that files were approved in the Changes panel.
- **After a branch switch** the button stays `PR #N opened` for the new branch.
- Once opened, the button is disabled. It should probably become "View PR" (open on GitHub) and later reflect PR status (checks, review state, merged).
- No path to merge after approval, and **Teardown after merge** has nothing to hook into.
- Pushing further commits after the PR is open isn't represented.
