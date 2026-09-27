# 07 · Worktree provisioning

## Summary

Provisioning is the short phase between creating a workspace and its agent starting work. Harness creates the worktree, runs the repo's configured setup commands in it (install dependencies, copy env files, migrate), then starts the agent. Progress shows on the workspace card, in the sidebar, in the thread, and as live output in the Output tab.

## Why it exists

A fresh worktree is a bare checkout: no `node_modules`, no `.env`, no local database. Without setup, the agent's first test run fails. Running the repo's setup commands automatically makes every workspace ready to build and run, the same way every time.

## Where it lives

Not a screen of its own. It appears in:

- **Workspace card:** `1 agent working` badge (provisioning counts as working) and the summary skeleton `Summarizing with Haiku 4.5…`.
- **Sidebar:** status dot `st-prov` and status word `Starting`.
- **Thread:** a live row at the end of the conversation.
- **Output tab:** the setup commands and their output, with a spinner on the tab.
- **Header:** the Run button is disabled with the tooltip `Available once the worktree is set up`.

## Phases

`ms` is the total provisioning time: 5,200ms for a new workspace, 4,200ms for a review workspace, 7,000ms for the sample Lazy-load dashboard workspace at launch.

| Phase | When | Workspace `activity` | Thread live row | `provStep` |
| --- | --- | --- | --- | --- |
| 1. Setup | 0 to 55% | `Running worktree setup…` (review: `Checking out the PR…`) | Title = activity; subtitle `Setting up the worktree` | 1 |
| 2. Agent starting | 55% to 100% | `Starting agent…` | Title `Starting agent…` | 2 |
| 3. Running | 100% | `Reading repo…` (Claude Code), `Drafting plan…` (other engines), `Reading the diff…` (review) | Spinner with rotating live steps | |

### Setup output

During the first half of provisioning, the repo's setup commands and their output stream into the Output tab one line at a time, evenly spaced. The block looks like:

```
Running 3 setup commands from my-app settings
pnpm install --frozen-lockfile
Lockfile is up to date, resolution step is skipped
Done in 6.4s
cp ~/code/my-app/.env.local .env.local
pnpm db:migrate
Applied 2 migrations to the local database
Done in 0.9s
──── Worktree ready ────
```

- Commands are the repo's **Worktree setup** lines (see [22](22-repos.md)), one per line. Blank lines and lines starting with `#` are skipped.
- If the repo has no setup commands, nothing streams.

### When provisioning finishes

1. The workspace status becomes `running` and its activity changes as above.
2. If it has no summary yet, one is written:
   - Build workspace: `Just started on “<goal>”, branched from <base>. <Engine> is reading the repo to find where the change belongs; no files have been touched yet.` (Claude Code) or `…is drafting a step-by-step plan for your approval; no files have been touched yet.` (other engines).
   - Review workspace: `Reviewing #<num> from <author> against <base>. <Engine> is reading the diff and will draft comments for your approval; nothing has been posted to GitHub yet.`
3. A step is added to the Lead's thread:
   - With setup commands: **Ran worktree setup**, detail `<N commands> from <repo> settings, see the Output tab`, chip `Worktree ready`.
   - Without: **Worktree ready**, detail `No setup commands configured for <repo>`, chip `Worktree ready`.
4. For review workspaces, the review completes 6 seconds later (see [19](19-review-workspaces.md)).

## Additional threads joining

When a new thread is added to an existing workspace (see [10](10-thread-tabs.md)), it goes through a lighter version:

- Status `provisioning`, activity `Joining worktree…`, live row subtitle `Joining the worktree`.
- First step: **Joined worktree**, detail `<branch> · shared with <N agents>`.
- After 2.4 seconds: status `running`, activity `Waiting for instructions…`, and the agent says: `I’m in the same worktree as the Lead. Tell me which part to take on and I’ll coordinate so we don’t edit the same files.`

Setup commands are not re-run for joining threads.

## Rules and edge cases

- Run, Restart and the palette's App commands are unavailable while provisioning.
- If the workspace is torn down mid-provisioning, every pending timer checks that the workspace still exists and does nothing if it doesn't.
- Switching branch is locked during provisioning (provisioning counts as a running agent).

## Data model

- Workspace: `status`, `activity`, `provStep`, `summary`, `summaryMin`, `kind`.
- App: `app.log` receives the setup lines; `app.timers` holds the streaming timers.

## Simulated in the prototype

Everything. Command output is canned, chosen by pattern:

| Command matches | Output |
| --- | --- |
| `uv sync` | `Resolved 48 packages in 212ms`, `Installed 48 packages in 1.9s` |
| `install` or `ci` | `Lockfile is up to date, resolution step is skipped`, `Done in 6.4s` |
| `migrate` | `Applied 2 migrations to the local database`, `Done in 0.9s` |
| starts with `cp` | nothing |
| anything else | `Done` |

A real build must:

- Run `git worktree add` and then each setup command in sequence with the worktree as the working directory, in the user's login shell so PATH, nvm, asdf and similar work.
- Stream stdout and stderr live to the Output tab.
- **Stop on the first failing command**, mark provisioning as failed, show which command failed and its exit code, and offer Retry, Skip and continue, or Edit setup commands. None of this failure path exists in the prototype.
- Decide whether the agent may start while setup is still running.
- Expose environment variables such as the main checkout path (the sample `cp ~/code/my-app/.env.local` hard-codes it) and the worktree path.

## Known gaps and open questions

- **No failure state** for setup or worktree creation.
- No progress indicator beyond the spinner; long installs give no sense of how far along they are.
- No timeout.
- Setup always runs; there's no per-workspace "skip setup".
- Should joining threads be able to run their own setup (for example a different engine's config)?
