# 22 · Repos and repo config

## Summary

Settings → **Repos** manages the local repositories Harness can create workspaces from. Each repo is identified by its folder path. Each repo also has two pieces of config: **Worktree setup** (commands run in every new worktree before the agent starts) and a **Run command** (what the Run button starts).

## Why it exists

Harness needs to know where repos live, how to make a fresh worktree usable, and how to run the project. Configuring this once per repo means every workspace on that repo is set up and runnable the same way, with no per-workspace fiddling.

## Where it lives

- Settings page, second section (`settings-repos`), between Agents and General (see [21](21-settings.md)).
- List `settings-repo-list`, one item `settings-repo-<key>` per repo; add form `settings-repo-add`, button `settings-repo-add-btn`.
- Per repo: `settings-repo-<key>-configure`, `settings-repo-<key>-config`.
- Also reached from an Output tab's **Add run command** button (see [15](15-run-app-and-output.md)).

## Anatomy

### Section header
`Repos` / `Local folders that new workspaces can check out from.`

### Repo list
One row per repo:

1. **Folder icon.**
2. **Name** (derived from the last folder in the path), for example `my-app`.
3. **Path** in monospace under the name, for example `~/code/my-app`, with the full path in a tooltip.
4. **Usage**: `2 workspaces` when any workspaces use this repo.
5. **Flag**: `No run command` in amber when the run command is empty.
6. **Configure** ghost button with a chevron, which expands the config panel (`aria-expanded`).
7. **Remove** ghost button with a trash icon.

### Config panel (expanded under the row)

**Default branch**
- Select of the repo's local and origin branches, monospace. Detected from `origin/HEAD` when the repo is added (`main` if that fails).
- Hint: `New workspaces branch from here. It stays in the repo's own checkout, so workspaces can't check it out; refresh it from the sidebar.`
- Saves on change. Refused while a workspace has that branch checked out: `<workspace> has <branch> checked out. Switch it to another branch first.`

**Worktree setup**
- Monospace textarea, 4 rows, no spellcheck.
- Placeholder: `pnpm install` / `cp <repo path>/.env .env`.
- Hint: `Runs top to bottom in each new worktree before the agent starts. One command per line.`

**Run command**
- Single-line text input, no spellcheck, placeholder `pnpm dev`.
- Hint: `Starts the app from the worktree root when you press Run in a workspace`.

### Add form (under the list)
- Label `Add a repo`.
- Path input, placeholder `~/code/my-project`.
- Secondary **Add repo** button with a plus icon.
- Hint: `The folder that contains the repo's .git directory`.
- Error line, replacing the hint when shown.

## Behaviour

### Editing config
- Changes save as you type; there's no Save button.
- The run command is trimmed. The setup text is kept as typed.
- The `No run command` flag appears and disappears live as the run field empties or fills.
- Setup parsing (used at provisioning time): split by line, trim each, skip blank lines and lines starting with `#`. So comments can be left in the setup box.
- Config changes affect the *next* workspace created (setup) and the *next* Run (run command). Running apps aren't restarted.

### Expanding and collapsing
- **Configure** toggles the panel. Opening it moves focus to the Worktree setup box.
- Expanded state is remembered while the app is open.

### Adding a repo
1. Enter a path and press **Add repo** (or Enter).
2. Trailing slashes are removed. The name is the last path segment.
3. Validation (the first failing rule shows its error and focuses the field):

   | Rule | Error |
   | --- | --- |
   | Not empty | `Enter the path to a folder` |
   | Starts with `~` or `/`, and has a folder name that isn't just `~` | `Use a full path, like ~/code/my-project` |
   | Path not already added | `That folder is already added` |
   | Name not already used by another repo | `A repo named <name> is already added` |

4. On success: the repo is added (branches `main`, empty setup, empty run command), the field clears, the new repo's config opens, the page scrolls to it and focus goes to its Worktree setup box.
5. Toast: `Added <name>. Add its setup and run commands below.`
6. The repo is immediately available in the New workspace dialog's Repository menu.

Typing in the path field clears any error.

