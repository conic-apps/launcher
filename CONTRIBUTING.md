# Contributing to Conic Launcher

Thank you for your interest in contributing to Conic Launcher! There are many ways to contribute and
we appreciate all of them.

## Fixing a bug or improving a feature

Generally it's fine to just work on these kinds of things and put a pull-request out for it. If there
is an issue accompanying it make sure to link it in the pull request description so it can be closed
afterwards or linked for context.

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

To set up your development environment, you'll need Rust installed. If you don't have Rust yet, follow
the official installation guide: <https://www.rust-lang.org/tools/install>.

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

Before submitting a pull request, please make sure the code is formatted, passes
lints, and builds clean docs:

```bash
cargo fmt --all -- --check
cargo check
cargo clippy --all-targets --release -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
cargo test
```

Every `.rs`, `.slint`, `.py` and `.sh` file needs its SPDX header — `hawkeye
check` verifies them and `hawkeye format` writes the missing ones. A new
dependency must pass the licence allowlist in `deny.toml` (`cargo deny check
licenses`), which CI runs on any change to `Cargo.toml` or `Cargo.lock`.

## Testing

Rust unit tests live beside the code they cover, under `app/src/` and in several
crates; run `cargo test`. There is no UI test framework in use beyond
`i-slint-backend-testing` as a dev-dependency of `app`.

## Translations

There are two translation surfaces, and they are updated differently:

- **In-app strings** live in `app/i18n/<locale>/LC_MESSAGES/conic-launcher.po`,
  one catalogue per language. `tools/update-i18n.py` re-keys them against
  `slint-tr-extractor`; do not hand-edit them. Adding a string to the UI means
  adding it to all twelve catalogues through that tool.
- **The READMEs and the desktop entry's `Comment`** are hand-maintained. The
  English [`README.md`](./README.md) is the source: when it changes, mirror the
  same structure into `docs/readme/README.<locale>.md`. The desktop entry's
  `Comment` carries one line per language; see
  [`packaging/linux/README.md`](./packaging/linux/README.md) for which locales
  need a region suffix.

Machine translation is welcome in both places — see the [AI Policy](./AI_POLICY.md).

## Versioning & releases

The app version is `version` in **`app/Cargo.toml`** — the single source.
`build.yml` reads it with `sed` (no toolchain needed in the version job), builds
the Linux packages on push to `master`, and publishes a release only when the
version differs from `HEAD~1`. Pull requests target `dev`; releases are cut
from `master`.

`CURSEFORGE_API_KEY` is read at build time by `crates/curseforge/build.rs`;
without it the official CurseForge API is simply unauthenticated.

## Packaging

`packaging/` holds the payload each format installs, one directory per platform.
The `README.md` in each documents its contents and what is easy to break there:

- `packaging/linux/` — the desktop entry and hicolor icon set, the only copy the
  deb, rpm, AppImage and Arch package install.
- `packaging/macos/` — the `.icns` and the `Info.plist` template.
- `packaging/windows/` — the `.wxs` and the `.ico`.
- `packaging/arch/` — the `PKGBUILD` and its `.SRCINFO`.

The scripts that assemble the artifacts, all writing to `<target-dir>/package/`:

```bash
tools/package-linux.sh              # deb + rpm + AppImage
tools/package-linux.sh deb rpm      # a subset

tools/package-macos.sh              # .app + .dmg, native arch
tools/package-macos.sh --universal  # both arches in one binary (local only; CI
                                    # builds each natively instead — see macOS)

pwsh tools/package-windows.ps1      # .exe + .msi, native arch
pwsh tools/package-windows.ps1 msi  # a subset
```

| Format        | Recipe                                                                |
| ------------- | --------------------------------------------------------------------- |
| `.msi`/`.exe` | `tools/package-windows.ps1`, see Windows                              |
| `.deb`        | `[package.metadata.deb]` in `app/Cargo.toml`                          |
| `.rpm`        | `[package.metadata.rpm]` + the spec at `app/.rpm/conic-launcher.spec` |
| `.AppImage`   | the AppDir `tools/package-linux.sh` assembles, then `appimagetool`    |
| `.app`/`.dmg` | the bundle `tools/package-macos.sh` assembles, then `hdiutil`         |
| Arch          | `packaging/arch/PKGBUILD` — `makepkg -si`                             |

