# Harness feature specs

One document per feature of the Harness prototype (`index.html`). Each spec describes what the prototype does today, down to copy, states, timings and keyboard behaviour, and ends with the gaps and open questions found while writing it.

Product context lives in [`PRODUCT.md`](../PRODUCT.md). Where a spec and the prototype disagree, the prototype is the source of truth until the spec is updated.

## How to read these

Every spec follows the same outline:

1. **Summary**: the feature in two or three sentences.
2. **Why it exists**: the user problem and the product principle it serves.
3. **Where it lives**: the screen, entry points, and the `data-od-id` hooks in the prototype.
4. **Anatomy**: every visible element, in reading order.
5. **States**: each state the feature can be in and what the user sees.
6. **Behaviour**: step-by-step flows, including timings.
7. **Rules and edge cases**: validation, locks, conflicts.
8. **Copy**: exact user-facing strings.
9. **Keyboard and accessibility**.
10. **Data model**: the fields the prototype reads and writes.
11. **Simulated in the prototype**: what is faked and what a real build must do instead.
12. **Known gaps and open questions**.

## Terminology

| Term | Meaning |
| --- | --- |
| Repo | A local folder containing a git repository, registered in Settings. |
| Workspace | One git worktree on its own branch, created from a repo, holding one or more threads. Named in plain language ("Auth session timeout"). |
| Thread | One agent conversation inside a workspace. The first thread is the **Lead** (or **Reviewer** / **Planner**). |
| Engine | The agent CLI a thread runs on: Claude Code, Cursor CLI, Codex CLI or Gemini CLI. |
| Homebase | The overview screen: workspace cards and open pull requests. |
| Approval | A request from an agent to perform a risky action, which the user approves or denies. |
| Findings | The output of a review: issues grouped as Blocking, Suggestions and Nits. |
| Teardown | Removing a workspace's worktree, optionally deleting its branch. |

## Specs

### App frame
- [01 · App shell and sidebar](01-app-shell-and-sidebar.md)
- [02 · Command palette and keyboard shortcuts](02-command-palette-and-shortcuts.md)
- [03 · Toasts and system feedback](03-toasts-and-feedback.md)

### Homebase
- [04 · Homebase and workspace cards](04-homebase-workspace-cards.md)
- [05 · Open pull requests](05-open-pull-requests.md)

### Workspace lifecycle
- [06 · New workspace](06-new-workspace.md)
- [07 · Worktree provisioning](07-worktree-provisioning.md)
- [08 · Teardown](08-teardown.md)

### Inside a workspace
- [09 · Workspace header](09-workspace-header.md)
- [10 · Thread tabs](10-thread-tabs.md)
- [11 · Agent conversation](11-agent-conversation.md)
- [12 · Composer](12-composer.md)
- [13 · Approvals](13-approvals.md)
- [14 · Changes panel and diff review](14-changes-panel.md)
- [15 · Run app and Output tab](15-run-app-and-output.md)
- [16 · Branch switching](16-branch-switching.md)
- [17 · Pull and rebase](17-pull-and-rebase.md)
- [18 · Create PR](18-create-pr.md)
- [25 · Branch stacks](25-branch-stacks.md)

### Review
- [19 · Review workspaces](19-review-workspaces.md)
- [20 · Review findings](20-review-findings.md)

### Settings
- [21 · Settings page](21-settings.md)
- [22 · Repos and repo config](22-repos.md)

### Integrations
- [26 · ClickUp sprint](26-clickup-sprint.md)

## Cross-cutting rules

These apply to every feature and are not repeated in each spec.

- **Everything is simulated.** No git, GitHub, agent or shell process runs. Timers stand in for work. Each spec has a "Simulated in the prototype" section describing what a real build must do.
- **Sample data is placeholder.** The three workspaces, five PRs, their findings and the four repos are invented. They must not be presented as real users, metrics or GitHub data.
- **Copy style.** Sentence case, plain and specific, no em-dashes, at most one separator (`·`) per metadata line, American spelling.
- **Workspace name versus branch.** The workspace name is always the plain-language title. The branch is shown separately in monospace and never stands in for the name.
- **One primary button per surface.** Each screen and dialog has at most one solid primary button.
- **Reduced motion.** The OS `prefers-reduced-motion` setting and the in-app **Reduce motion** switch both cut animations and transitions to effectively zero.
- **Focus.** Every interactive element has a visible `:focus-visible` ring. After a re-render, focus returns to the equivalent element when it still exists.
