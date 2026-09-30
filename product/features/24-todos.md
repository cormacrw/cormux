# 24 · TODOs

## Summary

A plain list of title-only tasks. **TODOs** sits in the sidebar directly under **Homebase** and opens its own page. Any task can be pinned, and pinned tasks show as cards at the very top of Homebase, above Scratches.

## TODOs page

- Title **TODOs** with a count pill. The sidebar item shows the same count when it is not zero.
- One table: a pin column, the task title, and a delete column.
- The last row is always an inline **Add a task** field, focused when the page opens. Enter adds the task and keeps focus there, so several can be typed in a row. Blank input is ignored. Escape clears the field.
- **Pin** (left of each title) toggles whether the task shows on Homebase. Unpinned pins are muted and tilted; pinned ones are solid.
- **Delete** (trash, right) removes the task at once, with no confirm. It shows on row hover or keyboard focus, and always on touch.
- New rows fly in, deleted rows collapse, and the rest slide into place.

## Homebase

- Section **Pinned tasks** with a count pill, first under the Homebase header. It only renders while at least one task is pinned, and slides open and closed.
- One-line cards in a grid (`minmax(240px, 1fr)`). The title opens the TODOs page; the pin button unpins, and the card scales out.

## Command palette

- **Go to TODOs** opens the page. **Add a task** (meta `todo`) switches the palette into todo mode.
- Typing `todo` then Space, Tab or Enter does the same: the word becomes a **TODO** chip in the search field and the field clears for the task title. Pasting `todo <title>` goes straight there with the title filled in.
- In todo mode the list shows one row, **Add “<title>”**. Enter adds the task, closes the palette and shows the toast `Added “<title>” to TODOs`. Backspace on an empty field turns the chip back into the text `todo`.

## Storage

`todos` table (migration 009): `id`, `title`, `pinned`, `created_at`. Listed oldest first in the snapshot. The UI updates first and rolls back with a toast if the core refuses the change (`create_todo`, `delete_todo`, `set_todo_pinned`).

## Reduced motion

Svelte transitions are driven by JS, so they check the **Reduce motion** setting and the OS `prefers-reduced-motion` query themselves and use zero durations when either is on.
