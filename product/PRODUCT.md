# Product

<!-- impeccable:product-schema 1 -->

> Written from the `index.html` prototype and updated with the confirmed tech stack. Facts marked **(inferred)** are hypotheses from the prototype and still need confirming. How the product is built lives in [`ARCHITECTURE.md`](ARCHITECTURE.md); feature behaviour lives in [`features/`](features/README.md).

## Platform

web

A native desktop app for macOS whose interface is built with web technology: a **Tauri 2** shell with a **Rust** backend, and a **Svelte 5** interface rendered in the system webview. Design guidance for the web applies to the interface. The window uses a native macOS title bar area with real traffic-light controls, a draggable top strip, a persistent sidebar and keyboard-first navigation.

macOS is the first and only target for now. Tauri keeps Windows and Linux possible later; see [Platforms in ARCHITECTURE.md](ARCHITECTURE.md#platforms) for what they would need.

## Users

Software developers who run several AI coding agents at once against their own repositories, and who review pull requests that they opened or are requested on. **(inferred)** Their job is supervision more than typing: starting agents on tasks, answering approvals, checking diffs and output, and getting finished work into a PR.

They already work in a terminal and have at least one agent CLI installed and signed in (Claude Code or Cursor CLI), git configured with push access, and a GitHub account.

Solo developer versus team, and how much PR review matters compared with building, is unconfirmed.

## Product Purpose

Cormux lets one person run many coding agents in parallel without them interfering with each other. Each task gets a **workspace**, which is an isolated git worktree on its own branch with one or more agent **threads**. Cormux covers the loop from creating a branch to tearing it down: set up the worktree, run agents, run the app, review changes, open a PR, and clean up.

Success means the user can keep several workspaces moving at once and always knows which one needs them. **(inferred)**

## Positioning

**(inferred, unconfirmed)** Cormux sits above individual agent CLIs rather than competing with them:

- **Engine-agnostic:** every thread picks an engine. Cormux drives the CLIs the user already has installed and signed in to, so it never handles their model accounts. **MVP launch:** Claude Code and Cursor CLI. Codex and Gemini use the same ACP adapters and appear in Settings when installed.
- **Isolation by default:** one worktree and branch per workspace, so parallel agents never share a checkout.
- **The whole lifecycle in one place:** worktree setup, running the app, diffs, PR creation, PR review and teardown, not just the chat.
- **Local and light:** a small native app (about 10 MB) that runs everything on the user's machine with their own git and credentials.

## Operating Context

- Work is organized by **repo** (a local folder containing a git repository), then **workspace** (worktree plus branch), then **thread** (one agent conversation).
- **Homebase** is the overview: workspace cards with status ("2 agents working", "Idle", "Needs Attention"), plus open GitHub PRs that the user opened, is requested on, or is mentioned in.
- Inside a workspace, the **agent conversation is the main surface**. Threads are tabs along the top. An Output tab shows the running app's logs, and a Changes panel shows the diff on demand. The file tree and editor are deliberately secondary for now.
- Agents ask for **approvals** before risky tool calls. Read-only tools can be auto-approved.
- **Review workspaces** check out a PR branch. A Reviewer thread produces **findings** grouped as Blocking, Suggestions and Nits, and the user picks which ones to send to an agent to fix.
- The user works in a local git environment with GitHub as the remote.
- Cormux is often in the background while agents work. It must reach the user through OS notifications and the dock badge when something needs them.

## Technical Foundation

The confirmed stack. Details, module boundaries and the feature-by-feature check are in [`ARCHITECTURE.md`](ARCHITECTURE.md).

| Layer | Choice | What it does for the product |
| --- | --- | --- |
| Native shell and backend | Tauri 2.x (Rust) | The desktop window and every side effect: git worktrees, agent processes, setup and run commands, GitHub, storage and notifications. Small on disk (about 5 to 10 MB) and light at idle (about 15 to 30 MB for the core). |
| Interface | Svelte 5 + Vite | A single page app whose fine-grained reactivity (runes) keeps streaming conversations, live logs and diffs smooth without a virtual DOM. |
| Components and styling | Tailwind CSS v4 + bits-ui / shadcn-svelte | A dark developer interface on accessible primitives: dialogs, command palette, menus, tabs, switches and resizable panes. |
| Icons | lucide-svelte | Status, git action and workspace state indicators. |

What this means for the product:

- **Agents are driven through structured protocols, not a terminal.** Each thread's CLI talks to Cormux in JSON (the Agent Client Protocol, or Claude Code's stream-JSON), which is what lets Cormux show tool steps, edits, plans and approval cards instead of raw terminal output. Setup and run commands do run in a terminal-like PTY, so the Output tab shows real colourised output.
- **Cormux uses the user's own tools and credentials.** It runs the system `git` with the user's config and SSH keys, spawns commands in the user's login shell environment (so `nvm`, `asdf`, `pnpm` and `uv` work), and keeps the GitHub token in the macOS Keychain.
- **The Rust core owns the truth.** The interface can reload without losing work, and agent sessions resume after Cormux restarts where the engine allows it.
- **Memory is mostly the agents.** The core is light, but each agent CLI uses 150 to 400 MB. The memory meter reports that honestly.

