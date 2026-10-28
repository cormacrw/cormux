# 28 · Task draft cards

## Summary

When an agent scopes work in a conversation before creating it, the result shows as a hierarchy under the reply instead of only as prose: the epic and its scope first, then the tasks that deliver it, then the subtasks inside each task. This is opt-in through the `cormux-task-cards` skill shipped in the repo (`skills/cormux-task-cards/`); the README covers installing it into `~/.claude/skills`. Cormux never installs it, and without it nothing changes.

## The hierarchy

Three levels, tracker-neutral so any task-creation skill can produce it:

- **Epic**: `outcome` (the change the work exists to produce), `scope` (`in` / `out` lists), and `acceptance` (how anyone can tell the epic is done).
- **Task**: an independently deliverable, testable slice of the epic, with its own `acceptance`.
- **Subtask**: a concrete step inside a task.

The skill scopes top-down: epic and its boundaries before any task, tasks before subtasks, and small work may skip the epic rather than invent one. Estimates sit at the leaves: a task with subtasks carries no points, so the tree never double-counts, and epics are never estimated.

## How it renders

- The conversation parses the block on render (`splitTaskCards` in `src/lib/tasks/task-cards-block.ts`), hides the raw JSON, and shows a **Scoped work** summary (epics, tasks, subtasks, total points, and how many leaf tasks have no points), then one section per epic: outcome, in/out scope, **Done when** list, and its tasks as divided rows with subtasks nested under a rule. Flat task lists (the skill's first format) still render, ungrouped.
- Parsing is tolerant: nesting may use `children`, `tasks`, `stories` or `subtasks`; `name`/`body`/`tags`/`acceptanceCriteria` aliases and string points are accepted, as with the Reviewer's `<cormux-findings>` block. Anything past the third level is dropped.
- The view is display-only. Its only action is opening a node's `url` when the created task has one; editing and creating stay in chat with the agent.

## Graceful degradation

- No block: the reply renders exactly as before. The skill's absence breaks nothing.
- A block still streaming (or a partial opening tag) is hidden; the hierarchy appears once it completes.
- A complete but unparseable block stays visible in the prose, so malformed work is never silently lost. The skill also requires the scoping to exist outside the block, so it is never the only copy.
