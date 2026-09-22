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
then builds and installs the package. Compilation requires Rust 1.88 or newer,
Node.js 24 LTS (`nodejs-lts-krypton`) and pnpm 11. The LTS Node package conflicts
with Arch's rolling `nodejs` package; a clean build chroot can be used if you need
to keep another Node version on your system.

Frontend and Rust dependencies are fetched during `prepare()` using the release's
lockfiles. The obsolete pnpm entry in the downloaded `.npmrc` is aligned with
`package.json`. The Tauri build produces a native executable without generating
AppImage, deb or rpm bundles or requiring an updater signing key.

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

Update this installation through rebuilt Arch packages. The launcher's built-in
Linux updater targets AppImage installations.

## Updating the package recipe

Update `_version` and `pkgver` for a new release, reset `pkgrel` to `1`, and refresh
the source checksums. For packaging-only changes, increment `pkgrel` instead.
After changing the recipe or desktop file, regenerate metadata from this directory:

```bash
updpkgsums # provided by pacman-contrib
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
```

The full upstream license, including its additional terms, is installed in
`/usr/share/licenses/conic-launcher/LICENSE`.
