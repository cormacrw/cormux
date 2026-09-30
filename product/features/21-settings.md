# 21 · Settings page

## Summary

Settings is a full page (not a popover) with four clearly separated sections on one scrolling page: **Agents**, **Repos**, **General** and **Skills**. A short sticky list on the left jumps between sections and highlights the one you're reading. Changes apply immediately; there's no Save button.

## Why it exists

Defaults matter in a tool used many times a day: which engine new workspaces use, which branch they start from, what agents may do without asking, which repos exist. A single page with clear sections makes these easy to scan and change.

## Where it lives

- `<main class="view" id="view-settings" data-od-id="settings-view">`.
- Entry points: the sliders button at the bottom of the sidebar (`nav-settings`, shown selected while open), palette **Open settings**, and **Add run command** in an Output tab's empty state (which opens Repos with a repo expanded).
- Hooks: `settings-title`, `settings-nav`, `settings-agents`, `settings-repos`, `settings-general`, `settings-skills`, plus one per control (below).

## Layout

- **Title** `Settings` (`h1`, focused on arrival).
- **Section list** (left, sticky): Agents, Repos, General, Skills.
- **Sections** (right), each with a heading, a one-line description, and one or more grouped panels of rows.
- **Narrow windows:** the section list becomes a row of buttons above the content.

## Sections

### Agents
Description: `How new agents start and what they can do without asking.`

**Default engine** (`settings-default-engine`): a radio group of four rows. Each row: the engine mark, name, the engine's hint, a `Default` tag on the current one, and a radio indicator.

| Engine | Hint |
| --- | --- |
| Claude Code | Edits files directly and asks before risky steps |
| Cursor CLI | Writes a plan first and waits for your go-ahead |
| Codex CLI | Works through the task in a sandbox and proposes a diff |
| Gemini CLI | Reads widely across the repo before suggesting changes |

Used as: the pre-selected engine in the New workspace dialog, and the engine for review workspaces.

**Permissions** (`settings-agent-permissions`):
- Switch **Auto-approve read-only tools**, `File reads and AST parsing run without asking. Edits and commands still need approval.` On by default.

### Repos
Description: `Local folders that new workspaces can check out from.` Full detail in [22](22-repos.md).

### Scratch macros
Description: `Saved prompts you can start as a scratch from the command palette.` Laid out like Repos: one row per macro (lightning icon, name, first line of the prompt) with **Configure** and **Remove**, then an **Add a macro** name field. Adding a macro expands it and focuses **Prompt**. Configure shows **Name** and **Prompt**; edits save 350ms after typing stops. A cleared name keeps the previous one. Names must be unique when added. A macro without a prompt is flagged `Not in the palette`. Stored as JSON in the `scratchMacros` settings row. See [23](23-scratches.md#scratch-macros).

### General
Description: `Defaults for new workspaces and how the app behaves.`

| Row | Control | Default | Description |
| --- | --- | --- | --- |
| Default repository | Select of added repos | First repo | `Pre-selected whenever you pick a repo for a new workspace or scratch` |
| Default base branch | Select of the default repository's branches | `main` | `New workspaces branch from here unless you pick another` |
| Teardown after merge | Switch | On | `Delete the worktree once its branch is merged` |
| Worktree location | Read-only path `~/.harness/worktrees` | | `Where each workspace's checkout lives on disk` |
| Reduce motion | Switch | Off | `Turn off pulses and transitions` |

### Skills
Description: `Reusable instructions any agent can load into its thread.`

An honest stub: `No skills yet` / `Skills you add here will be available to every engine. Managing them from this page isn't built yet.`

## Behaviour

- **Section list:** clicking a section smoothly scrolls it into place (instantly with Reduce motion on) and moves focus to the section. While scrolling, the list highlights the section whose top is within 120px of the top of the page; at the very bottom, the last section is highlighted.
- **Default engine:** choosing a row updates the default immediately, moves the `Default` tag, and keeps focus on the chosen radio.
- **Switches** toggle immediately and keep focus.
- **Reduce motion** also applies instantly across the whole app (adds a `reduce-motion` class to the document). The palette has a matching command.
- **Default repository** is pre-selected in the New workspace and New scratch dialogs. If that repo is removed, the first repo is used.
- **Default base branch** applies to the next New workspace dialog.
- Opening Settings while already on it does nothing.

## Keyboard and accessibility

- The section list is a `<nav>` labelled "Settings sections"; the current section is marked `aria-current="true"`.
- Each section is labelled by its heading.
- Default engine is a real radio group (visually hidden native radios), labelled "Default engine"; a focus ring appears on the row when its radio has keyboard focus.
- Switches are buttons with `role="switch"` and `aria-checked`.
- Default base branch has a proper `<label>`.

## Data model

```
state.settings = {
  defaultEngine: 'claude',
  autoApproveRead: true,
  defaultBase: 'main',
  teardownAfterMerge: true,
  reduceMotion: false
}
```

## Simulated in the prototype

- Settings aren't persisted; a reload resets them. A real build stores them (per user, maybe per machine).
- **Auto-approve read-only tools** doesn't change any behaviour.
- **Teardown after merge** has no effect, because there's no merge flow (Create PR replaced Merge).
- **Worktree location** can't be changed.

## Known gaps and open questions

- **Default base branch lists only `my-app`'s branches.** It should be per repo (most repos use `main`, but not all), probably moved into each repo's config.
- **Default engine isn't used for new threads** added with +; they're always Claude Code.
- Engine configuration is missing: whether each CLI is installed, its path and version, sign-in status, model choice, and extra flags.
- Permissions are a single switch. A real permission model (per engine, per repo, allowlists of commands) needs designing.
- **Skills** is a stub. Needs: what a Skill is (a markdown instruction file?), where it's stored, how it's attached to a thread, and whether it syncs with each engine's native skills or rules format.
- No GitHub account section (sign-in, which orgs, PR filters).
- No notifications section (desktop notifications for approvals, finished reviews).
- No keyboard shortcut reference.
- Worktree location should probably be editable, with a warning about existing worktrees.
