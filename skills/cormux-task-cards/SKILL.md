---
name: cormux-task-cards
description: Use whenever work is being scoped, drafted or proposed as tasks or tickets before being created, in any tracker (ClickUp, Linear, GitHub Issues, Jira, or a plain TODO list). It controls how the work is scoped and presented — epics first, then their tasks, then subtasks — and does not change where the tasks are created or the tracker workflow being followed.
---

# Scoping epics and presenting the hierarchy for Cormux

Work is scoped top-down, and Cormux displays it that way: an **epic** with its
scope, the **tasks** that deliver it, and the **subtasks** that break a task
down. Follow this order; do not start by listing implementation tasks.

## 1. Scope the epic first

Before any task exists, state the epic:

- **Outcome**: the change in the world this work produces, in one sentence.
  Not a restatement of the title.
- **In scope / out of scope**: what the epic covers, and — just as important —
  what it deliberately does not. Name the tempting neighbouring work in the
  out list.
- **Done when**: the acceptance conditions anyone can check to call the epic
  finished.

If the work is too small for an epic (one task, no real scope question), skip
the epic and draft the task directly. Never invent an epic to fill the shape.

## 2. Decompose into tasks

Break the epic into tasks that are each:

- **Independently deliverable**: finishing one leaves the product coherent,
  even if the epic is not done.
- **Testable on their own**: each carries its own `acceptance` conditions.
- **Estimated at the leaf**: put `points` on a task only if it has no
  subtasks; otherwise estimate the subtasks. Never estimate the epic, and
  never estimate both a task and its subtasks (Cormux totals the tree).

Subtasks are the concrete steps inside a task (a migration, an endpoint, a
test). Skip them when the task is already small enough to hold in one head.

## 3. Present the hierarchy

Present the scoping as you normally would in prose, then end the message with
the same work as a JSON tree in `<cormux-tasks>` tags:

```
<cormux-tasks>
[
  {
    "kind": "epic",
    "title": "Self-serve onboarding",
    "outcome": "New teams finish setup without a support ticket.",
    "scope": { "in": ["Invite flow", "Workspace defaults"], "out": ["SSO", "Billing"] },
    "acceptance": ["Median setup under 10 minutes in product analytics"],
    "children": [
      {
        "title": "Invite teammates during setup",
        "description": "Admins can invite up to 10 teammates before first login.",
        "acceptance": ["Invite links expire after 7 days"],
        "children": [
          { "title": "Add invite token model", "points": 2 },
          { "title": "Send invite email", "points": 1 }
        ]
      },
      { "title": "Seed workspace defaults", "points": 3, "labels": ["setup"] }
    ]
  }
]
</cormux-tasks>
```

Rules:

- The hierarchy is three levels: `epic` → `task` → `subtask`, carried by
  nesting under `children`. A node without `kind` takes its level's kind, so
  only epics and out-of-place nodes need the label.
- Only `title` is required on any node. Optional fields: `description`,
  `priority`, `points`, `labels`, `status`, and — once a task has actually
  been created — `ref` (the tracker's own ID, such as `ENG-123`) and `url`.
- Keep fields tracker-neutral. Leave out anything the tracker in use does not
  have; do not invent values to fill a field.
- The block is a presentation copy of the work, never the only copy. Keep
  everything needed to create the tasks in the prose or in your working
  context, so nothing is lost where the block is not rendered.
- Emit the block only in the message that presents the drafts (and again,
  with `ref`/`url` filled in, in the message that reports them created).
- Do not mention the tags or the JSON to the user.
