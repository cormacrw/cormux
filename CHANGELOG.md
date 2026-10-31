# Changelog

Each GitHub release shows the bullets under its version's heading, such as `## 0.1.10`, newest first. Add a section when you bump the version; the release workflow fails before building if the section is missing or empty.

## 0.1.18

- When an agent asks you something, the question appears in a card with options to pick from and a box for your own answer.
- Threads keep running while background subagents work, and Stop ends those subagents too.
- Deleting a workspace only asks for confirmation when the worktree has uncommitted changes, and a branch holding commits that exist nowhere else is kept by default.
- The composer's base branch picker is gone. Threads start from the repo's default branch, and the agent moves to another branch if your first message asks for it.
- New branches get a descriptive name from Haiku before the worktree is created.
- Claude threads can list closed threads and read their transcripts to pick up earlier work.
- Agents screenshot visible changes without being asked.
- A child thread finishing after its parent was killed no longer brings the parent back.
- Agent names and clay faces are gone; threads show their role instead. The context growth sparkline is gone from the prompt bar, and the teardown toast no longer appears.
- Codex and Gemini are removed. Existing threads from either carry on as Claude threads, starting a fresh session.

## 0.1.17

- Claude threads can hand work to other threads. An agent can start a new thread on its own branch and worktree, check on threads in the same repo, and send follow-ups to the ones it started. Those threads report back on their own when they finish a turn, get blocked or fail setup.
- Threads started by an agent sit under their parent in the sidebar, and the parent's header cost includes them. Starting a thread or messaging one asks for your approval as usual, and only you can answer a child thread's approvals.
- Every thread now gets its own worktree, with a flat thread sidebar and an empty thread as the first page. Working threads move into a collapsible Working section once you leave them.
- Finished replies render diagrams, charts, math, diffs and JSON as rich blocks, test runs are summed up, and the prompt bar shows context growth.

## 0.1.16

- Screenshots and recordings an agent references in a reply now show inline in the thread, and they survive the worktree being torn down. Every engine is told at startup that the thread renders them, so agents embed captures instead of just naming the path.
- Scoped work can show as a hierarchy in the conversation: the epic with its scope and done-when list, then its tasks and subtasks, with a total of points. It's opt-in through the `cormux-task-cards` skill in the repo (see the README to install it).
- The sidebar now sits flush to the window edges, and the traffic lights move in to match.

## 0.1.15

- The "cormux" wordmark falls away with the greeting when you send a conversation's first prompt, and rises back when you start a new session. It stays hidden while a thread has a conversation going.
- Pinning Git or Output beside the conversation is smooth again, and the tray now slides over the conversation instead of under it.
- Pages appear without the wordmark flashing sharp through their glass cards.
- In full screen the sidebar moves up into the space the traffic lights leave, and the DEV badge sits in the sidebar footer.
- Claude is the only engine for now: Cursor is gone, and existing Cursor threads carry on as Claude threads.
- Opening a workspace without gh-stack installed no longer shows a "Something went wrong" toast, and error toasts show the real message.

## 0.1.14

- A new look: frosted macOS glass over a colour theme. Pick Neutral or a bold colour in Settings › Appearance; it tints the window, the glass and the buttons, with a faint "cormux" behind it all. The window opens already drawn in your theme.
- Starting a new session clears the thread to a greeting just above the prompt bar. From there, resume an earlier conversation (the agent picks its session back up) or pick a task to work on.
- Bangs replace scratch macros: type `!name` in any prompt bar to drop in a saved prompt, or in the command palette to start a scratch with it.
- The Week page plans your TODOs and sprint tasks onto days, with a switch to hide weekends. Homebase shows today's tasks, and the TODOs page is gone.
- Tasks open in one flyout that slides over Homebase, Week and Sprint. TODOs now have IDs like `CMUX-12`; `#CMUX-12` in a prompt points the agent at the task, just like a ClickUp task.
- Git and Output slide out over the conversation, can be pinned beside it, and toggle with ⌘G and ⌘O.
- The workspace header keeps its actions in ⋯, with icon buttons to run, stop and restart the app. The sidebar groups workspaces by repo.
- The New workspace dialog opens instantly, with a searchable base branch picker.

## 0.1.13

- Ctrl+Tab switches between workspaces and the main pages (Homebase, TODOs, Sprint), most recent first. Tap it to jump back to where you were, or hold Ctrl to pick from a list.
- Type `#` in the composer to pick a task from your sprint board. It's sent as the task's ID, name and link, with its description for the agent.
- Agent replies type out quickly instead of falling far behind, one reply at a time, and formatting no longer flickers as it types.
- The composer keeps focus: opening or returning to a conversation, using its controls, or just starting to type puts you back in the prompt.
- New shortcuts: ⇧⌘] and ⇧⌘[ move between a workspace's tabs, ⇧⌘N starts a thread, and ⇧⌘P switches plan mode. Settings now lists every shortcut.
- Attached images show as large thumbnails, including ones picked or dropped from Finder.

## 0.1.12

- Attach images and files in the composer: paste a screenshot, drop files on it, or pick them with the paperclip. The agent gets each file by path, and the model sees images.
- Type `@` in the composer to pick one of your repos; `@cormux` is sent as that repo's folder, so an agent can work across repos.
- Find in a thread or scratch with ⌘F, with highlighted matches you can step through, even in long threads and collapsed steps.
- Each thread shows its tokens, how full its context is and what it has cost, with totals in the workspace header and on Homebase. Claude's usage is now read correctly, cached tokens included.
- Override the run command for one workspace, inline in its Output tab.
- Dismiss review findings you won't act on, and restore them; dismissed ones aren't sent to an agent or posted with the review.
- Fable is in the model picker for Claude threads.
- Menus in New workspace, New scratch, Findings and Plan match the rest of the app instead of using the system's.
- A new workspace's branch name is only checked against workspaces of the same repo.

## 0.1.11

- Plan mode for Claude threads: switch it on in the composer and the agent writes a plan before touching anything. The plan opens in its own tab, where you can highlight any part of it and comment, send the comments back for a revision, or approve it so the agent carries it out. While a plan waits, the agent shows as waiting for you instead of working.
- The skill picker opens for a `/` typed anywhere in the composer, and lists Claude Code plugin skills too.
- Discard uncommitted changes from the Changes panel, one file or all of them.
- Moved or renamed files show once in Changes, as a rename.
- The sidebar can be resized by dragging its edge.
- Deleting an unmerged local branch, such as a stacked one, asks first instead of failing, and only asks when the branch isn't safely pushed.
- A workspace's agents leave the sidebar as soon as it starts deleting.
- Appearance comes first in Settings.
- Sprint points include a half point, and the sprint board no longer lists assignees.

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
