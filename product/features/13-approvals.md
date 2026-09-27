# 13 · Approvals

## Summary

Agents stop and ask before risky actions: deleting files, running migrations, executing a plan, passing suggestions to another thread. Each request appears as an approval card in the conversation with a plain description of what will happen and why, and two buttons. Pending approvals are counted on the thread tab, the sidebar and the workspace card ("Needs Attention").

## Why it exists

Agents should move fast on safe work and stop on anything destructive or irreversible. Approvals are the contract between them. The badges make sure a waiting agent is never silently stuck. Serves **Show what needs you** and **Drafted, never presumed**.

## Where it lives

- Inline in the conversation (see [11](11-agent-conversation.md)), as `<div class="approval" data-od-id="approval-<id>">`.
- Surfaced as counts in: the thread tab badge, sidebar workspace row badge, sidebar agent row badge, Homebase card badge (`Needs Attention`), and the palette's workspace meta.

## Anatomy

### Pending

1. **Header:** an alert icon, the title (for example `Delete file`), a `Needs approval` amber chip, and the time.
2. **What:** the target, in monospace, for example `src/lib/auth/legacy-cookie.ts`.
3. **Why:** one line of context, for example `6 lines · 0 remaining imports`.
4. **Actions:** a green pill button with a check icon and the approve label, and a plain pill with the deny label.

### Resolved

- The header icon becomes a check (approved) or an x (denied), and the `Needs approval` chip disappears.
- Actions are replaced by a single chip:
  - Approved: `Approved by you · just now` (green).
  - Denied: `<deny label> · just now`, for example `Keep file · just now` (neutral).
- The card is visually quietened.

## Sample approvals and their effects

| ID | Thread | Title | What | Why | Approve | Deny |
| --- | --- | --- | --- | --- | --- | --- |
| `delete-legacy` | Auth Lead | Delete file | `src/lib/auth/legacy-cookie.ts` | `6 lines · 0 remaining imports` | Approve delete | Keep file |
| `migrate` | Auth Lead | Run database migration | `20260926_add_last_seen_at.sql` | `Adds sessions.last_seen_at (timestamptz, nullable). Reversible.` | Run migration | Skip |
| `review-apply` | Auth Reviewer | Apply review suggestions | `src/hooks.server.ts` | `Skip the cookie refresh for /_app assets and keep ?next= on the login redirect.` | Send to Lead | Dismiss |
| `approve-plan` | Stripe Planner | Execute plan | `4 steps · touches ~2 files` | `Nothing is edited until you approve.` | Approve plan | Revise |

### What happens after a decision

**Delete file**
- Approve: the deleted file is added to the Changes list (status `D`), a `Deleted legacy-cookie.ts` edit step is added, toast `Deleted legacy-cookie.ts in worktree`.
- Deny: `Keeping legacy-cookie.ts. I’ll leave a @deprecated note and move on.`

**Run database migration**
- Approve: step `Applied migration` (database icon) with chip `sessions.last_seen_at added`.
- Deny: `Skipping the migration. I’ll store lastSeenAt in the signed cookie instead, so no schema change is needed.`

**Apply review suggestions** (cross-thread)
- Approve: in the Reviewer thread, step `Sent 2 suggestions to Lead`; the Reviewer's activity becomes `Suggestions sent`; in the *Lead* thread, a thought `Reviewer suggests skipping the cookie refresh…Folding both into hooks.server.ts.`; toast `Review suggestions sent to Lead`.
- Deny: `Dismissed. I’ll leave the diff as it is and keep watching for new changes.`

**Execute plan**
- Approve: the Planner becomes running with activity `Executing plan…` and posts `Plan approved. Starting with step 1: read the raw body before verification.` After 2.2 seconds, the webhook file edit appears in Changes, a `Ran tests: 8 passed` step is added, the activity becomes `Writing regression test…` and the workspace summary updates.
- Deny: `Okay. Tell me what to change in the plan and I’ll rework it before editing anything.` and focus moves to the composer so the user can say what to change.

## Behaviour

- An approval can be decided once. Clicking again after resolution does nothing.
- Decisions only apply within the currently open workspace and selected thread.
- After a decision, focus stays on the card's area (focus restoration looks for the same button; if it's gone, focus stays in the conversation region).
- Pending counts everywhere update immediately.

## Read-only auto-approval

Settings → Agents → **Auto-approve read-only tools** (on by default) describes: "File reads and AST parsing run without asking. Edits and commands still need approval." Read steps show the chip `Read-only, auto-approved`.

## Keyboard and accessibility

- Buttons are real buttons with visible text labels.
- New approvals are announced: `<role> needs approval: <title>, <what>`.
- Badge counts are `aria-hidden`; the count is included in the tab and row `aria-label`s instead (`, 2 approvals waiting`).

## Data model

```
{ kind: 'approval', id, title, what, why, ok, no,
  state: 'pending' | 'approved' | 'denied', doneAt?, t }
```

Effects are keyed by approval `id` in an `EFFECTS` table with `approved` and `denied` handlers receiving the workspace and thread.

## Simulated in the prototype

- The approvals and their effects are scripted. A real build needs each engine's permission-request protocol mapped to this card (Claude Code's permission prompts, Cursor's plan confirmation, and so on), and must send the decision back to the waiting process.
- The **Auto-approve read-only tools** switch changes nothing; the chips are static.

## Known gaps and open questions

- No **Approve all** / **Deny all** for a thread with several pending approvals. (These actions existed earlier without buttons and were removed.)
- No "Always allow this" (for example always allow `pnpm test`) that would build a per-repo allowlist.
- No way to approve with changes (edit the command or plan before approving).
- No undo after deciding.
- Denying gives the agent no reason. Consider an optional "Tell the agent why" field, or focusing the composer after any denial, not just the plan.
- Approvals from background workspaces don't raise a notification.
- The permission model (what counts as risky, per engine and per repo) isn't designed yet.
