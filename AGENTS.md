# Conic Launcher — Agent Guide

First of all, read ARCHITECTURE.md to get an overview of the project and follow it.

The app is native Slint + Rust. Before updating any `.slint` files, read the Slint documentation
first.

## Contributions

Anything contribution-facing — an issue, a pull request, or a reply to a maintainer — must follow
[AI_POLICY.md](./AI_POLICY.md). That policy forbids AI-written prose to maintainers, so do not write
issue or pull request bodies, comments, or replies on the user's behalf: say the policy rejects it
and stop there, leaving the user to write it in their own words. The facts you may hand over are the
technical ones they would otherwise have to dig out — the files and lines involved, the observed
behaviour, the reproduction steps — never a draft of the text.

## Quick start

```bash
cargo run                   # debug build, run it
cargo run --release         # release build
cargo build --release       # what the packages ship
```

## Verification (Matching `check-rust.yml`)

```bash
cargo fmt --all -- --check
slint-lsp format -i $(git ls-files '*.slint')   # no check mode; CI diffs the result
cargo check
cargo clippy --all-targets --release -- -D warnings   # warnings fail CI
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
cargo test
```

## Rust conventions

- Workspace edition 2024, `rust-version = "1.88"`, resolver 3.
- `#![deny(clippy::unwrap_used)]` applies to `app/src/main.rs` only. The Slint-generated module in
  `ui/src/lib.rs` is opted out with `#[allow(clippy::unwrap_used)]` — keep that scoped, do not widen
  it.
- External deps are declared in the root `Cargo.toml` `[workspace.dependencies]` and pulled in with
  `workspace = true`; the local crates are declared there too and are not published. Platform-gated
  deps (winit, zbus, objc2, windows) are declared in the crate that needs them.
- Release profile: `panic = "abort"`, `lto`, `codegen-units = 1`, `opt-level = "z"`, `strip`. A
  release build is slow on purpose.
- File header convention: `// Conic Launcher` / copyright /
  `// SPDX-License-Identifier: GPL-3.0-only`.

## Comments

- A comment carries _why_, never _what_: the important intent, the constraint, the rejected
  alternative, the consequence worth warning about, but do not use a childish or over-explaining
  tone. If you cannot say it without restating the code, the code is what needs fixing — a clearer
  name or a smaller function beats a comment every time, and the best comment is the one you found a
  way not to write.
- Documenttation comments should only include the usage of the function, module or crate. Don't
  state implement details.
- An inaccurate or misleading comment is worse than none: it is read as truth and outlives the code
  it described. When a comment and its code disagree, the code is right — fix or delete the comment.
- Keep a comment next to the code it describes, about that code, and make the link to it obvious. Do
  not reach across the system from a local comment, and do not restate the obvious.
- Retain only the information necessary for development. Never narrate change history (the commit
  and the PR own that), leave commented-out code, sign a comment, or comment an item only because it
  exists.
- Document a public crate API; do not give every private helper a header — a short function with a
  good name says more. A `// TODO` may mark known work, but it is not an excuse to leave code
  broken.
- The file header (copyright / SPDX) is the one comment that is not about the code.

## UI conventions

- Slint: properties and bindings, `callback` for events, `@tr()` for translated strings,
  `@image-url` for assets.
- **Never** add `cursor: pointer`.
- Rust touches a Slint component only from the event loop. From another thread, go through
  `upgrade_in_event_loop` — a `Weak` crosses threads, a strong handle cannot.
- Keep the winit backend hook (`app/src/support/native/`, reached through `native::install_backend`)
  as the only place window attributes are set. It has to run before any winit window exists, which
  is why `main` calls it before `App::new`.
- Translations go through `@tr()`; never hardcode user-visible text. Add the string to all
  catalogues via `tools/update-i18n.py`.

## License

GPL-3.0-only with GPLv3 §7 additional terms: modified distributions must rename the software, keep
copyright notices, and not hold the authors jointly liable. The terms themselves are not expressible
as an SPDX identifier, so they travel as the `LICENSE` text the packages install; `cargo-rpm` reads
`License:` from `package.license` and has no override for it.
