# Packaging

One directory per platform, each holding the payload that platform's formats install. The
`README.md` in each documents its contents and what is easy to break there:

- [`linux/`](./linux/README.md) — the desktop entry and hicolor icon set, the only copy the deb,
  rpm, AppImage and Arch package install.
- [`macos/`](./macos/README.md) — the `.icns`, the `Info.plist` template and the disk image's window
  background.
- [`windows/`](./windows/README.md) — the `.wxs` and the `.ico`.
- [`arch/`](./arch/README.md) — the `PKGBUILD` and its `.SRCINFO`.

The scripts that assemble the artifacts, all writing to `<target-dir>/package/`:

```bash
tools/package-linux.sh              # deb + rpm + AppImage
tools/package-linux.sh deb rpm      # a subset

tools/package-macos.sh              # .app + .dmg, native arch
tools/package-macos.sh --universal  # both arches in one binary, local only; CI
                                    # builds each natively instead (see macos/)

pwsh tools/package-windows.ps1      # .exe + .msi, native arch
pwsh tools/package-windows.ps1 msi  # a subset
```

| Format        | Recipe                                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------------------ |
| `.msi`/`.exe` | `tools/package-windows.ps1`, see [windows/](./windows/README.md)                                                   |
| `.deb`        | `[package.metadata.deb]` in `app/Cargo.toml`                                                                       |
| `.rpm`        | `[package.metadata.rpm]` + the spec at `app/.rpm/conic-launcher.spec`                                              |
| `.AppImage`   | the AppDir `tools/package-linux.sh` assembles, then `appimagetool`                                                 |
| `.app`/`.dmg` | the bundle `tools/package-macos.sh` assembles, then `hdiutil`, with the window layout from `tools/dmg-ds-store.py` |
| Arch          | [`arch/PKGBUILD`](./arch/PKGBUILD) — `makepkg -si`                                                                 |

One rule spans every format: **each spells the version differently.** `0.1.0-alpha.2` ships as
`0.1.0~alpha.2-1` in the deb (cargo-deb maps `-` to `~`; a Debian version may carry only one `-`),
as `Version: 0.1.0` / `Release: 0.alpha.2` in the rpm (cargo-rpm splits on the first `-`, which is
what makes the pre-release sort below the final), as `0.1.0` + a build of `0.1.0-alpha.2` in the
macOS bundle (`CFBundleShortVersionString` must be dotted integers), and verbatim in the AppImage,
the `.dmg` and the Arch package. Never hardcode a filename.

`tools/package-linux.sh` needs Linux: `dpkg-deb`, `rpmbuild`, `dpkg-shlibdeps` and `appimagetool`
have no counterpart elsewhere, and it says so rather than failing three steps in.

## Linux build dependencies

Not declared anywhere in the build, so a missing one is a link error in the middle of a release
build:

```
pkg-config  libfontconfig1-dev  libasound2-dev  libudev-dev
```

fontconfig via `fontique`, ALSA via `cpal`, libudev via `i-slint-backend-linuxkms`. GL, xkbcommon,
X11 and Wayland are `dlopen`ed by glutin, winit and Skia rather than linked: runtime dependencies of
the package, not build dependencies of the crate, and invisible to every dependency scanner — see
[`linux/`](./linux/README.md) for how the deb and rpm name them by hand. At runtime the app also
shells out to `xdg-open` and `dbus-send`; file and folder pickers go through the XDG desktop portal
(`rfd`), whose file chooser `xdg-desktop-portal` implements.
