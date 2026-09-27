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

- **Engine-agnostic:** every thread picks an engine. Cormux drives the CLIs the user already has installed and signed in to, so it never handles their model accounts. Claude Code and Cursor CLI are the launch engines; Codex CLI and Gemini CLI follow, since all four can speak the same structured protocol.
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

Decided:

- Desktop app on Tauri 2 with a Svelte 5 interface, macOS first.
- Worktrees live at `~/.cormux/worktrees/<repo>/<branch>`, so two repos can use the same branch name.

Undecided (see [Open technical decisions](ARCHITECTURE.md#open-technical-decisions) for recommended answers):

- Whether Codex CLI and Gemini CLI ship at launch or follow.
- How GitHub sign-in works (reuse the GitHub CLI's token, or Cormux's own sign-in) and whether reviews are posted with line comments.
- Where small model calls come from (through the user's Claude Code sign-in, or an API key they add).
- Whether apps and agents keep running when Cormux quits.
- Whether transcripts are archived after teardown.
- Skills management (the Settings section is a stub).
- How the file tree and editor fit in.
- Merge flow (Create PR replaced the old direct merge).
- Pricing, licensing and distribution.

## Brand Commitments

- The product name is **Cormux**.
- UI copy is plain and specific, in sentence case, with no em-dashes and at most one separator per metadata line. **(inferred from past edits)**
- The interface is dark, built on shadcn-svelte components themed to Cormux. Beyond that the visual direction isn't settled: the prototype uses its own dark palette and Geist, and there are two alternative directions (Ledger and Tidepool) in `alternatives/`. No logo exists.

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
