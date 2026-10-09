# Changelog

Each GitHub release shows the bullets under its version's heading, such as `## 0.1.10`, newest first. Add a section when you bump the version; the release workflow fails before building if the section is missing or empty.

## 0.1.10

- Release notes on GitHub now list what changed in each version.

## 0.1.9

- The sprint board only shows tasks assigned to you.

## 0.1.8

- New interface font (Outfit) and code font (JetBrains Mono).
- Each agent gets a clay face and a stable name from The Office.
- A repo page, opened from the sidebar, lists its workspaces and local branches and can delete branches nothing has checked out.
- Each agent reply ends with a quiet line showing how long the turn took.
- The composer stays focused after you send a message.
- The New workspace dialog hides the engine picker when only one agent is installed.
- The ticket flyout no longer shows a status.

## 0.1.7

- The / skill picker also lists Claude Code's built-in skills.

## 0.1.6

- A Review button reviews a workspace's branch and fills the Findings tab (⌘I).
- A / skill picker in the composer lists the repo's and your Claude skills.
- Web fetches, web searches and skills show up as quiet timeline lines.
- The Git tab opens instantly and loads diffs as you scroll.
- Create PR opens pull requests as drafts.
- Clicking a toast or notification takes you to its thread or Findings tab.
- Chat text can be selected and copied.
- Header buttons fold into a ⋯ menu when the window is narrow, and Restart is gone.
- The ClickUp key is stored in `~/.cormux/credentials.json` instead of the Keychain.
- Ending a scratch is faster, and the restart toast after a branch switch is gone.

## 0.1.5

- ClickUp sprint board with rich task cards, hideable lanes and points-based progress, plus an In progress section on Homebase.
- A per-repo "Run one instance at a time" setting stops the app in other workspaces when you run it.
- The Git tab's line counts animate and clear as soon as changes are committed.
- Keyboard shortcuts have their own Settings section.

## 0.1.4

- Scratches are listed in the sidebar.
- The sidebar shows repo commit counts and marks running apps.
- Stacked PRs are grouped into cards, and stacks made in other worktrees are picked up.
- Choose which terminal app to open, and open https apps over https.
- Test files are collapsed in diffs, and Vue files are syntax-coloured.
- Homebase and Settings scroll properly, with a pinned Homebase header.

## 0.1.3

- Claude's tool calls and thinking appear live, and threads no longer hang on "Working".
- Stack and Changes are combined into one Git tab.
- A Repos section in the sidebar keeps each repo's default branch pulled.
- Per-thread model picker in the composer.
- Code blocks in replies have syntax highlighting and a copy button.
- Keyboard: Esc stops the agent, ↑/↓ cycle past prompts, ⌘↵ focuses the composer and confirms dialogs.
- Diff comments are restyled as cards, can be cleared, and are counted per stack level.
- A PR prompt setting, and a roomier Create PR dialog.
- Dev builds are clearly marked and keep their own data; repo config lives in `.cormux`.
- Deleting workspaces disappear from the sidebar straight away.
- Fixed removing a repo that has torn-down workspaces or synced PRs.

## 0.1.2

- Fixed the app becoming unclickable after closing a dialog or menu.

## 0.1.1

- Branch pickers list remote branches, and a branch's stack is picked up from GitHub.
- Fixed the UI getting stuck when a dialog or popup didn't finish closing.

## 0.1.0

- First release: workspaces on git worktrees, with Claude and Cursor agent threads.
- Homebase with workspace cards and open pull requests synced from GitHub.
- Changes tab with live diffs and line comments, plus branch switching, pull and rebase.
- Create PR and review workflows, with a Findings tab for review results.
- Run your app with logs in an Output tab.
- Scratches: one-off, read-only conversations against a repo.
- Command palette, global shortcuts, toasts and OS notifications.
- Settings for repos, agents, GitHub and appearance, including light mode.
