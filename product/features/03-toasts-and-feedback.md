# 03 · Toasts and system feedback

## Summary

Short-lived toast notifications confirm actions that happen away from where the user is looking, or that complete after a delay: a workspace was created, an app started, a review finished, a branch was deleted. Toasts never carry the only copy of important information; the same outcome is also written to the relevant thread or screen.

## Why it exists

Many Harness actions finish asynchronously (provisioning, starting an app, drafting, reviews). The user may have moved on. Toasts confirm the result without interrupting.

## Where it lives

- Container: `<div class="toasts" id="toasts" role="status" aria-live="polite">`, fixed to a screen corner.

## Anatomy

Each toast is one row: an icon and a single line of text, which may include inline `code` for branches and ports.

## Tones

Each toast has a solid background in its tone's colour, the same in light and dark themes.

| Tone | Background | Icon | Used for |
| --- | --- | --- | --- |
| `ok` | Green, white text | check-circle | Successful completion. |
| `bad` | Red, white text | trash | Failures and destructive outcomes (teardown). |
| default | Yellow, dark text | layers | Neutral information and notices. |

## Behaviour

- A toast appears immediately, stays for **3.6 seconds**, then plays a 160ms exit and is removed.
- Multiple toasts stack; there is no maximum and no deduplication.
- Toasts are not dismissible and not interactive.

## Complete list of toasts

| Trigger | Text | Tone |
| --- | --- | --- |
| Workspace created | `Created <name> on <branch> from <base>` | ok |
| Review workspace opened | `Opened a review workspace for #<num>` | ok |
| Review finished | `Review of #<num> finished · <N findings>` | ok |
| Findings sent | `Sent <N findings> to <thread>` | ok |
| Review submitted | `Submitted your review on #<num>` | ok |
| PR created | `Opened PR #<num> for <branch>` | ok |
| New thread started | `Started a new thread in <name>` | ok |
| Pull | `Pulled <N commits> from <base>` | ok |
| Rebase | `Rebased <branch> onto <base>` | ok |
| Branch switched | `Switched <name> to <branch>` | ok |
| App running | `<name> is running on localhost:<port>` | ok |
| App stopped | `Stopped the app in <name>` | default |
| App restarting | `Restarting the app in <name>` | default |
| Teardown | `Tore down <name>` or `Tore down <name> and deleted <branch>` | bad |
| Repo added | `Added <repo>. Add its setup and run commands below.` | default |
| Repo removed | `Removed <repo>. The folder on disk wasn't touched.` | default |
| Approval: review suggestions sent | `Review suggestions sent to Lead` | ok |
| Approval: legacy file deleted | `Deleted legacy-cookie.ts in worktree` | ok |

## Keyboard and accessibility

- The container is a polite live region, so screen readers announce each toast without interrupting.
- Toasts receive no focus.

## Simulated in the prototype

Toasts themselves are real UI; the events they report are simulated.

## Known gaps and open questions

- No OS-level notifications. For a supervision tool, a desktop notification when an agent needs approval or a review finishes (while Harness is in the background) is probably essential.
- No notification history or inbox.
- Toasts can't be clicked to jump to the workspace they refer to.
- 3.6 seconds may be too short for users relying on screen magnification. Consider pausing on hover.
- "Needs approval" events currently produce no toast at all; they only surface via badges.
