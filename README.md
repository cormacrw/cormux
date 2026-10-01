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

`pnpm test:e2e` is how UI changes get verified without the native window. It stubs Tauri invoke (see `src/lib/dev/browser-harness.ts`), clicks through the Svelte shell, and writes `e2e/output/*.png`. Overlay traffic lights and Keychain still need `pnpm tauri dev`.

## Releasing

Every push to `main` runs `.github/workflows/release.yml`, but it only publishes a release when the version is new: if `v<version>` is already released, the run passes in seconds and builds nothing. So bump the version before merging `develop` into `main`, in all four places:

- `src-tauri/tauri.conf.json` (the version the workflow reads)
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` (the `cormux` package entry)
- `package.json`
