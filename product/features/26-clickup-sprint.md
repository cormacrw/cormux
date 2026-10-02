# 26 · ClickUp sprint

## Summary

A **Sprint** page shows the team's current ClickUp sprint as a Kanban board, one lane per status. Cards can be dragged between lanes to change status, and a resizable pane on the right shows the selected task's details, where sprint points can be set. Nothing ClickUp-related shows until an API key is saved in Settings.

## Setup (Settings › ClickUp)

- **API key**: a personal token (`pk_…`) from ClickUp › Settings › Apps. **Save** checks it against `GET /user` first, so a bad key fails with `ClickUp rejected the API key` and is never stored. The key goes to the macOS Keychain (service `cormux`, user `clickup-api-key`) and never reaches the webview. **Remove** deletes it and hides every ClickUp surface.
- **Workspace**, **Space**, **Sprint folder**: pickers that appear once a key is stored. Picking a parent clears its children. With only one option, it's picked automatically; a folder whose name contains "sprint" is preferred when it's the only one.

### How the current sprint is found

ClickUp's API has no sprint endpoint. With the Sprints ClickApp, each sprint is a **List** inside a Sprint folder, and the list's `start_date` and `due_date` are the sprint's dates. The board uses the list in the chosen folder whose dates include today. Between sprints it keeps the latest one that has started; before the first, it takes the earliest upcoming one.

## Sprint page

- Sidebar item **Sprint** (under TODOs), with a count of the sprint's open tasks: every task not in a status with ClickUp type `done` or `closed`. Palette: **Go to sprint** (meta `clickup`).
- Header: the sprint's name, `Sep 22 – Oct 5 · 4 days left`, a progress meter of points done out of total points (done means a `done` or `closed` status; unpointed tasks don't count), a warning chip `N tasks without points`, **Lanes** and **Refresh**.
- Lanes follow the list's statuses in ClickUp's order. Each lane shows its task count and points. Tasks come from `GET /list/{id}/task` with `include_timl` (sprint tasks usually live in another home list), `subtasks` and `include_closed`.
- **Hide** a lane from the eye button in its header (shown on hover or focus), or toggle lanes from the header's **Lanes** menu, which counts hidden lanes and offers **Show all lanes**. Hidden statuses are saved by name in the `clickupHiddenStatuses` setting, so they stay hidden across sprints and restarts. The header meter and warning still cover the whole sprint.
- Cards lead with the title, then a footer with the task's ID, a priority flag, assignee initials and a points pill. The ID is the custom ID, or ClickUp's own when the workspace has no Custom Task IDs. **Unpointed cards are highlighted** with a dashed amber outline, an amber tint and a dashed `No pts` pill.
- Hovering or focusing a card shows **Copy ID** and **Open in ClickUp** buttons. They don't start a drag or select the card.
- **Drag** a card onto another lane to change its status. Drags are pointer-based, so Tauri's native file-drop handling doesn't interfere. A drag starts after 4px of travel so a click still selects, the board scrolls when the pointer nears its edge, and Escape cancels. The move applies at once and rolls back with a toast if ClickUp refuses it.
- **Click** a card to open the details pane, and click it again to close. The pane's width is saved (`autoSaveId`). Escape closes it and returns focus to the card.

## Task pane

A header with the ID as a one-click **copy** button, **Open in ClickUp** and close. Then the title, the status as a read-only pill (status changes by dragging), **Sprint points** (1, 2, 3, 5 or 8; a caution sign by the label while unset), assignees, priority, due date, tags, the description rendered as rich markdown (headings, lists, tables, strikethrough, checklists and code; sanitised, with links opening in the browser) and who created it. Points use `PUT /task/{id}` `{ points }` and status uses `{ status }`. Both update the board at once, and the board reloads shortly after so it picks up anything ClickUp's automations did in response.

## Refresh

On boot, on opening the page, on window focus, every 2 minutes and from **Refresh**. A refresh that started before a drag or points change is discarded, so it can't undo the change. A refresh asked for while another is running is queued, not dropped. After a failed refresh, the last board stays up with a notice.

## Not yet

- Sprint Points per assignee.
- Creating tasks, or moving them between sprints.
- Avatars: the CSP blocks remote images, so assignees show as initials.
