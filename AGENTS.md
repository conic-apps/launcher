# Conic Launcher — Agent Guide

The app is native Slint + Rust. Before updating any `.slint` files, read the Slint documentation first.

## Quick start

```bash
cargo run                   # debug build, run it
cargo run --release         # release build
cargo build --release       # what the packages ship
```

The launcher resolves three storage roots at startup — the launcher's own data,
the shared Minecraft install and the instances — under `storage::LOCATIONS`. By
default they hang off one root, with the launcher data at the root itself and
`minecraft/` and `instances/` under it. The root is platform and packaging
dependent: `~/Library/Application Support/app.conicmc.launcher` on macOS,
`$XDG_DATA_HOME/conic` (falling back to `~/.local/share/conic`) on Linux, and
`%APPDATA%\conic` on Windows — except a single-file portable Windows build, which
keeps everything in `.minecraft-conic` next to the executable. Debug builds
append `-debug` to the folder name.

`Settings → General → Data storage` redirects each root. That choice cannot live
in `config.toml`, because the config file is inside the launcher location, so it
is written to a `locations.toml` bootstrap file at the platform anchor and applied
on the next start.

A first run (`Config.setup_completed` is `false`) opens the setup wizard. Its
storage step, right after the language step, offers the same three roots;
finishing writes the config into the chosen launcher directory, moves the data
there and restarts, so a mid-wizard location change does not lose the steps
before it. Closing the wizard keeps the defaults and marks it done.

Delete the debug folder by hand to reset a debug install.

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

Python tooling in `tools/` is not part of any CI gate:

| Script                                   | Does                                                       |
| ---------------------------------------- | ---------------------------------------------------------- |
| `generate-icns.py`                       | writes `packaging/macos/conic-launcher.icns` (committed)   |
| `generate-ico.py`                        | writes `packaging/windows/conic-launcher.ico` (committed)  |
| `generate-icons.py`                      | writes `packaging/linux/icons/` (committed; see Packaging) |
| `check-icon-names.py`                    | every `AppIcon`/`icon:` name in a `.slint` has an SVG      |
| `update-i18n.py`                         | re-keys the `.po` catalogues against `slint-tr-extractor`  |
| `merge-digit-font.py`, `verify_merge.py` | build and verify the merged font                           |

## Architecture

- **`app/`** — the application crate (`conic-launcher`), the binary everything
  ships. `app/src/*.rs` is the wiring layer between the Rust crates and the UI;
  `app/ui/**.slint` is the whole interface; `app/build.rs` compiles the `.slint`
  tree and generates `app/ui/icons.slint` from the SVGs under
  `app/ui/assets/icons/`. The generated file is gitignored.
- **`crates/*/`** — Rust domain crates, one per capability, independent of the
  UI. The interface talks to them directly; there is no IPC layer.
- **`packaging/`** — see Packaging.
- **`app/i18n/<locale>/LC_MESSAGES/conic-launcher.po`** — 12 locales.
  `tools/update-i18n.py` owns them; do not hand-edit.

### Platform plumbing worth knowing

- `crates/platform` — OS detection (`PLATFORM_INFO`, `OsFamily`).
- `crates/storage` — the three storage roots (`LOCATIONS`), which resolve the
  launcher data, the shared Minecraft install and the instances from the platform
  default plus the `locations.toml` bootstrap file.
- `crates/window` — window operations (minimize/maximize/fullscreen,
  `bring_to_front`) and the winit event-filter fan-out. macOS gets a
  transparent title bar with the real traffic lights; Windows gets
  `with_decorations(false)` plus `app/src/windows_caption.rs` drawing the
  controls itself; Linux draws them in the title bar.
- `crates/single-instance` — the second launch is not a second window. Linux uses
  D-Bus (`zbus`), macOS a socket, Windows a named mutex.
- `crates/authcode` — the loopback listener the Microsoft browser flow hands its
  authorization code back on.
