# 05 · Open pull requests

## Summary

Below the workspace cards, Homebase lists open GitHub pull requests that involve the user: ones they opened, ones where their review is requested, and ones where they're mentioned. Each row carries enough signal (checks, review state, size, age) to decide whether to act, and a **Review in workspace** button that spins up a review workspace for that PR.

## Why it exists

Reviewing is half of a developer's day. Bringing PRs into Homebase means the user can treat "review #482" as just another parallel task with its own worktree and agent, rather than switching to GitHub.

## Where it lives

- `<section class="pr-section" data-od-id="open-prs">` on Homebase, after the Workspaces section.
- Hooks: `pr-sync`, `pr-filter`, `pr-list`, and one `pr-row-<number>` per row.

## Anatomy

### Section header

- Title **Open pull requests** with a count of all PRs.
- On the right:
  - Sync status: a PR icon and `Synced from GitHub 2m ago`.
  - A segmented filter: **All**, **Review requested**, **Yours**, each with a count.

### PR list

A single bordered list (not cards), one row per PR, so it reads as a feed and doesn't compete visually with the workspace cards.

### PR row

Left to right:

1. **PR icon.** Muted when the PR is a draft. Screen-reader text: `Open pull request` or `Draft pull request`.
2. **Main column.**
   - Title line: the PR title, then `#482` in a subtler style.
   - Meta line, separated by spacing only:
     - **Relationship**: `Review requested` (emphasised), `Opened by you`, `Mentioned`, or `Assigned to you`.
     - **Author**: `@maya-r`. Omitted when the author is you.
     - **Branches**: `maya/webhook-retry → main`, both in code style.
     - **Size**: `6 files +214 −38`, additions green, deletions red.
     - **Updated**: `Updated 35m ago`. Ages of a day or more show as `Nd ago`.
3. **Signals column.**
   - Checks: `Checks passing` (check icon), `2 checks failing` (x icon), or `Checks running` (clock icon).
   - Review state chip: `Awaiting review` (neutral), `Changes requested` (amber), `Approved` (green), `Draft` (neutral).
4. **Action column.**
   - If no review workspace exists for this PR: secondary **Review in workspace** button with a PR icon. Accessible name: `Review #482 in a workspace`.
   - If one exists: secondary **Go to Workspace** button with a right arrow, which opens it.

## States

### Filter

| Segment | Shows |
| --- | --- |
| All | Every PR |
| Review requested | `rel = review` |
| Yours | `rel = author` |

There is no segment for Mentioned or Assigned; those appear only under All.

### Empty states

| Situation | Heading | Body |
| --- | --- | --- |
| No PRs at all | `No open pull requests` | `Pull requests you open, or that ask for your review, show up here.` |
| None match the filter | `Nothing in this filter` | `Switch the filter to see the rest.` |

## Behaviour

- **Review in workspace** creates a review workspace and navigates straight into it. Full flow in [19](19-review-workspaces.md).
- A PR can only have one review workspace. Once it exists, the row's button becomes **Go to Workspace**, preventing duplicates.
- **PRs created in Harness** (via Create PR, see [18](18-create-pr.md)) are inserted at the top of this list as `Opened by you`, `Checks running`, `Awaiting review`, updated `just now`.
- The sync label's age does not currently tick forward.

## Responsive behaviour

- **Above 1180px:** four columns: icon, main, signals, action.
- **1180px and below:** signals move under the main column on a second row; the action spans both rows on the right.
- **720px and below:** everything stacks in one column, with the action button on its own row, left-aligned.

## Keyboard and accessibility

- The list is a `<ul>`, each row an `<li>`, each title an `h3`.
- The filter is a `role="group"` labelled "Filter pull requests" with `aria-pressed` segments.

## Data model

`state.prs[]`, each with:

| Field | Example | Notes |
| --- | --- | --- |
| `num` | `482` | PR number. |
| `title` | `Retry Stripe webhooks with exponential backoff` | |
| `author` | `maya-r` or `you` | |
| `rel` | `review` / `author` / `mention` / `assigned` | How it involves the user. |
| `head`, `base` | `maya/webhook-retry`, `main` | |
| `updatedMin` | `35` | Minutes since last update. |
| `checks` | `pass` / `fail` / `running` | |
| `failing` | `2` | Count when `checks = fail`. |
| `review` | `required` / `changes` / `approved` / `draft` | |
| `a`, `d`, `files` | `214`, `38`, `6` | Lines added, removed, files changed. |

`state.prFilter`: `all` | `review` | `author`. `state.prSyncedMin`: minutes since sync.

### Sample PRs

| # | Title | Relationship | Checks | Review |
| --- | --- | --- | --- | --- |
| 482 | Retry Stripe webhooks with exponential backoff | Review requested | Passing | Awaiting review |
| 479 | Move the session store to Redis | Review requested | 2 failing | Awaiting review |
| 477 | Add CSV export to the invoices table | Opened by you | Passing | Changes requested |
| 471 | Migrate settings routes to Svelte 5 runes | Mentioned | Running | Draft |
| 468 | Rate-limit the public search endpoint | Opened by you | Passing | Approved |

## Simulated in the prototype

- **All PR data is placeholder.** Nothing is fetched from GitHub. A real build needs: GitHub auth (OAuth app or GitHub App), a query for open PRs where the user is author, requested reviewer, assignee or mentioned (the GitHub search qualifiers `author:@me`, `review-requested:@me`, `assignee:@me`, `mentions:@me`), combined status/check runs, and review decision. It must also handle rate limits and decide a refresh cadence (polling vs webhooks).
- **Which repo a PR belongs to** is not modelled. Every PR is implicitly in `my-app`.

## Known gaps and open questions

- No manual refresh button, and the sync time never changes.
- No link out to the PR on GitHub.
- PRs from repos not registered in Settings: show them, hide them, or offer to add the repo?
- No indication of which repo each PR belongs to, which matters once there are several repos.
- No Mentioned or Assigned filter.
- For PRs the user authored, "Review in workspace" is the only action. A "Continue in workspace" (check out and keep building, for example to address requested changes on #477) is probably more useful than reviewing your own PR.
- Should PRs that already have a building workspace (the user's own) link to that workspace?
