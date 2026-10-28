# Cormux

<p align="center"><img src="product/logos/full_cormux.png" alt="Cormux" width="491"></p>

A macOS app for running many coding agents in parallel, each in its own git worktree. Product context lives in [`product/PRODUCT.md`](product/PRODUCT.md) and the technical design in [`product/ARCHITECTURE.md`](product/ARCHITECTURE.md).

## Install

Download the `.dmg` from the [latest release](https://github.com/cormacrw/cormux/releases/latest), open it, and drag Cormux to Applications. It needs macOS 13 or later and runs on Apple Silicon and Intel Macs.

The app isn't signed with an Apple Developer ID yet, so macOS will say it's damaged or can't be opened. Clear the quarantine flag once after installing:

```sh
xattr -dr com.apple.quarantine /Applications/Cormux.app
```

## Stack

Tauri 2 (Rust) shell and core in `src-tauri/`, with a Svelte 5 + Vite + TypeScript single page app in `src/`.

## Development

Requirements: macOS, Node 22+, pnpm 8+, and rustup (the toolchain in `rust-toolchain.toml` installs automatically).

```sh
pnpm install
pnpm tauri dev     # run the app with hot reload
pnpm tauri build   # build Cormux.app and a .dmg
pnpm build:dmg     # build just the .dmg, for this Mac's architecture
pnpm clean         # delete build output, including src-tauri/target (next build starts from scratch)
pnpm lint          # ESLint + Prettier
pnpm check         # svelte-check + tsc
pnpm test          # Vitest (pure TS helpers)
pnpm test:e2e      # Playwright against Vite + mocked IPC (Chromium)
cargo test --manifest-path src-tauri/Cargo.toml
```

`pnpm test:e2e` is how UI changes get verified without the native window. It stubs Tauri invoke (see `src/lib/dev/browser-harness.ts`), clicks through the Svelte shell, and writes `e2e/output/*.png`. Overlay traffic lights still need `pnpm tauri dev`.

## Optional skill: task draft cards

When an agent scopes work in a conversation, Cormux can show it as a hierarchy instead of leaving it in the prose: the epic and its scope first, then the tasks under it, then their subtasks. This is opt-in: install the [`cormux-task-cards` skill](skills/cormux-task-cards/SKILL.md), which asks any task-drafting workflow to scope epics before breaking them down, and to also present the result in a structured block that Cormux renders.

```sh
mkdir -p ~/.claude/skills
cp -R skills/cormux-task-cards ~/.claude/skills/
```

(Or symlink it instead of copying, so repo updates flow through: `ln -s "$PWD/skills/cormux-task-cards" ~/.claude/skills/cormux-task-cards`.)

The skill pairs with whatever skill or workflow you already use to draft and create tasks — it only controls presentation, and creating the tasks still happens with the agent in chat. Nothing is installed automatically, and without the skill conversations render exactly as before; a reply that contains no block, or a malformed one, is shown as plain prose.

## Releasing

Every push to `main` runs `.github/workflows/release.yml`, but it only publishes a release when the version is new: if `v<version>` is already released, the run passes in seconds and builds nothing. So bump the version before merging `develop` into `main`, in all four places:

- `src-tauri/tauri.conf.json` (the version the workflow reads)
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` (the `cormux` package entry)
- `package.json`
