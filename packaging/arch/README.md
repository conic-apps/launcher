# Arch Linux package

This package builds Conic Launcher **v0.1.0-alpha.2** from the release source on
`x86_64` or `aarch64` (Arch Linux ARM). The Arch package version is `0.1.0alpha.2`
because `pkgver` cannot contain a hyphen. Local changes to the checkout are not
included in the build.

## Build and install

Install the standard Arch build tools:

```bash
sudo pacman -Syu --needed base-devel
```

From the repository root, run as a regular user:

```bash
cd packaging/arch
makepkg -si
```

`makepkg` installs the declared dependencies, downloads and verifies the release,
then builds and installs the package. Compilation requires Rust 1.88 or newer.

There is no Node.js or pnpm step: the app is a Slint/Rust binary, and
`cargo build --release --locked` is the whole build. The `.deb`, `.rpm` and
`.AppImage` that `tools/package-linux.sh` produces are not used here — Arch takes
a native package.

Launch **Conic Launcher** from the application menu or run `conic-launcher`.
The package also installs icons and the `conic-launcher://` URL handler.

Java is optional at installation time: the launcher can download a suitable
runtime for Minecraft, or use an installed `java-runtime` provider.

For authenticated access to the official CurseForge API, supply your own key
when building:

```bash
CURSEFORGE_API_KEY='your-key' makepkg -si
```

The key is embedded in the compiled executable. Without it, the official
CurseForge API is unauthenticated.

Update this installation through rebuilt Arch packages. The launcher has no
built-in updater, so an update is a new release downloaded from
[the releases page](https://github.com/conic-apps/launcher/releases).

## The payload

The desktop entry and the icon set live in `packaging/linux/` at the repository
root, shared with the deb, the rpm and the AppImage — they are one product with
one set of files. `package()` above installs from there; the icon ladder is
regenerated with `python3 tools/generate-icons.py` and committed, so no Python or
Pillow is needed to build this package.

The full upstream license, including its additional terms, is installed in
`/usr/share/licenses/conic-launcher/LICENSE`.

## Updating the package recipe

Update `_version` and `pkgver` for a new release, reset `pkgrel` to `1`, and
refresh the source checksum. For packaging-only changes, increment `pkgrel`
instead. Regenerate the metadata from this directory after changing the recipe or
the dependency list:

```bash
updpkgsums # provided by pacman-contrib
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
```

`.SRCINFO` is what an AUR page reads, so it has to match `PKGBUILD`; a stale copy
shows users the old dependency list and the old `pkgver`.