# 19 · Review workspaces

## Summary

A review workspace is a workspace created from a pull request instead of a prompt. Harness checks out the PR's branch into its own worktree and starts a **Reviewer** thread instructed to find bugs, risky changes and missing tests, without pushing anything. When the review finishes, the findings open in their own tab (see [20](20-review-findings.md)), and the header's primary action is **Submit review**.

## Why it exists

Reviewing a PR properly means checking it out, running it and reading it carefully. A review workspace does the checkout and the first reading pass in parallel with everything else, and returns a structured, selectable list of issues rather than a wall of text.

## Where it lives

- Created from Homebase → Open pull requests → **Review in workspace** on any PR row (see [05](05-open-pull-requests.md)).
- Otherwise it's a normal workspace: card on Homebase, rows in the sidebar, tabs, Output, Changes.
- Differences are keyed on `kind: 'review'`.

## How it differs from a build workspace

| Aspect | Build workspace | Review workspace |
| --- | --- | --- |
| Created from | New workspace dialog | A PR row |
| Branch | New branch from base | The PR's head branch |
| Name | From the prompt | `PR #482 Review` |
| First thread | Lead | Reviewer |
| Engine | Chosen in dialog | Default engine from Settings |
| Card details line | `Branch: …` first | `Reviewing #482` first |
| Header primary | Create PR | Submit review |
| Extra tab | | Findings, once the review finishes |
| Provisioning time | 5.2s | 4.2s |

## Behaviour

### Starting a review
1. User clicks **Review in workspace** on PR #482.
2. If a review workspace for #482 already exists, it's opened instead (the row would already show **Go to Workspace**).
3. A workspace is created at the top of the list:
   - id `pr482-<8 random hex>`, name = `PR #482 Review`, branch = PR head, base = PR base, engine = default engine.
   - Status provisioning, activity `Checking out the PR…`, goal `Review #482`.
   - Thread role **Reviewer**. Live steps: `Reading the PR description`, `Walking the diff file by file`, `Checking test coverage for the changed code`.
   - Timeline:
     - User message: `Review #482 “Retry Stripe webhooks with exponential backoff” by @maya-r. Flag bugs, risky changes and missing tests. Don’t push commits; draft comments for me to approve.` (`by you` for your own PRs.)
     - Step **Checked out #482** (PR icon), detail `maya/webhook-retry → main · 6 files changed`.
4. The Homebase filter resets to All; toast `Opened a review workspace for #482`.
5. Provisioning runs for 4.2 seconds (setup commands stream to Output; see [07](07-worktree-provisioning.md)), and the app **stays on Homebase**, like new build workspaces. Toasts link to the workspace: one when it is created, and `Reviewer started on #482` once the Reviewer has its prompt.
6. When provisioning ends: activity `Reading the diff…`, and the summary becomes `Reviewing #482 from @maya-r against main. <Engine> is reading the diff and will draft comments for your approval; nothing has been posted to GitHub yet.`

### Review finishing
The engine prompt (not shown in the thread) asks the Reviewer to write its review in prose, then end its final message with every finding as a JSON array inside `<cormux-findings>…</cormux-findings>` tags: `severity` (`blocking` / `suggestion` / `nit`), `title`, `file` (repo-relative or null), `line` (in the PR's version of the file, or null) and `explanation`. The conversation never shows the block, even while it streams in.

When a Reviewer turn ends and the review isn't ready yet, Cormux reads the block from the agent's reply since the last user message (`src-tauri/src/findings_block.rs`):

- **Parsed:** findings are stored (severities are normalised, absolute paths made repo-relative, bad lines dropped), and then:
  1. The review is marked ready and the Reviewer's activity becomes `Review ready`.
  2. Summary: `Reviewed #482 from @maya-r: 2 blocking issues, 2 suggestions and 2 nits. Choose which to send back for fixes in the Findings tab; nothing has been posted to GitHub yet.` (or `…: no findings.`)
  3. The **Findings** tab appears, with blocking findings selected. Toast: `Review of #482 finished · 6 findings` (or `· no findings`).
- **Missing or not valid JSON:** a step **Couldn’t read the review findings** explains what went wrong, and the review stays open. The next turn is checked again, so asking the Reviewer to finish its review with the findings block recovers.

### Submitting the review
1. **Submit review** in the header.
2. Every thread becomes idle with activity `Review submitted`.
3. Step **Submitted review on #482** (PR icon, success tone), detail `Posted to GitHub`.
4. Toast: `Submitted your review on #482`.
5. **Submit review** becomes disabled.

There's no confirmation dialog, no choice of verdict (approve / request changes / comment), and findings aren't included.

## Copy

- Card: `Reviewing #482`
- Header primary: `Submit review`
- Thread step: `Checked out #482`, `Submitted review on #482`

## Data model

Review workspace adds:

```
kind      'review'
pr        PR number
author    '@maya-r' or 'you'
findings  { items: [...], target: threadIndex }   // set when the review finishes
```

`prWs(num)` finds a review workspace by PR number.

## Simulated in the prototype

- Checkout is a timer. A real build fetches the PR head (`git fetch origin pull/<N>/head:<branch>` for forks) and creates a worktree on it.
- "Posted to GitHub" is not real. A real build uses the GitHub reviews API.

## Known gaps and open questions

- **Submit review doesn't use the findings.** The natural flow: findings the user *didn't* send for fixing become draft review comments on the right lines; the user picks a verdict and submits.
- No verdict choice (Approve / Request changes / Comment).
- The Changes panel is empty in review workspaces; it should show the PR's diff, and later the diff of fixes.
- Fixes made from findings are "committed locally, not pushed". Pushing fixes to someone else's PR branch needs a decision: push to their branch (if allowed), open a follow-up PR, or suggest changes as review suggestions.
- The repo for the PR isn't known; review workspaces assume `my-app`.
- No re-review after the author pushes new commits.
- Reviewing your own PR (#477, #468) is odd; see [05](05-open-pull-requests.md).
- Engine can't be chosen for a review; it always uses the default.