- `crates/music` — local music listing **and** playback. `symphonia` decodes and
  `cpal` plays natively.
- `crates/markdown` — `comrak` + `parley`/`fontique` render a content panel's
  body and push a display list into the Slint model.

## Crate map

| Crate             | Purpose                                                    |
| ----------------- | ---------------------------------------------------------- |
| `account`         | Microsoft / offline / Authlib / Yggdrasil accounts         |
| `authcode`        | Loopback listener for the Microsoft login callback         |
| `config`          | App config load/save, background image                     |
| `content`         | Saves, datapacks, resourcepacks, screenshots, mods         |
| `curseforge`      | CurseForge API client (`CURSEFORGE_API_KEY` at build time) |
| `download`        | Generic downloader tasks                                   |
| `storage`         | Data directory layout (`LOCATIONS`)                       |
| `install`         | Minecraft + loader installation                            |
| `instance`        | Instance CRUD, playtime                                    |
| `java-discovery`  | Java discovery/parsing                                     |
| `launch`          | Game launch with progress reporting                        |
| `markdown`        | Markdown/HTML body of a content detail panel               |
| `modrinth`        | Modrinth API client                                        |
| `multiplayer`     | Conic Nexus cross-LAN multiplayer                          |
| `music`           | Local music files + playback                               |
| `platform`        | OS detection                                               |
| `shared`          | Common types/utilities (the shared `HTTP_CLIENT`)          |
| `single-instance` | One instance per machine                                   |
| `statistics`      | Playtime statistics                                        |
| `version`         | Minecraft version metadata                                 |
| `window`          | Window operations and the winit backend hook               |

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
- Keep the winit backend hook (`app/src/main.rs`) as the only place window
  attributes are set. It has to run before any winit window exists.
- Translations go through `@tr()`; never hardcode user-visible text. Add the
  string to all 12 catalogues via `tools/update-i18n.py`.

## Testing

Rust unit tests live beside the code they cover, under `app/src/` and in several
crates; run `cargo test`. There is no UI test framework in use beyond
`i-slint-backend-testing` as a dev-dependency of `app`.

## Versioning & releases

The app version is `version` in **`app/Cargo.toml`** — the single source.
`build.yml` reads it with `sed` (no toolchain needed in the version job), builds
the Linux packages on push to `master`, and publishes a release only when the
version differs from `HEAD~1`. README asks contributors to target `dev`;
releases are cut from `master`.

`CURSEFORGE_API_KEY` is read at build time by `crates/curseforge/build.rs`;
without it the official CurseForge API is simply unauthenticated.

## Packaging

`packaging/linux/` holds the **only** copy of the payload the Linux formats
install — the desktop entry and the hicolor icon set. `packaging/macos/` holds
the `.icns` and the `Info.plist` template for the `.app`. Both directories are
**committed**; the icon sets are regenerated by `tools/generate-icons.py` /
`tools/generate-icns.py`, while the desktop entry and `Info.plist` are
hand-written. Do not add a step that needs Python at package time, because
`dpkg-deb`, `rpmbuild` and `makepkg` run on machines without it, and `iconutil`
only exists on macOS.

```bash
tools/package-linux.sh              # deb + rpm + AppImage
tools/package-linux.sh deb rpm      # a subset

tools/package-macos.sh              # .app + .dmg, native arch
tools/package-macos.sh --universal  # both arches in one binary (local only; CI
                                    # builds each natively instead — see macOS)

pwsh tools/package-windows.ps1      # .exe + .msi, native arch
pwsh tools/package-windows.ps1 msi  # a subset
```

Linux's three are assembled by `dpkg-deb`, `rpmbuild` and `appimagetool`, none of
which exist elsewhere, so `package-linux.sh` refuses to run off Linux. Needs
`cargo-deb`, `cargo-rpm`, `appimagetool`, `file`, and `dpkg-dev` (for
`dpkg-shlibdeps`, without which `depends = "$auto"` resolves to nothing
_silently_). All three scripts write to `<target-dir>/package/`.

