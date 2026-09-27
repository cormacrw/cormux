# 06 · New workspace

## Summary

The New workspace dialog hands a task to an agent in a fresh, isolated worktree. The user picks a repo, a base branch and an engine, and writes an initial prompt. Harness drafts the workspace name and branch name from the prompt as the user types; both remain editable.

## Why it exists

Starting parallel work must be fast and safe. One dialog covers everything needed to create a worktree and start an agent, with sensible defaults so the common case is "type the task, press ⌘↵". It serves **Isolation is non-negotiable** (every task gets its own branch and worktree) and **Drafted, never presumed** (names are suggested, not imposed).

## Where it lives

- `<dialog class="dialog" id="spool-dialog">` (internally called "spool").
- Entry points:
  - **New Workspace** primary button on Homebase.
  - **New workspace** in the sidebar.
  - **Create a workspace** in Homebase's empty state.
  - Palette command **New workspace**.
  - `⌘N` / `Ctrl+N` anywhere.

## Anatomy

### Header
- Title **New workspace**. No description line (removed on purpose).
- Close (×) icon button.

### Body, in order

**Row 1 (two columns):**

1. **Workspace name**
   - Text input, placeholder `OAuth login`, max 48 characters.
   - Hint: `Shown in the sidebar and on Homebase`.
2. **Branch name**
   - Monospace text input, placeholder `feat/oauth-login`, no spellcheck.
   - Hint: `New branch created for this worktree`.
   - Error line below (hidden by default).

**Row 2 (two columns):**

3. **Repository**
   - Select listing every repo from Settings → Repos by name.
   - Hint: `The worktree is created from this repo`.
4. **Base branch**
   - Monospace combobox (typeahead) with a search icon, default value from Settings (`main`).
   - Hint: `Your new branch starts from here`.
   - Error line below (hidden by default). The hint hides whenever the error shows, so the two never stack and this column stays the same height as Repository.

**Row 3 (full width):**

5. **AI engine**
   - Select: Claude Code, Cursor CLI, Codex CLI, Gemini CLI. Defaults to the default engine from Settings.
   - Hint changes with the selection:

   | Engine | Hint |
   | --- | --- |
   | Claude Code | `Edits files directly and asks before risky steps` |
   | Cursor CLI | `Writes a plan first and waits for your go-ahead` |
   | Codex CLI | `Works through the task in a sandbox and proposes a diff` |
   | Gemini CLI | `Reads widely across the repo before suggesting changes` |

**Row 4 (full width, last):**

6. **Initial Prompt**
   - Multi-line textarea, 4 rows (about 112px), user-resizable up to 280px.
   - Placeholder `e.g., Implement OAuth login with Supabase`.
   - Required. Error: `Describe the task so the agent knows where to start.`

Below 760px, both two-column rows collapse to one column.

### Footer
- Left: `Esc to cancel`.
- Right: primary **Create Workspace** button with a `⌘↵` key hint.

## Behaviour

### Opening
1. Any open popover, the palette and any confirmation dialog are closed. If the dialog is already open, nothing happens.
2. The form resets: all fields cleared, engine set to the Settings default (and its hint updated), repo list rebuilt from Settings, repo set to `my-app` if it exists or else the first repo, base branch set to the Settings default.
3. All errors are cleared and the "user edited name/branch" flags reset.
4. The submit button is reset to **Create Workspace**.
5. Focus goes to **Workspace name**.

### Drafting name and branch from the prompt
As the user types in Initial Prompt:

- **Workspace name** is filled with a draft *unless the user has typed in it*. The draft takes whole words from the prompt up to 40 characters, strips trailing full stops and spaces, and capitalises the first letter. Example: `lazy-load the chart widgets on /dashboard so first paint doesn't wait` becomes `Lazy-load the chart widgets on /dashboard`.
- **Branch name** is filled with a draft *unless the user has typed in it*:
  - Prefix `fix/` if the prompt contains fix, bug, broken, error, crash or repair; `refactor/` if it contains refactor, clean or rename; otherwise `feat/`.
  - Slug: lowercase, remove anything that isn't a letter, number, space or dash, take the first five words, join with dashes.
  - Example: `Implement OAuth login with Supabase` becomes `feat/implement-oauth-login-with-supabase`.
- Clearing a field you edited (making it empty) hands it back to the auto-draft.
- Clearing the prompt clears both drafted fields.

