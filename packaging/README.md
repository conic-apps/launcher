# Packaging

One directory per platform, holding the payload that platform's formats install *and* the script
that assembles them. The `README.md` in each documents its contents and what is easy to break there:

- [`linux/`](./linux/README.md) — the desktop entry and hicolor icon set, the only copy the deb,
  rpm, AppImage and Arch package install.
- [`macos/`](./macos/README.md) — the `.icns`, the `Info.plist` template, the disk image's window
  background, and the two scripts that lay the `.dmg` out.
- [`windows/`](./windows/README.md) — the `.wxs` and the `.ico`.
- [`arch/`](./arch/README.md) — the `PKGBUILD` and its `.SRCINFO`.

Each script writes its artifacts to `<target-dir>/package/` and runs from the repository root:

```bash
packaging/linux/package.sh              # deb + rpm + AppImage
packaging/linux/package.sh deb rpm      # a subset

packaging/macos/package.sh              # .app + .dmg, native arch
packaging/macos/package.sh --universal  # both arches in one binary, local only; CI
                                        # builds each natively instead (see macos/)

pwsh packaging/windows/package.ps1      # .exe + .msi, native arch
pwsh packaging/windows/package.ps1 msi  # a subset
```

| Format        | Recipe                                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------------------ |
| `.msi`/`.exe` | `packaging/windows/package.ps1`, see [windows/](./windows/README.md)                                               |
| `.deb`        | `[package.metadata.deb]` in `app/Cargo.toml`                                                                       |
| `.rpm`        | `[package.metadata.rpm]` + the spec at `app/.rpm/conic-launcher.spec`                                              |
| `.AppImage`   | the AppDir `packaging/linux/package.sh` assembles, then `appimagetool`                                             |
| `.app`/`.dmg` | the bundle `packaging/macos/package.sh` assembles, then `hdiutil`, with the window layout from `macos/ds-store.py` |
| Arch          | [`arch/PKGBUILD`](./arch/PKGBUILD) — `makepkg -si`                                                                 |

One rule spans every format: **each spells the version differently.** `0.1.0-alpha.2` ships as
`0.1.0~alpha.2-1` in the deb (cargo-deb maps `-` to `~`; a Debian version may carry only one `-`),
as `Version: 0.1.0` / `Release: 0.alpha.2` in the rpm (cargo-rpm splits on the first `-`, which is
what makes the pre-release sort below the final), as `0.1.0` + a build of `0.1.0-alpha.2` in the
macOS bundle (`CFBundleShortVersionString` must be dotted integers), and verbatim in the AppImage,
the `.dmg` and the Arch package. Never hardcode a filename.

`packaging/linux/package.sh` needs Linux: `dpkg-deb`, `rpmbuild`, `dpkg-shlibdeps` and
`appimagetool` have no counterpart elsewhere, and it says so rather than failing three steps in.

## Linux build dependencies

Not declared anywhere in the build, so a missing one is a link error in the middle of a release
build:

```
pkg-config  libfontconfig1-dev  libasound2-dev  libudev-dev
```

fontconfig via `fontique`, ALSA via `cpal`, libudev via `i-slint-backend-linuxkms`. Vulkan,
xkbcommon, X11 and Wayland are opened by wgpu, winit and Skia at runtime rather than linked: runtime
dependencies of the package, not build dependencies of the crate, and invisible to every dependency
scanner — see [`linux/`](./linux/README.md) for how the deb and rpm name them by hand. At runtime the app also
shells out to `xdg-open` and `dbus-send`; file and folder pickers go through the XDG desktop portal
(`rfd`), whose file chooser `xdg-desktop-portal` implements.
