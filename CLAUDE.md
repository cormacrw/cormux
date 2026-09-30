# Cormux

See `README.md` for the stack and the commands to build, lint and test.

## Releasing

When merging `develop` into `main`, always bump the version first (patch by default, e.g. 0.1.1 → 0.1.2), unless told otherwise. The release workflow only publishes a new GitHub Release when the version changes; with an unchanged version the push builds nothing. Keep these four in sync:

- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` (the `cormux` package entry)
- `package.json`

`main` CI also runs `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`, so run both (plus `pnpm lint` and `pnpm check`) before pushing to `main`.
