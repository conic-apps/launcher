# Conic Launcher — Agent Guide

The app is native Slint + Rust. Before updating any `.slint` files, read the Slint documentation
first.

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
slint-lsp format -i $(git ls-files '*.slint')   # no check mode; CI diffs the result
cargo check
cargo clippy --all-targets --release -- -D warnings   # warnings fail CI
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
cargo test
```

Matching `check-rust.yml`, which runs `fmt` on Linux; `check` + `clippy`
(`--all-targets --release -- -D warnings`) and `doc`
(`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items`) on Linux, macOS **and**
Windows; plus a "test" job that is `cargo test --all --release --verbose --all-targets`, and a
`slint-format` job that checks `.slint` files with `slint-lsp format` on Linux.

## Rust conventions

- Workspace edition 2024, `rust-version = "1.88"`, resolver 3.
- `#![deny(clippy::unwrap_used)]` applies to `app/src/main.rs` only. The Slint-generated module is
  opted out with `#[allow(clippy::unwrap_used)]` — keep that scoped, do not widen it.
- External deps are declared in the root `Cargo.toml` `[workspace.dependencies]` and pulled in with
  `workspace = true`; the local crates are declared there too and are not published. Platform-gated
  deps (winit, zbus, objc2, windows) are declared in the crate that needs them.
- Release profile: `panic = "abort"`, `lto`, `codegen-units = 1`, `opt-level = "z"`, `strip`. A
  release build is slow on purpose.
- `slint` is pulled with `default-features = true`, which brings the femtovg (OpenGL) and software
  renderers.
- File header convention: `// Conic Launcher` / copyright /
  `// SPDX-License-Identifier: GPL-3.0-only`.

## Comments

- A comment carries _why_, never _what_: the intent, the constraint, the rejected alternative, the
  consequence worth warning about. If you cannot say it without restating the code, the code is what
  needs fixing — a clearer name or a smaller function beats a comment every time, and the best
  comment is the one you found a way not to write.
- An inaccurate or misleading comment is worse than none: it is read as truth and outlives the code
  it described. When a comment and its code disagree, the code is right — fix or delete the comment.
- Keep a comment next to the code it describes, about that code, and make the link to it obvious. Do
  not reach across the system from a local comment, and do not restate the obvious.
- Put the long rationale in the module's `//!` doc; keep an inline `//` to a line or two. Never
  narrate change history (the commit and the PR own that), leave commented-out code, sign a comment,
  or comment an item only because it exists.
- Document a public crate API; do not give every private helper a header — a short function with a
  good name says more. A `// TODO` may mark known work, but it is not an excuse to leave code broken.
- The file header (copyright / SPDX) is the one comment that is not about the code.


## UI conventions

- Slint: properties and bindings, `callback` for events, `@tr()` for translated strings,
  `@image-url` for assets.
- **Never** add `cursor: pointer`; this is a desktop app.
- Rust touches a Slint component only from the event loop. From another thread, go through
  `upgrade_in_event_loop` — a `Weak` crosses threads, a strong handle cannot.
- Keep the winit backend hook (`app/src/support/native/`, reached through `native::install_backend`)
  as the only place window attributes are set. It has to run before any winit window exists, which
  is why `main` calls it before `App::new`.
- Translations go through `@tr()`; never hardcode user-visible text. Add the string to all 12
  catalogues via `tools/update-i18n.py`.

## License

GPL-3.0-only with GPLv3 §7 additional terms: modified distributions must rename the software, keep
copyright notices, and not hold the authors jointly liable. The terms themselves are not expressible
as an SPDX identifier, so they travel as the `LICENSE` text the packages install; `cargo-rpm` reads
`License:` from `package.license` and has no override for it.