One rule spans every format: **each spells the version differently.**
`0.1.0-alpha.2` ships as `0.1.0~alpha.2-1` in the deb (cargo-deb maps `-` to `~`;
a Debian version may carry only one `-`), as `Version: 0.1.0` / `Release:
0.alpha.2` in the rpm (cargo-rpm splits on the first `-`, which is what makes the
pre-release sort below the final), as `0.1.0` + a build of `0.1.0-alpha.2` in the
macOS bundle (`CFBundleShortVersionString` must be dotted integers), and verbatim
in the AppImage, the `.dmg` and the Arch package. Never hardcode a filename.

`tools/package-linux.sh` needs Linux: `dpkg-deb`, `rpmbuild`, `dpkg-shlibdeps`
and `appimagetool` have no counterpart elsewhere, and it says so rather than
failing three steps in.

### Windows

`tools/package-windows.ps1` assembles the `.msi` from the WiX toolset and copies
the standalone `.exe` beside it; `packaging/windows/README.md` is that recipe in
detail, including the `.msi`-specific traps. The script and its CI job carry a
few decisions the payload README does not:

- **PowerShell, not bash.** `wix`, `msiexec` and `dotnet` are Windows programs,
  and a bash script driving them spends its length fighting Git-Bash's path
  rewriting on exactly the strings the installer needs verbatim.
- **WiX 6 is installed as a pinned .NET tool on both runners, not taken from the
  image.** `windows-2025` carries WiX v3.14.1, `windows-11-arm` carries none, and
  WiX v3 is EOL — the image's copy would mean two dialects for one `.wxs`. The
  version is pinned (6.0.2): an installer that builds one week and not the next is
  not a reproducible release.
- **`WixToolset.UI.wixext` is installed explicitly.** Without it the MSI builds
  and installs with no interface at all, so its absence is not an error.
- **CI builds each architecture natively** (`windows-2025`, `windows-11-arm`),
  for the reason `packaging/macos/README.md` gives. The script therefore has no
  `--target` and reads its architecture out of `rustc -vV`.
- **The workflow installs and uninstalls the MSI.** `wix` validates XML;
  `msiexec` is what refuses a package the processor type does not support, and it
  is the only check that finds the packaged binary is the one that was built.

### macOS

`tools/package-macos.sh` assembles the `.app` and `.dmg` itself out of
`codesign`, `hdiutil` and `lipo`; `packaging/macos/README.md` covers the payload,
the native-runner build and the signing gap. One thing worth knowing before
touching the script:

- **`x86_64-apple-darwin` needs care here.** `objc-sys` types an Objective-C
  `BOOL` as `bool` on arm64 (where C's `BOOL` is `_Bool`) and as `i8` on x86_64
  (where it is `signed char`), so code that returns one straight out of an FFI
  call builds for Apple Silicon and fails for Intel with `expected bool, found
  i8`. `app/src/support/native/macos/traffic_lights.rs`'s `add_method` is the
  one place this bites; it goes through `i8` to satisfy both. A `--universal`
  build is what surfaces this.

### Linux build dependencies

Not declared anywhere in the build, so a missing one is a link error in the
middle of a release build:

```
pkg-config  libfontconfig1-dev  libasound2-dev  libudev-dev
```

(fontconfig via `fontique`, ALSA via `cpal`, libudev via
`i-slint-backend-linuxkms`). GL, xkbcommon and X11 are `dlopen`ed by glutin,
winit and Skia — runtime dependencies of the package, not build dependencies of
the crate. At runtime the app also shells out to `xdg-open` and `dbus-send`;
file and folder pickers go through the XDG desktop portal (`rfd`), whose file
chooser `xdg-desktop-portal` implements.