## Capabilities and Constraints

Shown in the prototype, all simulated there, and all supported by the stack:

- Create a workspace from a repo, base branch, engine and an initial prompt. The workspace name and branch are drafted from the prompt.
- Per-repo config: worktree setup commands (one per line, run in order before the agent starts) and a run command.
- Run, restart and stop the app from the workspace header, with output in an Output tab. Each workspace's app gets its own port.
- Switch a workspace's branch, which is locked while any agent is running. Pull and rebase onto the base branch.
- Create PR, with the "why is this change necessary" text drafted by a small model call.
- Review a PR in its own workspace, pick findings to fix, and submit the review.
- Teardown, optionally deleting the branch (on by default).
- A command palette, and Settings for Agents, Repos, General and Skills.

Required by a real build, and missing from the prototype:

- **Failure states** for worktree creation, setup commands, app crashes, git conflicts, push failures and GitHub errors. Each one says what failed and offers a next step.
- **OS notifications** when an agent needs approval or a review finishes while Cormux is in the background.
- **Engine detection** in Settings: which CLIs are installed, their versions and whether they are signed in.
- **Persistence** of settings, repos, workspaces and transcripts across restarts.

Constraints:

- The user must have git, at least one supported agent CLI, and a GitHub account. Cormux doesn't install or sign in to engines for them.
- Nothing leaves the machine except the engines' own model traffic, GitHub API calls, and any small model calls the user enables.
- **Pause is interrupt and hold.** An agent can't be frozen mid-request; Pause cancels the current turn and holds the thread until Resume. The composer spec should be updated to match.

Terminology: **workspace**, **thread**, **engine**, **repo**, **Homebase**, **findings**, **teardown**. Workspaces are named in plain language ("Auth session timeout"); the branch (`feat/auth`) is shown separately and never stands in for the name.

## Decisions (MVP)