| Format        | Recipe                                                                |
| ------------- | --------------------------------------------------------------------- |
| `.msi`/`.exe` | `tools/package-windows.ps1`, see Windows                              |
| `.deb`        | `[package.metadata.deb]` in `app/Cargo.toml`                          |
| `.rpm`        | `[package.metadata.rpm]` + the spec at `app/.rpm/conic-launcher.spec` |
| `.AppImage`   | the AppDir `tools/package-linux.sh` assembles, then `appimagetool`    |
| `.app`/`.dmg` | the bundle `tools/package-macos.sh` assembles, then `hdiutil`         |
| Arch          | `packaging/arch/PKGBUILD` — `makepkg -si`                             |

Four things that are easy to break here, all of them Linux:

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

9. **An MSI's version field cannot hold a pre-release.** `Package/@Version` is
   three integers, so `0.1.0-alpha.2` and `0.1.0-alpha.3` are one *version* to
   Windows Installer and are distinguishable only by `ProductCode`, which
   `tools/package-windows.ps1` derives from the full Cargo version. Two
   consequences, both of them load-bearing:
   `MajorUpgrade/@AllowSameVersionUpgrades="yes"` or the second of two
   same-version packages installs *beside* the first instead of replacing it, and
   the script refuses a Cargo version it cannot split rather than writing `0.1`
   into the field.
10. **WiX's v4+ schema is not the v3 one.** The `.wxs` uses
    `Package/@ProductCode` (there is no `@PackageCode`),
    `<SummaryInformation Description="…" />` (there is no `<Description>` child of
    `<Package>`), and a `<MajorUpgrade>` whose `AllowSameVersionUpgrades` and
    `DowngradeErrorMessage` are *both* required and cannot be swapped for
    `AllowDowngrades`. Each wrong spelling is a hard `WIX0004`/`WIX0010`/`WIX0035`
    at build time, which is the good case; the trap is that the v3
    `candle`/`light` pair in the `windows-2025` image would accept neither file as
    it stands.
11. **The `.ico`'s frames have to be BMPs, and WiX does not check.** The MSI
    `Icon` table stores `BITMAPINFOHEADER` DIBs. Pillow writes PNG frames by
    default, and WiX copies them into the table unconverted with the build
    succeeding — so the only symptom is an installer that cannot draw its own
    icon. `tools/generate-ico.py` forces `bitmap_format="bmp"` and verifies what
    it wrote.
12. **`ProgramFilesFolder` is the *32-bit* folder.** `ProgramFiles64Folder` is
    the 64-bit one, and neither Windows Installer nor WiX translates the first for
    the package's platform: a package built `wix build -arch x64`, whose summary
    `Template` reads `x64;1033`, installs into `C:\Program Files (x86)` and
    reports success with exit code 0. Nothing in the tables says so either — the
    `Directory` row reads `PFiles` either way. This is the one thing about the
    `.msi` that only installing it can find, which is why `build.yml` installs
    and uninstalls it rather than reading its tables and calling it a day.

`tools/package-linux.sh` needs Linux: `dpkg-deb`, `rpmbuild`, `dpkg-shlibdeps`
and `appimagetool` have no counterpart elsewhere, and it says so rather than
failing three steps in.

## Windows

`tools/package-windows.ps1` assembles the `.msi` out of the WiX toolset and
copies the standalone `.exe` beside it. The
payload is `packaging/windows/` — the `.wxs` and the `.ico`, both committed — and
is read the way `app/Cargo.toml`'s `[package.metadata.deb]` is on Linux. See
`packaging/windows/README.md` for the per-file detail.

