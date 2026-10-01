# Conic Launcher — Agent Guide

> **The app is native Slint + Rust.** It was Tauri + Vue until the migration
> completed: `core/`, `src/`, `slint/` and the whole `pnpm` toolchain are gone.
> `docs/slint-migration.md` still describes the *old* layout and a `slint/`
> directory that no longer exists — treat it as history, not as a map.

## Quick start

```bash
cargo run -p conic-launcher                    # debug build, run it
cargo run -p conic-launcher --release          # release build
cargo build --release --locked -p conic-launcher   # what the packages ship
```

There is no frontend build step. Debug builds use the `~/.conic-debug` data
directory; release builds use `~/.conic` (see `folder::DATA_LOCATION`).

Delete `~/.conic-debug` by hand to reset a debug install.

## Verification

```bash
cargo fmt --all -- --check
cargo check
cargo clippy --all-targets --release -- -D warnings   # warnings fail CI
cargo test
```

Matching `check-rust.yml`, which runs `fmt` on Linux, and `check` + `clippy`
(`--all-targets --release -- -D warnings`) on Linux **and** Windows, plus a
"test" job that is really `cargo check --all --all-targets`.

Python tooling in `tools/` is not part of any CI gate:

| Script                     | Does                                                        |
| -------------------------- | ----------------------------------------------------------- |
| `generate-icons.py`        | writes `packaging/linux/icons/` (committed; see Packaging)  |
| `check-icon-names.py`      | every `AppIcon`/`icon:` name in a `.slint` has an SVG      |
| `update-i18n.py`           | re-keys the `.po` catalogues against `slint-tr-extractor`   |
| `merge-digit-font.py`, `verify_merge.py` | build the merged digit font                    |

## Architecture

- **`app/`** — the application crate (`conic-launcher`), the binary everything
  ships. `app/src/*.rs` is the wiring layer between the Rust crates and the UI;
  `app/ui/**.slint` is the whole interface; `app/build.rs` compiles the `.slint`
  tree and generates `app/ui/icons.slint` from the SVGs under
  `app/ui/assets/icons/`. The generated file is gitignored.
- **`crates/*/`** — Rust domain crates, one per capability, all
  Tauri-agnostic. The frontend talks to them directly; there is no IPC layer.
- **`packaging/`** — see Packaging.
- **`app/i18n/<locale>/LC_MESSAGES/conic-launcher.po`** — 12 locales.
  `tools/update-i18n.py` owns them; do not hand-edit.

### Platform plumbing worth knowing

- `crates/platform` — OS detection (`PLATFORM_INFO`, `OsFamily`).
- `crates/window` — window operations (minimize/maximize/fullscreen,
  `bring_to_front`) and the winit window-attributes hook. macOS gets a
  transparent title bar with the real traffic lights; Windows gets
  `with_decorations(false)` plus `app/src/windows_caption.rs` drawing the
  controls itself; Linux draws them in the title bar.
- `crates/single-instance` — the second launch is not a second window. Linux uses
  D-Bus (`zbus`), macOS a socket, Windows a named mutex.
- `crates/authcode` — the loopback listener the Microsoft browser flow hands its
  authorization code back on.
- `crates/music` — local music listing **and** playback. `cpal` decodes and plays
  natively; the Vue version used the Web Audio API.
- `crates/markdown` — `comrak` + `parley`/`fontique` render a content panel's
  body and push a display list into the Slint model.

## Crate map

| Crate             | Purpose                                                  |
| ----------------- | -------------------------------------------------------- |
| `account`         | Microsoft / offline / Authlib / Yggdrasil accounts       |
| `authcode`        | Loopback listener for the Microsoft login callback        |
| `config`          | App config load/save, background image                   |
| `content`         | Saves, datapacks, resourcepacks, screenshots, mods       |
| `curseforge`      | CurseForge API client (`CURSEFORGE_API_KEY` at build time) |
| `download`        | Generic downloader tasks                                 |
| `folder`          | Data directory layout (`DATA_LOCATION`)                  |
| `install`         | Minecraft + loader installation                          |
| `instance`        | Instance CRUD, playtime                                  |
| `java-runtime`    | Java scanning/parsing                                    |
| `launch`          | Game launch with progress reporting                      |
| `markdown`        | Markdown/HTML body of a content detail panel             |
| `modrinth`        | Modrinth API client                                      |
| `multiplayer`     | Conic Nexus cross-LAN multiplayer                        |
| `music`           | Local music files + playback                             |
| `platform`        | OS detection                                             |
| `shared`          | Common types/utilities (the shared `HTTP_CLIENT`)        |
| `single-instance` | One instance per machine                                 |
| `statistics`      | Playtime statistics                                      |
| `version`         | Minecraft version metadata                               |
| `window`          | Window operations and the winit backend hook             |

## Rust conventions