### Base branch typeahead
- **On focus:** opens the list showing every branch, highlights the current value and selects the input text.
- **Branch list** contains the selected repo's branches plus branches used by existing workspaces on that repo.
- **Typing** filters the list (case-insensitive substring) and highlights the matching characters.
- **Meta labels** on the right: `default` (main), `integration` (develop), `deploys to staging` (staging), or `workspace` for branches that belong to an existing workspace.
- **Keys:** `↓`/`↑` move, `↵` picks, `Esc` closes the list (and doesn't close the dialog), `Tab` picks the highlighted item if the typed text isn't already an exact branch name.
- **Mouse:** clicking an option picks it.
- **No matches:** `No branches match “<text>”`.
- **On blur:** after 120ms the list closes; if the value isn't a known branch, the error `No branch called “<value>”. Pick one from the list.` appears.
- **Changing the Repository** resets the base branch to `main` and clears its error.

### Validation

| Field | When checked | Rule | Error |
| --- | --- | --- | --- |
| Initial Prompt | Blur after typing; submit | Must not be empty | `Describe the task so the agent knows where to start.` |
| Branch name | Blur (if not empty); submit | Required | `Add a branch name, e.g. feat/oauth-login.` |
| | | Only letters, numbers, `.`, `_`, `/`, `-`; no `//`, no leading or trailing `/`, no `..` | `Use letters, numbers, dashes and slashes only, e.g. feat/oauth-login.` |
| | | Must not already exist | `<branch> already exists. Pick a new branch name.` |
| | | Must not be used by another workspace | `Another workspace already uses this branch.` |
| Base branch | Blur; submit | Must be in the branch list | `No branch called “<value>”. Pick one from the list.` |

Errors clear as soon as the user edits the field.

### Submitting
Triggered by the button, `↵` in a single-line field, or `⌘↵` / `Ctrl+↵` in Initial Prompt (plain `↵` there adds a new line).

1. Validate in order: prompt, then branch (using the draft if the field is empty), then base branch. The first failing field shows its error and gets focus.
2. Workspace name falls back to the draft if empty.
3. The button is disabled and shows a spinner with `Creating worktree…`.
4. After 1 second:
   - A workspace is added at the top of the list with status `provisioning`, activity `Running worktree setup…`, the selected engine and repo, the prompt as its goal, and a single **Lead** thread.
   - The Lead's timeline starts with the prompt as a user message and a `Created worktree` step showing `<branch> ← <base>`.
   - The dialog closes, the Homebase filter resets to All, and the app navigates to Homebase (or re-renders it if already there).
   - Toast: `Created <name> on <branch> from <base>`.
   - Provisioning starts (see [07](07-worktree-provisioning.md)).

The user lands on Homebase, not inside the new workspace, so they can keep spinning up more.

### Cancelling
`Esc` (unless the base branch list is open, in which case Esc closes only the list), the × button, or clicking the backdrop.

## Keyboard and accessibility

- Dialog is labelled by its title.
- Every field has a `<label>` and `aria-describedby` pointing at its hint and error.
- Invalid fields get `aria-invalid="true"`.
- Base branch implements the ARIA combobox pattern: `role="combobox"`, `aria-expanded`, `aria-controls`, `aria-activedescendant`; options are `role="option"` with `aria-selected`.
- The key hint on the submit button is `aria-hidden`.

## Data model

Creates a workspace object:

```
id           'ws' + base36 timestamp
title        workspace name
branch       branch name
repo         selected repo id
base         base branch
engine       'claude' | 'cursor' | 'codex' | 'gemini'
goal         the initial prompt
status       'provisioning'
activity     'Running worktree setup…'
provStep     1
role         'Lead'
extra        []
files        []
summary      null (filled when provisioning finishes)
timeline     [user prompt, Created worktree, live]
```

Reads: `REPOS`, `state.settings.defaultEngine`, `state.settings.defaultBase`, `state.workspaces` (for branch conflicts).

## Simulated in the prototype

- **Name and branch drafting** is a local heuristic. A real build could keep the heuristic for instant feedback and optionally refine with a small model.
- **Worktree creation** is a 1-second timer. A real build runs `git worktree add -b <branch> <path> <base>` in the selected repo, surfacing git errors (dirty state, branch exists remotely, disk full) in the dialog.
- **Branch existence** is checked against `my-app`'s branch list only.

## Known gaps and open questions

- **Branch conflict check uses the wrong list.** "Already exists" is checked against `my-app`'s branches regardless of the selected repo. It should use the selected repo's branches (and ideally remote branches too).
- **Changing repo resets base to `main`,** not to the Settings default base branch, and doesn't check the new repo actually has `main`.
- Base branch must exist locally. Decide whether remote-only branches (`origin/...`) should be offered.
- No way to attach files, images or links to the initial prompt.
- No way to start a workspace with several threads at once, or with a Skill.
- No template or recent-prompt reuse.
- After creating, the user stays on Homebase. Some users will expect to land in the workspace. Consider a setting or a "Go to workspace" action on the toast.
- The prompt can't be edited after creation except by messaging the agent.
