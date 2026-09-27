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
| Name | From the prompt | The PR title |
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
   - id `pr482`, name = PR title, branch = PR head, base = PR base, engine = default engine.
   - Status provisioning, activity `Checking out the PR…`, goal `Review #482`.
   - Thread role **Reviewer**. Live steps: `Reading the PR description`, `Walking the diff file by file`, `Checking test coverage for the changed code`.
   - Timeline:
     - User message: `Review #482 “Retry Stripe webhooks with exponential backoff” by @maya-r. Flag bugs, risky changes and missing tests. Don’t push commits; draft comments for me to approve.` (`by you` for your own PRs.)
     - Step **Checked out #482** (PR icon), detail `maya/webhook-retry → main · 6 files changed`.
4. The Homebase filter resets to All; toast `Opened a review workspace for #482`.
5. Provisioning runs for 4.2 seconds (setup commands stream to Output; see [07](07-worktree-provisioning.md)), and the app **navigates into the workspace immediately** (unlike new build workspaces, which stay on Homebase).
6. When provisioning ends: activity `Reading the diff…`, and the summary becomes `Reviewing #482 from @maya-r against main. <Engine> is reading the diff and will draft comments for your approval; nothing has been posted to GitHub yet.`

### Review finishing
Six seconds after provisioning ends (about 10 seconds after clicking). If the Reviewer is paused at that moment, it checks again every 2 seconds until resumed.

1. Findings are loaded for the PR. Blocking findings start selected.
2. Steps and messages added to the Reviewer thread:
   - **Read <N changed files>**, detail `<head> against <base>`, chip `Read-only, auto-approved`.
   - Thought: `Done. I found 2 blocking issues, 2 suggestions and 2 nits. Pick the ones you want fixed in the Findings tab and I’ll work through them; nothing has been posted to GitHub.` (or `Done. I didn’t find anything worth flagging in this diff.`)
   - A **Review findings** card with counts and an **Open findings** button.
3. The Reviewer becomes idle with activity `Review ready`.
4. Summary: `Reviewed #482 from @maya-r: 2 blocking issues, 2 suggestions and 2 nits. Choose which to send back for fixes in the Findings tab; nothing has been posted to GitHub yet.`
5. The **Findings** tab appears. Toast: `Review of #482 finished · 6 findings`.

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
- The review itself is scripted. A real build prompts the engine with the PR description, diff and review rubric, and must get structured findings back (severity, title, file, line, explanation). That structure is essential to the Findings tab.
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