- Workspace edition 2024, `rust-version = "1.88"`, resolver 3.
- `#![deny(clippy::unwrap_used)]` applies to `app/src/main.rs` only. The
  Slint-generated module is opted out with `#[allow(clippy::unwrap_used)]` —
  keep that scoped, do not widen it.
- All external deps live in the root `Cargo.toml` `[workspace.dependencies]`; the
  local crates are declared there too and are not published.
- Release profile: `panic = "abort"`, `lto`, `codegen-units = 1`,
  `opt-level = "z"`, `strip`. A release build is slow on purpose.
- `slint` is pulled with `default-features = true`, which includes the Skia
  renderer. Skia resolves its binaries through `skia-bindings`' `binary-cache`,
  so the build downloads a prebuilt and needs no `gn`/`ninja`/`clang`.
- File header convention: `// Conic Launcher` / copyright /
  `// SPDX-License-Identifier: GPL-3.0-only`.
- Comment density is a house style here: explain *why*, and the alternatives that
  were rejected. Match it.

## UI conventions

- Slint, not Vue: properties and bindings, `callback` for events, `@tr()` for
  translated strings, `@image-url` for assets.
- **Never** add `cursor: pointer`; this is a desktop app.
- Rust touches a Slint component only from the event loop. From another thread,
  go through `upgrade_in_event_loop` — a `Weak` crosses threads, a strong handle
  cannot.
- Keep the winit backend hook (`app/src/main.rs`) as the only place window
  attributes are set. It has to run before any winit window exists.
- Translations go through `@tr()`; never hardcode user-visible text. Add the
  string to all 12 catalogues via `tools/update-i18n.py`.

## Testing

Rust unit tests exist only where there is logic worth isolating (`install`,
`java-runtime`, `markdown`); run `cargo test`. There is no UI test framework in
use beyond `i-slint-backend-testing` as a dev-dependency of `app`.

## Versioning & releases

The app version is `version` in **`app/Cargo.toml`** — the single source. It used
to be `core/tauri.conf.json`. `build.yml` reads it with `sed` (no toolchain
needed in the version job), builds the Linux packages on push to `master`, and
publishes a release only when the version differs from `HEAD~1`. README asks
contributors to target `dev`; releases are cut from `master`.

The Tauri self-updater and its signing keys are **gone**: there is no `.sig`
artifact and no `TAURI_SIGNING_PRIVATE_KEY` anywhere. `CURSEFORGE_API_KEY` is
still read at build time by `crates/curseforge/build.rs`; without it the official
CurseForge API is simply unauthenticated.

## Packaging

`packaging/linux/` holds the **only** copy of the payload the Linux formats
install — the desktop entry and the hicolor icon set. `packaging/macos/` holds
the `.icns` and the `Info.plist` template for the `.app`. Both are **committed**,
regenerated by `tools/generate-icons.py` / `tools/generate-icns.py`; do not add a
step that needs Python at package time, because `dpkg-deb`, `rpmbuild` and
`makepkg` run on machines without it, and `iconutil` only exists on macOS.

```bash
tools/package-linux.sh              # deb + rpm + AppImage
tools/package-linux.sh deb rpm      # a subset

tools/package-macos.sh              # .app + .dmg, native arch
tools/package-macos.sh --universal  # both arches in one binary (local only; CI
                                    # builds each natively instead — see macOS)
```

Linux's three are assembled by `dpkg-deb`, `rpmbuild` and `appimagetool`, none of
which exist elsewhere, so `package-linux.sh` refuses to run off Linux. Needs
`cargo-deb`, `cargo-rpm`, `appimagetool`, `file`, and `dpkg-dev` (for
`dpkg-shlibdeps`, without which `depends = "$auto"` resolves to nothing
*silently*). Both scripts write to `<target-dir>/package/`.

| Format       | Recipe                                                        |
| ------------ | ------------------------------------------------------------- |
| `.deb`       | `[package.metadata.deb]` in `app/Cargo.toml`                  |
| `.rpm`       | `[package.metadata.rpm]` + the spec at `app/.rpm/conic-launcher.spec` |
| `.AppImage`  | the AppDir `tools/package-linux.sh` assembles, then `appimagetool` |
| `.app`/`.dmg`| the bundle `tools/package-macos.sh` assembles, then `hdiutil`  |
| Arch         | `packaging/arch/PKGBUILD` — `makepkg -si`                      |

Four things that are easy to break here:

1. **The spec is required.** `cargo rpm` reads `app/.rpm/<name>.spec` and
   substitutes only `@@VERSION@@` / `@@RELEASE@@`. There is no embedded fallback,
   so a missing spec fails the build rather than producing a package. Its
   `%files` must list everything the package installs, or rpmbuild fails on
   "Installed (but unpackaged) file(s) found" — cargo-rpm's own generated
   template only lists `%{_bindir}/*`, which is why this one is hand-written.
2. **`app/.rpm/usr/` is generated, `.rpm/conic-launcher.spec` is not.** The
   script stages the payload there; the file next to it is committed.
