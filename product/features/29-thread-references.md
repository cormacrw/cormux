# 29 · Thread references

## Summary

A thread's agent can pin the things its work is about (a pull request, an issue, a ticket, a doc) to a **References** panel docked beside the conversation, so they stay one click away instead of scrolling past in the prose.

## Experimental flag

Off by default. **Settings › Experimental › References** turns it on, stored as the `experimentalReferences` setting.

- Off: the panel is hidden, Claude sessions start with `mcp__cormux__attach_entity` in `--disallowedTools` and without the prompt sentence about it, and a call that still arrives is refused. Entities already saved are kept.
- The panel follows the switch at once. The tool and prompt are fixed when a session starts, so an agent already running keeps what it started with until its next session.

## The tool

Claude threads get `attach_entity` on the in-process `cormux` MCP server (`src-tauri/src/mcp.rs`), next to the orchestration tools, and the system prompt asks them to use it whenever the work involves something with a URL.

- Arguments: `name` (a short label, like "PR #42: Fix login redirect") and `url` (http or https only; anything else is refused).
- It runs without an approval: it only writes to Cormux's own database.
- The entity belongs to the calling thread, taken from the session the call came through, never from an argument.
- Attaching a URL the thread already has renames it instead of adding a second row.

Entities live in `thread_entities` (migration 022), are deleted with their thread, and reach the webview in the snapshot as `persisted.entities`.

## The panel

- Shown only while the open thread has at least one entity, listed oldest first.
- Each row shows an icon and a source line read from the URL (`src/lib/thread/entities.ts`): GitHub pull requests and issues show their repo, trackers (Linear, Jira, ClickUp and others) and docs (Google Docs, Notion, Figma, Confluence) their host. Clicking a row opens it in the browser.
- It docks in the right-hand gutter beside the 760px conversation column, above the prompt bar. When the gutter is too narrow, the conversation and prompt bar move left to make room. Below a 900px thread pane it hides, and the conversation keeps the space.

## Not yet

- No way to remove an entity, from the panel or by the agent.
- No panel on narrow windows, and no toggle to hide it.
