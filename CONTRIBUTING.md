# Contributing to Conic Launcher

Thank you for your interest in contributing to Conic Launcher! There are many ways to contribute and
we appreciate all of them.

## Fixing a bug or improving a feature

Generally it's fine to just work on these kinds of things and put a pull-request out for it. If
there is an issue accompanying it make sure to link it in the pull request description so it can be
closed afterwards or linked for context.

## Implementing a new feature

It's advised to first open an issue for any kind of new feature so the team can tell upfront whether
the feature is desirable or not before any implementation work happens. We want to minimize the
possibility of someone putting a lot of work into a feature that is then going to waste as we deem
it out of scope (be it due to generally not fitting in with Conic Launcher, or just not having the
maintenance capacity). If there already is a feature issue open but it is not clear whether it is
considered accepted feel free to just drop a comment and ask!

## Use of AI

All use of AI in contributions must follow the [AI Policy](./AI_POLICY.md).

Contributions not following the AI Policy will be closed.

## Getting started

Before you start, please read the README and search existing issues. To get a quick overview of the
crates and structure of the project take a look at the [ARCHITECTURE.md](./ARCHITECTURE.md) manual.
You can also join our [Discord server](https://discord.gg/xWKY5NMuf7) to discuss development-related
topics.

To set up your development environment, you'll need Rust installed. If you don't have Rust yet,
follow the official installation guide: <https://www.rust-lang.org/tools/install>.

Once Rust is installed, clone the repository and build the project:

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo build
```

To run the project locally:

```bash
cargo run
```

To run the test suite:

```bash
cargo test
```

Before submitting a pull request, please make sure the code is formatted, passes lints, and builds
clean docs:

```bash
cargo fmt --all -- --check
slint-lsp format -i $(git ls-files '*.slint')   # no check mode; CI diffs the result
cargo check
cargo clippy --all-targets --release -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
cargo test
```

`.slint` files are formatted with `slint-lsp` (`cargo binstall slint-lsp`), pinned to the `slint`
version in `Cargo.lock`; CI fails on any diff. Keep it in step with `slint`: bumping `slint` without
bumping `slint-lsp` can reformat the tree.

Every `.rs`, `.slint`, `.py` and `.sh` file needs its SPDX header — `hawkeye check` verifies them
and `hawkeye format` writes the missing ones. A new dependency must pass the licence allowlist in
`deny.toml` (`cargo deny check licenses`), which CI runs on any change to `Cargo.toml` or
`Cargo.lock`.

## Testing

Rust unit tests live beside the code they cover, under `app/src/` and in several crates; run
`cargo test`. There is no UI test framework in use beyond `i-slint-backend-testing` as a
dev-dependency of `app`.

## Translations

There are two translation surfaces, and they are updated differently:

- **In-app strings** live in `ui/i18n/<locale>/LC_MESSAGES/conic-ui.po`, one catalogue per language.
  `tools/update-i18n.py` re-keys them against `slint-tr-extractor`; do not hand-edit them. Adding a
  string to the UI means adding it to all twelve catalogues through that tool.
- **The READMEs and the desktop entry's `Comment`** are hand-maintained. The English
  [`README.md`](./README.md) is the source: when it changes, mirror the same structure into
  `docs/readme/README.<locale>.md`. The desktop entry's `Comment` carries one line per language; see
  [`packaging/linux/README.md`](./packaging/linux/README.md) for which locales need a region suffix.

Machine translation is welcome in both places — see the [AI Policy](./AI_POLICY.md).

## Versioning & releases

The app version is `version` in **`app/Cargo.toml`** — the single source. `build.yml` reads it with
`sed` (no toolchain needed in the version job) and compares it against the previous commit: a push
to `dev` builds and publishes only when the version moved, so a `release: v<version>` pull request
is what cuts a version. Pull requests build without publishing, which is what the check on that pull
request reports.

`CURSEFORGE_API_KEY` is read at build time by `crates/curseforge/build.rs`; without it the official
CurseForge API is simply unauthenticated.

## Packaging

[`packaging/README.md`](./packaging/README.md) covers the scripts that assemble each format, which
file is the recipe for which one, and the Linux build dependencies; the `README.md` in each
`packaging/<platform>/` directory covers that platform's payload and its own traps. The version they
all read is `version` in `app/Cargo.toml` — see [Versioning & releases](#versioning--releases).