3. **`target/release/` in `[package.metadata.deb]` `assets` is literal.** It is
   cargo-deb's marker for "the binary cargo built"; a real path makes it package a
   stale file.
4. **`Icon=conic-launcher` must keep matching.** Every payload file is installed
   under the name `conic-launcher` whatever its size; break that and the packages
   install with an invisible launcher.

Three more, each of which cost a build to find:

5. **`app/Cargo.toml` must spell `license` out.** `cargo rpm` reads the manifest
   with its own deserialiser, and `license.workspace = true` makes it a table
   where it wants a string — the whole file then fails to parse with
   `invalid type: map, expected a string for key 'package'`, which names neither
   `license` nor the field at fault. There is a comment in the file.
6. **Neither deb nor rpm can see a `dlopen`ed library.** `$auto` and
   `find-requires` only read the ELF's `NEEDED` entries, which for this binary
   are `libc`, `libgcc_s`, `libm`, `libfontconfig1` and `libasound2`. GL,
   xkbcommon, X11 and Wayland are all `dlopen`ed by glutin, winit and Skia, and
   are listed by hand. A dependency scanner will never tell you this for you.
7. **Every format spells the version differently.** `0.1.0-alpha.2` ships as
   `0.1.0~alpha.2-1` in the deb (cargo-deb maps `-` to `~`; a Debian version may
   carry only one `-`), as `Version: 0.1.0` / `Release: 0.alpha.2` in the rpm
   (cargo-rpm splits on the first `-`, which is what makes the pre-release sort
   below the final), as `0.1.0` + a build of `0.1.0-alpha.2` in the macOS bundle
   (`CFBundleShortVersionString` must be dotted integers), and verbatim in the
   AppImage and the `.dmg`. Never hardcode a filename.

8. **The macOS bundle is signed ad-hoc, not notarized.** `codesign -s -` is all a
   build machine can do without a Developer ID certificate, and it is what makes
   the binary executable at all on arm64. Gatekeeper still blocks the result on
   anyone else's machine, so a user has to clear the quarantine flag themselves.
   Distributing it properly needs a certificate and notarization credentials
   this repository does not have. Note the consequence for CI: with Gatekeeper
   assessments off on a runner (as they are on a dev machine), `spctl --assess`
   returns success whatever the signature, so it proves nothing — `codesign
   --verify --deep --strict` is the check that is actually meaningful.

`tools/package-linux.sh` needs Linux: `dpkg-deb`, `rpmbuild`, `dpkg-shlibdeps`
and `appimagetool` have no counterpart elsewhere, and it says so rather than
failing three steps in.

## macOS

Tauri's bundler is gone, so `tools/package-macos.sh` assembles the `.app` and
`.dmg` itself out of `codesign`, `hdiutil` and `lipo`. Three things worth knowing
before touching either script:

- **`x86_64-apple-darwin` did not compile until this was fixed.** `objc-sys`
  types an Objective-C `BOOL` as `bool` on arm64 (where C's `BOOL` is `_Bool`)
  and as `i8` on x86_64 (where it is `signed char`), so code that returns one
  straight out of an FFI call builds for Apple Silicon and fails for Intel with
  `expected bool, found i8`. `app/src/traffic_lights.rs`'s `add_method` is the
  one place this bites; it goes through `i8` to satisfy both. A `--universal`
  build is what surfaces this, and it is why the CI job builds one.
- **CI builds each macOS architecture on a native runner** (`macos-15` and
  `macos-15-intel`), not one runner cross-compiling the other. A `--target`
  build compiles for the guest while running every build script on the host, so
  a host-only mistake in a `-sys` crate passes and a real Intel build fails — and
  Skia and `aws-lc-rs` are both built through `cc`. Two thin `.dmg`s beat one
  universal one for that reason. `--universal` is still there for a local build.
- **`CFBundleIdentifier` must stay `app.conicmc.launcher.slint`**, matching
  `single_instance::APP_ID`. macOS derives the permission prompts and the
  notification identity from it.

### Linux build dependencies

Not declared anywhere in the build, so a missing one is a link error in the
middle of a release build:

```
pkg-config  libfontconfig1-dev  libasound2-dev  libudev-dev
```

(fontconfig via `fontique`, ALSA via `cpal`, libudev via
`i-slint-backend-linuxkms`). GL, xkbcommon and X11 are `dlopen`ed by glutin,
winit and Skia — runtime dependencies of the package, not build dependencies of
the crate. At runtime the app also shells out to `xdg-open`, `dbus-send` and
(only for the background picker) `zenity`.

## License

GPL-3.0-only with GPLv3 §7 additional terms: modified distributions must rename the
software, keep copyright notices, and not hold the authors jointly liable. The
terms themselves are not expressible as an SPDX identifier, so they travel as the
`LICENSE` text the packages install; `cargo-rpm` reads `License:` from
`package.license` and has no override for it.