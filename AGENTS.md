# Conic Launcher — Agent Guide

The app is native Slint + Rust. Before updating any `.slint` files, read the Slint documentation first.

## Quick start

```bash
cargo run                   # debug build, run it
cargo run --release         # release build
cargo build --release       # what the packages ship
```

First of all, read ARCHITECTURE.md to get an overview of the project.

## Verification

```bash
cargo fmt --all -- --check
cargo check
cargo clippy --all-targets --release -- -D warnings   # warnings fail CI
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
cargo test
```

Matching `check-rust.yml`, which runs `fmt` on Linux; `check` + `clippy`
(`--all-targets --release -- -D warnings`) and `doc` (`RUSTDOCFLAGS="-D warnings"
cargo doc --no-deps --document-private-items`) on Linux, macOS **and** Windows;
plus a "test" job that is `cargo test --all --release --verbose --all-targets`.

## Rust conventions

- Workspace edition 2024, `rust-version = "1.88"`, resolver 3.
- `#![deny(clippy::unwrap_used)]` applies to `app/src/main.rs` only. The
  Slint-generated module is opted out with `#[allow(clippy::unwrap_used)]` —
  keep that scoped, do not widen it.
- External deps are declared in the root `Cargo.toml` `[workspace.dependencies]`
  and pulled in with `workspace = true`; the local crates are declared there too
  and are not published. Platform-gated deps (winit, zbus, objc2, windows) are
  declared in the crate that needs them.
- Release profile: `panic = "abort"`, `lto`, `codegen-units = 1`,
  `opt-level = "z"`, `strip`. A release build is slow on purpose.
- `slint` is pulled with `default-features = true`, which brings the femtovg
  (OpenGL) and software renderers.
- File header convention: `// Conic Launcher` / copyright /
  `// SPDX-License-Identifier: GPL-3.0-only`.
- Comment density is a house style here: explain _why_, and the alternatives that
  were rejected. Match it.

## UI conventions

- Slint: properties and bindings, `callback` for events, `@tr()` for
  translated strings, `@image-url` for assets.
- **Never** add `cursor: pointer`; this is a desktop app.
- Rust touches a Slint component only from the event loop. From another thread,
  go through `upgrade_in_event_loop` — a `Weak` crosses threads, a strong handle
  cannot.
- Keep the winit backend hook (`app/src/support/platform/`, reached through
  `native::install_backend`) as the only place window attributes are set. It has
  to run before any winit window exists, which is why `main` calls it before
  `App::new`.
- Translations go through `@tr()`; never hardcode user-visible text. Add the
  string to all 12 catalogues via `tools/update-i18n.py`.

## License

GPL-3.0-only with GPLv3 §7 additional terms: modified distributions must rename the
software, keep copyright notices, and not hold the authors jointly liable. The
terms themselves are not expressible as an SPDX identifier, so they travel as the
`LICENSE` text the packages install; `cargo-rpm` reads `License:` from
`package.license` and has no override for it.
