# 04 · Homebase and workspace cards

## Summary

Homebase is the landing screen. It shows every workspace as a card with a single, glanceable status ("2 agents working", "Idle" or "Needs Attention"), a model-written summary of where the work stands, and a button to open it. Below the cards is the Open pull requests list (see [05](05-open-pull-requests.md)).

## Why it exists

The user needs one place to see all parallel work and decide where to spend attention next. Cards compress each workspace to: what is it, is it waiting on me, and what has happened so far.

## Where it lives

- `<main class="view" id="view-home">`. Default view on launch.
- Entry points: app launch, sidebar **Homebase**, palette **Go to Homebase**, and automatically after creating a workspace or tearing down the open one.
- Hooks: `home-title`, `open-prs`.

## Anatomy

### Header row

- **Homebase** as the page title (`h1`), left-aligned. It receives focus when you navigate to Homebase.
- **New Workspace** primary button on the right, with a plus icon and `⌘N` hint. This is the only solid primary button on the screen.

There are no summary stats (workspace counts, agents working, approvals waiting). They were removed on purpose; the cards and sidebar carry that information.

### Workspaces section

- Section title **Workspaces** with a count of all workspaces.
- A segmented filter on the right: **All**, **Running**, **Idle**, each with a count.
- A responsive card grid: columns fill the width with a minimum card width of 380px (or 100% on narrow screens).

### Workspace card

In reading order:

1. **Title**: the workspace name (for example `Auth session timeout`) in the regular sans font, truncated with an ellipsis if too long. Never the branch.
2. **Status badge** on the right of the title (see States).
3. **Details line**, items separated by spacing only (no bullets):
   - `Reviewing #482` (review workspaces only, number in code style)
   - `Branch: feat/auth` (branch in code style)
   - `3 agents`
   - `3 modified files`
   - `Created 12m ago`
4. **Summary block**: up to three lines of the summary, then a source line with a small speech icon: `Summarized by Haiku 4.5, 2m ago`. While there is no summary yet, two shimmering skeleton lines and `Summarizing with Haiku 4.5…` are shown, and the block is `aria-busy`.
5. **Go to Workspace** secondary button with a right arrow.

## States

### Card status badge

Evaluated in this order; the first match wins:

| Condition | Badge text | Colour |
| --- | --- | --- |
| Any thread in the workspace has a pending approval | `Needs Attention` | Amber |
| One or more threads are running or provisioning and not paused | `1 agent working` / `N agents working` | Green, with pulsing dot |
| Otherwise (including all threads paused) | `Idle` | Grey |

### Filter

- **All** shows everything.
- **Running** shows workspaces whose status is anything other than `idle` (so provisioning workspaces count as running).
- **Idle** shows workspaces with status `idle`.
- Counts on each segment are always calculated from the full list.
- Creating a workspace or opening a review workspace resets the filter to **All** so the new card is visible.

Note: the filter uses the *workspace's own* status (the Lead thread), while the badge considers *all* threads. A workspace can show `2 agents working` on its badge and still appear under **Idle** if its Lead is idle and a secondary thread is running.

### Empty states

| Situation | Heading | Body | Action |
| --- | --- | --- | --- |
| No workspaces at all | `No workspaces running` | `Create one to hand a task to an agent in its own worktree. Homebase stays untouched until you merge.` | Secondary **Create a workspace** button (secondary so it doesn't compete with the header's primary). |
| Workspaces exist but none match the filter | `Nothing in this filter` | `Switch the filter to see your other workspaces.` | Secondary **Show all** button. |

## Behaviour

- **Card entry animation:** only newly added cards fade in. Existing cards do not replay their animation when anything else changes.
- **Card exit:** when a workspace is torn down while Homebase is visible, its card fades and scales to 98% over about 150ms before it's removed.
- **Card hover:** the border strengthens. Cards themselves aren't clickable; only the button is.
- **Timestamps:** every 60 seconds, each workspace's "Created" age and its summary age increase by one minute, and Homebase re-renders if it's visible.
- **Age format:** `just now` under a minute, `Nm ago` under an hour, `Nh ago` after that.

## Copy

- Page title: `Homebase`
- Primary: `New Workspace` (title case, matching the dialog's `Create Workspace`)
- Card button: `Go to Workspace`
- Summary source: `Summarized by Haiku 4.5, <age>` / `Summarizing with Haiku 4.5…`

## Keyboard and accessibility

- Each card is an `<article>` labelled by its title.
- The filter is a `role="group"` labelled "Filter workspaces"; each segment uses `aria-pressed`.
- The Homebase heading is focusable (`tabindex="-1"`) so navigation can move focus to it.

## Data model

Per workspace: `id`, `title`, `branch`, `kind` (`review` or absent), `pr`, `status`, `paused`, `extra[]`, `files[]`, `createdMin`, `summary`, `summaryMin`, and each thread's `timeline` for pending approvals.

`state.filter`: `all` | `running` | `idle`.

### Sample workspaces

| Name | Branch | Engine | State at launch |
| --- | --- | --- | --- |
| Auth session timeout | `feat/auth` | Claude Code | Running. Three threads (Lead, Test writer, Reviewer). Three pending approvals. App already running on port 5173. |
| Stripe webhook parsing | `fix/stripe` | Cursor CLI | Idle with a plan awaiting approval. |
| Lazy-load dashboard | `perf/lazy` | Claude Code | Provisioning; finishes about 7 seconds after launch. |

## Simulated in the prototype

- **Summaries** are hand-written strings. The label claims Haiku 4.5. A real build must call a small, fast model on thread and diff activity and store the result with a timestamp, and decide how often to refresh (on every significant event, or on a debounce).
- **Created age** and summary age tick forward on a timer rather than being derived from real timestamps.

## Known gaps and open questions

- No sort control (by recent activity, needs attention first, name).
- The Running/Idle filter and the badge use different rules (see Filter note). Consider filtering by the badge's three states instead: All, Needs Attention, Working, Idle.
- No per-card quick actions (Pause all, Teardown, Create PR) without opening the workspace.
- No grouping by repo when a user works across several repos.
- The summary model name is shown in the UI. Decide whether users care, or whether "Summary, 2m ago" is enough.
- No indication on the card that the workspace's app is running, or which port.