Recorded from the shipped build (epic COR-31). Technical detail in [`ARCHITECTURE.md` → Decisions (MVP)](ARCHITECTURE.md#decisions-mvp).

| ID | Decision |
| --- | --- |
| COR-207 | **Engines at launch:** Claude Code + Cursor CLI. Codex and Gemini ACP adapters are in the app and selectable when the CLI is installed; not required for MVP positioning. |
| COR-208 | **GitHub:** Personal access token in macOS Keychain (Settings), with optional fallback to `gh auth token` when no PAT is stored. Submit review posts **line comments** for findings that have file and line; others go in the review body. |
| COR-209 | **Small model:** One-shot calls through the installed Claude CLI (`haiku`) using the user's existing Claude sign-in. Optional Anthropic API key in Keychain if set. No separate API key required for MVP. |
| COR-210 | **Quit:** On exit, the process supervisor runs `stop_all` (app and setup PTYs). Agent engines get a **5s graceful shutdown** on workspace teardown; nothing is left running in the background after Cormux quits. |
| COR-211 | **Transcripts after teardown:** Archive workspace and thread history in SQLite (`archived_at`); worktree and branch still removed on teardown. |
| COR-212 | **Changes Approve / Reject:** Approve **stages** the file; Reject **discards** worktree edits for that path (restore tracked, remove untracked). **Undo** clears the UI review mark only. Create PR does not require per-file approvals. |
| COR-213 | **Permissions:** Auto-approve read-only tools when enabled; edits, deletes and shell execution always need the user. Per-repo command allowlists are deferred (hook only). |
| COR-214 | **Skills:** Settings placeholder only for MVP; no skill storage or engine attachment yet. |
| COR-215 | **Merge, file tree, editor:** No in-app file tree or editor. Pull/merge conflicts offer abort; open files in the OS editor. Shipping is via Create PR, not an in-app merge action. |
| COR-216 | **Visual:** shadcn-svelte **nova**, Geist and Geist Mono, dark developer UI aligned with the prototype, not pixel-perfect. Logo still open. |
| COR-217 | **Threads and worktrees:** Join-thread adds another agent on the **same** worktree as the Lead, not sub-worktrees; coordination is conversational. |
| COR-218 | **Pricing / distribution:** UNLICENSED local app for MVP; no billing or accounts. Commercial packaging and distribution later. |

Also decided earlier:

- Desktop app on Tauri 2 with a Svelte 5 interface, macOS first.
- Worktrees live at `~/.cormux/worktrees/<repo>/<branch>`, so two repos can use the same branch name.

## Brand Commitments

- The product name is **Cormux**.
- UI copy is plain and specific, in sentence case, with no em-dashes and at most one separator per metadata line. **(inferred from past edits)**
- The interface is dark, built on shadcn-svelte **nova** with Geist and Geist Mono (see COR-216). Ledger and Tidepool in `alternatives/` remain reference only. No logo yet.

## Evidence on Hand

- Prototype: `index.html`. Alternative visual directions: `alternatives/ledger/`, `alternatives/tidepool/`. These aren't checked into this repository yet.
- Feature specs for every prototype feature in [`features/`](features/README.md), and the architecture in [`ARCHITECTURE.md`](ARCHITECTURE.md).
- All workspaces, PRs, findings, repos and agent output are **placeholder sample data**. There are no real users, metrics, testimonials, customers or GitHub data. Future work must not present any of these as real.

## Product Principles

**(inferred from decisions made while building the prototype)**

1. **The conversation is the product.** The agent thread is the main surface; code, diffs and output are available on demand without taking over.
2. **Show what needs you.** Status comes down to working, idle or needs attention, so the user can supervise many workspaces at a glance, even when Cormux is in the background.
3. **Isolation is non-negotiable.** One worktree per workspace. Actions that could break a running agent (like switching branches) are locked, and the lock explains why.
4. **Drafted, never presumed.** Models draft names, PR reasons and findings, but the user edits, selects and confirms before anything leaves the machine.
5. **Clean up is part of the job.** Teardown removes the worktree and, by default, the branch.
6. **Your tools, your credentials.** Cormux drives the git, shell and agent CLIs the user already trusts, and never asks for more access than they already have.

## Accessibility & Inclusion

No product-specific requirement confirmed. The prototype already supports reduced motion, full keyboard navigation and visible focus, and announces new agent messages to screen readers. The chosen components (bits-ui) provide accessible dialogs, menus, tabs and focus management, so the build should keep all of this. OS notifications must have a text equivalent inside the app, since not everyone allows them.
