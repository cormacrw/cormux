# 20 · Review findings

## Summary

When a review finishes, a **Findings** tab appears in the review workspace. It lists every issue the Reviewer found, grouped as **Blocking**, **Suggestions** and **Nits**. The user selects which to fix and sends them to a thread, which works through them and marks each one fixed.

## Why it exists

Review output is only useful if acting on it is easy. Grouping by severity lets the user triage at a glance; selection turns "fix these three" into one click instead of copying text into a chat. Serves **Drafted, never presumed**: the agent proposes findings, the user chooses what gets acted on.

## Where it lives

- Tab `thread-tab-findings` in the tab bar, after the thread tabs (see [10](10-thread-tabs.md)).
- Panel `<section class="thread" id="findings-panel" data-od-id="findings">`, with body `#findings-body` and send bar `#findings-bar` (`findings-send-bar`).
- Also reached from the **Open findings** button on the Review findings card in the Reviewer's conversation.
- Hooks: `findings-intro`, `findings-blocking`, `findings-suggestion`, `findings-nit`, `finding-<id>`, `findings-target`, `findings-send`.

## Anatomy

### Tab
List icon, `Findings`, and a count badge of findings still open (not yet sent). No badge when all are sent.

### Intro
- Engine mark, heading `Review findings`, subtitle `#482 · reviewed by Reviewer with Claude Code`.
- **Summary row:** one item per severity with a coloured dot and a count, for example `2 blocking`, `2 suggestions`, `2 nits`.
- **Quick select** (right of the summary): label `Select`, and ghost buttons **Blocking**, **All**, **None**. Blocking and All are disabled when nothing is open; None is disabled when nothing is selected.

### Groups
One section per severity that has findings, in this order:

| Group | Dot | Note |
| --- | --- | --- |
| Blocking | Red | `Fix before this merges` |
| Suggestions | Amber | `Worth doing in this PR` |
| Nits | Grey | `Optional polish` |

Group header: a select-all checkbox (checked when all open items are selected, indeterminate when some are, disabled when none are open), the dot, the group name, a count of all findings in the group, and the note.

### Finding row
The entire row is a clickable label:

- Checkbox.
- **Title**, for example `Retries can process the same event twice`.
- **Location** in monospace: `src/lib/stripe/retry.ts:48` (line omitted when not applicable).
- **Explanation**: one or two sentences describing the problem and the fix.
- **Status chip** on the right:
  - Open: none.
  - Sent: `Sent to <thread>` with a spinner (accent).
  - Fixed: `Fixed by <thread>` with a check (green).
- Selected rows are tinted with the accent's soft background. Sent and fixed rows are dimmed, their checkbox checked and disabled.

### Send bar (pinned to the bottom, like the composer)
- **Selection summary** (live region): `3 findings selected · 2 blocking`, or `Select the findings you want fixed`.
- **Send to** select, only when the workspace has more than one thread, listing thread roles.
- Primary button: `Send 3 to Reviewer` (arrow icon), or disabled `Send to agent` when nothing is selected.
- When every finding has been sent: `Every finding has been sent. Follow the fixes in the Reviewer thread.` and a secondary **Go to Reviewer** button.
- When the review found nothing: `No findings to send.`

### Empty state
`Nothing to fix` / `Reviewer didn’t flag anything in this pull request.`

## Behaviour

### Selecting
- Clicking a row or its checkbox toggles it.
- A group checkbox selects or clears every *open* finding in that group.
- Quick select: **Blocking** selects exactly the open blocking findings (and clears the rest); **All** selects every open finding; **None** clears all.
- Blocking findings are selected by default when the review completes.
- After each change the panel re-renders, keeping scroll position and focus on the control you used.

### Sending
1. Only runs if at least one open finding is selected.
2. Each selected finding becomes **sent** to the target thread (default: the first thread).
3. In the target thread:
   - User message: `Fix the 3 review findings I selected. Keep each fix small, add a test where the finding asks for one, and don’t push yet.`
   - Step **Received 3 findings from the review** (list icon), detail = the finding titles joined with `·`.
   - Status running, activity `Fixing 3 findings…`, live steps = `<title> · <file>` for each finding.
4. Toast: `Sent 3 findings to Reviewer`.
5. After **2.6 seconds + 0.7 seconds per finding**:
   - Each becomes **fixed**.
   - Step **Fixed 3 findings** (success), detail = the affected file names, chips `Tests pass` and `Committed locally, not pushed`.
   - Thread idle, activity `Fixes ready`.

The user can send in several batches, to different threads.

### Navigation
- Opening Findings from the tab keeps focus on the tab; from the card's button, focus moves to the `Review findings` heading. The panel scrolls to the top either way.
- If a workspace somehow has no findings while the Findings tab is selected, it falls back to the thread.

## Sample findings

| PR | Blocking | Suggestions | Nits |
| --- | --- | --- | --- |
| #482 Stripe retries | 2 (duplicate processing; no backoff ceiling) | 2 | 2 |
| #479 Redis sessions | 3 (TTL not refreshed; outage signs everyone out; tests fail in CI without Redis, which explains the 2 failing checks) | 2 | 2 |
| #477 Invoice CSV | 1 (cells not escaped / formula injection) | 2 | 1 |
| #471 Svelte 5 runes | 1 ($effect loop) | 2 | 1 |
| #468 Search rate-limit | 1 (shared bucket behind proxy) | 1 | 1 |

All placeholder content.

## Keyboard and accessibility

- Each group is a `<section>` labelled by its heading.
- Row checkboxes are described by the explanation text (`aria-describedby`).
- Group checkboxes have `aria-label="Select all open blocking"` (and so on) and use the native `indeterminate` state.
- Quick select is a `role="group"` labelled "Quick select".
- The selection summary is a polite live region.
- The Findings tab takes part in arrow-key tab navigation.

## Data model

```
workspace.findings = {
  target: threadIndex,
  items: [{
    id: '482-0', sev: 'blocking' | 'suggestion' | 'nit',
    title, file, line | null, body,
    sel: boolean,
    state: 'open' | 'sent' | 'fixed',
    to?: threadRole
  }]
}
```

## Simulated in the prototype

- Findings are fixed per PR. A real build needs structured output from the reviewing engine (severity, file, line, title, explanation), validated before display.
- Fixing is a timer. A real build sends the selected findings as a structured instruction to the target thread and tracks completion per finding. That needs the agent to report which finding each commit addresses, or the user to confirm.

## Known gaps and open questions

- Findings can't be dismissed, edited, re-classified or commented on.
- No link from a finding to the diff at its line (the Changes panel is empty in review workspaces).
- **Findings don't flow into Submit review.** Unsent findings should become draft review comments.
- "Fixed" is asserted by the agent. There's no verification step or view of the fix's diff.
- No way to add your own finding.
- No re-run of the review after fixes.
- Sent findings can't be recalled if the user changes their mind.
- Should the user be able to send findings to a brand new thread (for example "New thread: Fixer") from the Send to menu?