### Removing a repo
- **Remove** is disabled (with a tooltip) when:
  - Workspaces use the repo: `<name> has <N workspaces>. Tear it down first.` / `…Tear them down first.`
  - It's the last repo: `Harness needs at least one repo`.
- Otherwise the repo is removed immediately (no confirmation). Toast: `Removed <name>. The folder on disk wasn't touched.`
- Focus moves to the next enabled Remove button, or to the path field.

### Jumping here from a workspace
**Add run command** in an Output tab's empty state opens Settings (or re-renders it), expands that repo's config, scrolls it into view (24px from the top) and focuses its Run command field.

### Where repo config is used
- **Setup** runs during provisioning, streaming to the Output tab (see [07](07-worktree-provisioning.md)); the Lead thread's step reads `Ran worktree setup` with `<N commands> from <repo> settings, see the Output tab`.
- **Run command** is used by Run and Restart, shown in the Output header and palette meta, and determines the app type and base port (see [15](15-run-app-and-output.md)).
- **Branches** feed the New workspace base-branch typeahead and the workspace branch picker.

### The default branch belongs to the repo checkout
Git lets a branch be checked out in one worktree only, and the sidebar's Repos refresh button pulls the default branch in the repo's own checkout (see [01](01-app-shell-and-sidebar.md)). So no workspace may check it out:
- New workspace: a branch name equal to the default branch shows `<branch> is the repo's default branch. Pick a new branch name.` The base branch field starts on the repo's default branch.
- Branch picker: the default branch is listed with a `default` tag and disabled.
- The core refuses creating a workspace or review workspace on it, and switching to or creating it in a workspace: `Workspaces can't check out <branch>, the default branch of <repo>. Use another branch.`

## Sample repos

| Name | Path | Worktree setup | Run command |
| --- | --- | --- | --- |
| my-app | `~/code/my-app` | `pnpm install --frozen-lockfile`, `cp ~/code/my-app/.env.local .env.local`, `pnpm db:migrate` | `pnpm dev` |
| my-app-api | `~/code/my-app-api` | `uv sync`, `cp ~/code/my-app-api/.env .env` | `uv run fastapi dev` |
| design-system | `~/code/design-system` | `pnpm install` | `pnpm storybook` |
| marketing-site | `~/code/marketing-site` | `npm ci` | (none, to show the flag and empty state) |

All sample workspaces belong to `my-app`.

## Keyboard and accessibility

- The list is a `<ul>` labelled by the section heading.
- Configure buttons have `aria-expanded` and `aria-controls`, and accessible names `Configure <name>`; Remove buttons are named `Remove <name>`.
- Every config field has a `<label>` and is described by its hint.
- The path input is described by its hint and error; `aria-invalid` when invalid.

## Data model

```
REPOS = [{
  id: 'my-app',               // name, from the last path segment
  path: '~/code/my-app',
  branches: ['main', ...],
  setup: 'multi-line string',
  run: 'pnpm dev'
}]
```

Workspaces refer to a repo by `repo` id (defaulting to `my-app` when absent).

## Simulated in the prototype

A real build must:

- Verify the path exists and is a git repo (contains `.git`), and offer a native folder picker instead of typing paths.
- Read branches from git (`git branch`, plus remotes) instead of a fixed list, and refresh them.
- Detect the default branch.
- Persist repo config. Consider also reading it from a file committed in the repo (`.cormux/config.json`), so a team shares setup and run commands, with Settings as a local override.
- Suggest setup and run commands by inspecting the repo (lockfile → package manager, `package.json` scripts, `pyproject.toml`).

## Known gaps and open questions

- **No folder picker;** paths are typed by hand.
- No path validation beyond format.
- Removing a repo has no confirmation (it's harmless on disk, but loses its config).
- Repos can't be renamed; two repos with the same folder name can't both be added.
- One run command per repo; see [15](15-run-app-and-output.md) for multiple processes.
- No per-repo environment variables, default engine, or merge/rebase preference.
- No variables in setup commands (for example `$HARNESS_MAIN_CHECKOUT`), so the sample hard-codes `~/code/my-app/.env.local`.
- No "Test setup" button to try the commands before creating a workspace.
