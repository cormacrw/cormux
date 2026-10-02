# 26 · ClickUp sprint

## Summary

A **Sprint** page shows the team's current ClickUp sprint as a Kanban board, one lane per status. Cards can be dragged between lanes to change status, and a resizable pane on the right shows the selected task's details, where sprint points can be set. Homebase lists the user's own in-progress tasks under Open pull requests. Nothing ClickUp-related shows until an API key is saved in Settings.

## Setup (Settings › ClickUp)

- **API key**: a personal token (`pk_…`) from ClickUp › Settings › Apps. **Save** checks it against `GET /user` first, so a bad key fails with `ClickUp rejected the API key` and is never stored. The key goes to the macOS Keychain (service `cormux`, user `clickup-api-key`) and never reaches the webview. **Remove** deletes it and hides every ClickUp surface.
- **Workspace**, **Space**, **Sprint folder**: pickers that appear once a key is stored. Picking a parent clears its children. With only one option, it's picked automatically; a folder whose name contains "sprint" is preferred when it's the only one.

### How the current sprint is found

ClickUp's API has no sprint endpoint. With the Sprints ClickApp, each sprint is a **List** inside a Sprint folder, and the list's `start_date` and `due_date` are the sprint's dates. The board uses the list in the chosen folder whose dates include today. Between sprints it keeps the latest one that has started; before the first, it takes the earliest upcoming one.

## Sprint page

- Sidebar item **Sprint** (under TODOs), with a count of the user's in-progress tasks. Palette: **Go to sprint** (meta `clickup`).
- Header: the sprint's name, `Sep 22 – Oct 5 · 4 days left`, total points, a warning chip `N tasks without points`, and **Refresh**.
- Lanes follow the list's statuses in ClickUp's order. Each lane shows its task count and points. Tasks come from `GET /list/{id}/task` with `include_timl` (sprint tasks usually live in another home list), `subtasks` and `include_closed`.
- Cards show the custom ID, title and points. **Unpointed cards are highlighted** with an amber border, tint, left stripe and a `No points` chip.
- **Drag** a card onto another lane to change its status. Drags are pointer-based, so Tauri's native file-drop handling doesn't interfere. A drag starts after 4px of travel so a click still selects, the board scrolls when the pointer nears its edge, and Escape cancels. The move applies at once and rolls back with a toast if ClickUp refuses it.
- **Click** a card to open the details pane, and click it again to close. The pane's width is saved (`autoSaveId`). Escape closes it and returns focus to the card.

## Task pane

Custom ID and list, **Open in ClickUp**, close. Then the title, a **Status** select (the keyboard alternative to dragging), **Sprint points** (1, 2, 3, 5, 8, 13 or any other value; a `Not estimated yet` warning while unset), assignees, priority, due date, tags, the markdown description (sanitised, with links opening in the browser) and who created it. Points use `PUT /task/{id}` `{ points }`, status uses `{ status }`, and both update the board optimistically.

## Homebase

Section **In progress** under Open pull requests, shown only with a key. It lists tasks in the current sprint that are assigned to the key's owner and sit in a status of ClickUp type `custom`, which covers everything between to do and done (so "in progress" and "in review" both count). Rows show the custom ID, title, status and points, or `No points`. Clicking one opens the Sprint page with that task selected.

## Refresh

On boot, on opening the page, on window focus, every 2 minutes and from **Refresh**. A refresh that started before a drag or points change is discarded, so it can't undo the change. After a failed refresh, the last board stays up with a notice.

## Not yet

- Sprint Points per assignee.
- Creating tasks, or moving them between sprints.
- Avatars: the CSP blocks remote images, so assignees show as initials.