- **PowerShell, not bash**, unlike the other two scripts. Not a preference: `wix`,
  `msiexec` and `dotnet` are Windows programs, and a bash script driving them
  spends its length fighting Git-Bash's path rewriting on exactly the strings the
  installer needs verbatim. Preconditions are checked and named, the way
  `package-linux.sh` checks `dpkg-shlibdeps`.
- **WiX 6 is installed as a .NET tool on both runners rather than taken from the
  image.** `windows-2025` carries WiX v3.14.1 and `windows-11-arm` carries no WiX
  at all, so using the image's copy means two dialects and two install paths for
  two packages that come out of one `.wxs`. WiX v3 is also EOL. The version is
  pinned (6.0.2): an installer that builds one week and not the next is not a
  reproducible release.
- **`WixToolset.UI.wixext` is a separate package, and its absence is not an
  error.** Without it the MSI builds and installs with no interface at all — a
  window that flashes and an install that happens behind it — so the workflow
  installs it explicitly and the `.wxs` refers to `ui:WixUI`.
- **CI builds each architecture natively** (`windows-2025` and `windows-11-arm`),
  for the reason the macOS section gives: a `--target` build runs every build
  script on the host, so Skia and `aws-lc-rs` — both through `cc` — come out
  subtly wrong rather than failing. The script therefore has no `--target` and
  reads its architecture out of `rustc -vV`, mapping `x86_64`→`x64` and
  `aarch64`→`arm64`: Windows' own vocabulary, which is also what `wix build
  -arch` and an MSI's platform field want. A cross build from the ARM64 machine to
  x64 is possible and deliberately not offered, for the same reason
  `--universal` is only a local convenience.
- **The workflow installs and uninstalls the MSI.** `wix` validates XML;
  `msiexec` is what refuses a package the processor type does not support, and it
  is the only check in the job that finds the packaged binary is the one that was
  built. A `perMachine` package needs an elevated shell, which a runner has and a
  developer's machine may not.
- **Nothing is signed.** No certificate exists in this repository, so both
  artifacts ship unsigned and SmartScreen warns on both — the same gap the macOS
  bundle has, which `package-windows.ps1` does not paper over.
- **The `.exe` is one file, but it imports `VCRUNTIME140.dll`.** 31 of its 32 PE
  imports are Windows itself; the 32nd is the Visual C++ Redistributable, which is
  a redistributable and not an OS component. Every machine with Office or .NET has
  it and a bare one does not, and because the `.msi` installs the same executable
  this is a question about the Windows distribution as a whole rather than about
  the portable artifact. `-C target-feature=+crt-static` in a
  `.cargo/config.toml`, a WiX Burn chainer and app-local deployment are the three
  ways out; none of them is a `packaging/windows/` change, and picking one is a
  distribution decision rather than a packaging one. See
  `packaging/windows/README.md`.
- **The `.exe` carries no resource.** No `.rc`, so no embedded icon and no version
  information: Explorer shows the default icon for the file. The window icon is
  set at runtime from the embedded image and the Start menu shortcut takes its icon
  out of the MSI `Icon` table, so only Explorer's listing is affected. Fixing it
  is an `app/build.rs` change (a `winres`-style build step), not a packaging one.

## macOS

`tools/package-macos.sh` assembles the `.app` and `.dmg` itself out of
`codesign`, `hdiutil` and `lipo`. Three things worth knowing before touching
either script:

- **`x86_64-apple-darwin` needs care here.** `objc-sys`
  types an Objective-C `BOOL` as `bool` on arm64 (where C's `BOOL` is `_Bool`)
  and as `i8` on x86_64 (where it is `signed char`), so code that returns one
  straight out of an FFI call builds for Apple Silicon and fails for Intel with
  `expected bool, found i8`. `app/src/traffic_lights.rs`'s `add_method` is the
  one place this bites; it goes through `i8` to satisfy both. A `--universal`
  build is what surfaces this, and it is one reason CI builds each architecture
  on its own native runner instead of a cross-compiled universal one.
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
