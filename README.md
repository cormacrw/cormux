# Harness

A macOS app for running many coding agents in parallel, each in its own git worktree. Product context lives in [`product/PRODUCT.md`](product/PRODUCT.md) and the technical design in [`product/ARCHITECTURE.md`](product/ARCHITECTURE.md).

## Stack

Tauri 2 (Rust) shell and core in `src-tauri/`, with a Svelte 5 + Vite + TypeScript single page app in `src/`.

## Development

Requirements: macOS, Node 22+, pnpm 8+, and rustup (the toolchain in `rust-toolchain.toml` installs automatically).

```sh
pnpm install
pnpm tauri dev     # run the app with hot reload
pnpm tauri build   # build Harness.app and a .dmg
pnpm lint          # ESLint + Prettier
pnpm check         # svelte-check + tsc
pnpm test          # Vitest
cargo test --manifest-path src-tauri/Cargo.toml

```
