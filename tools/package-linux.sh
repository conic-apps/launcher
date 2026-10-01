#!/usr/bin/env bash
# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

# Build the Linux packages: deb, rpm and AppImage.
#
# One script for the developer and for CI, because the three of them have to
# agree about the payload and there is only one copy of it:
# `packaging/linux/` holds the desktop entry and the icon set, the `.deb` and
# `.rpm` read it through `app/Cargo.toml`, and the AppDir is laid out from it
# here. See `packaging/linux/README.md`.
#
# Usage:
#   tools/package-linux.sh              # deb, rpm and AppImage
#   tools/package-linux.sh deb rpm      # just those two
#
# Everything lands in `<target-dir>/package/`.
#
# Deliberately not a Tauri/NPM script: there is no Tauri and no `package.json`
# any more. `cargo build --release` is the whole build.

set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly CRATE=conic-launcher
readonly PAYLOAD="$ROOT/packaging/linux"
readonly STAGE="$ROOT/app/.rpm"     # what cargo-rpm reads extra files from

cd "$ROOT"

# The whole hicolor ladder `tools/generate-icons.py` writes. Kept in step with
# `[package.metadata.deb]` and `[package.metadata.rpm]` by hand, which is why it
# is spelled out rather than globbed: a size added to the generator has to be
# added to two tables in Cargo.toml anyway, and a glob that silently matches
# nothing is not a check.
readonly ICON_SIZES=(16 22 24 32 48 64 128 256 512)

die() {
    printf '\033[31merror:\033[0m %s\n' "$*" >&2
    exit 1
}

step() { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }

require() {
    command -v "$1" > /dev/null 2>&1 ||
        die "$1 is not installed.$2"
}

# ---------------------------------------------------------------------------
# What to build
# ---------------------------------------------------------------------------

declare -a TARGETS=()
if [[ $# -eq 0 ]]; then
    TARGETS=(deb rpm appimage)
else
    TARGETS=("$@")
fi

for target in "${TARGETS[@]}"; do
    case "$target" in
    deb | rpm | appimage) ;;
    *) die "unknown target '$target' (expected: deb, rpm, appimage)" ;;
    esac
done

wants() {
    local wanted="$1" target
    for target in "${TARGETS[@]}"; do
        [[ "$target" == "$wanted" ]] && return 0
    done
    return 1
}

# ---------------------------------------------------------------------------
# Preconditions
# ---------------------------------------------------------------------------

# All three formats are assembled by a Linux program — `dpkg-deb`, `rpmbuild`,
# `appimagetool` — and there is no macOS or Windows equivalent to call instead.
# Failing here is worth a lot more than a `command not found` three steps in.
[[ "$(uname -s)" == "Linux" ]] ||
    die "Linux packages can only be built on Linux (this is $(uname -s)).
    The build itself is platform-independent, so a container works:
      docker run --rm -v \"\$PWD:/src\" -w /src ubuntu:24.04 \\
        bash -c 'apt-get update && apt-get install -y --no-install-recommends ... && tools/package-linux.sh'"

if wants deb; then
    require dpkg-deb "  (Debian/Ubuntu: 'dpkg-dev', which also provides dpkg-deb)"
    # `dpkg-shlibdeps` is what resolves `depends = "$auto"`, and it is not in the
    # `dpkg` package that provides `dpkg-deb`. Without it the shared-library
    # dependencies come out empty *silently* rather than failing, so the package
    # builds and is then missing every library it needs.
    require dpkg-shlibdeps "  (Debian/Ubuntu: 'dpkg-dev')"
fi

if wants rpm; then
    require rpmbuild "  (Debian/Ubuntu: 'rpm'; Fedora ships it as 'rpm-build')"
fi

if wants appimage; then
    require appimagetool "  (https://github.com/AppImage/AppImageKit/releases/download/continuous/)"
    require file "  (appimagetool shells out to 'file')"
fi

# `cargo <subcommand>` takes the subcommand's short name (`deb`, `rpm`), not its
# binary name: cargo looks for `cargo-<subcommand>`, so `cargo cargo-deb` goes
# looking for `cargo-cargo-deb`. Checking the binary directly avoids having to
# remember which of the two names applies.
if wants deb; then
    require cargo-deb "  (cargo install cargo-deb)"
fi
if wants rpm; then
    require cargo-rpm "  (cargo install cargo-rpm)"
fi

[[ -f "$PAYLOAD/conic-launcher.desktop" ]] ||
    die "$PAYLOAD/conic-launcher.desktop is missing"
[[ -f "$PAYLOAD/icons/256x256.png" ]] ||
    die "the icon set is missing; regenerate it with tools/generate-icons.py"

# ---------------------------------------------------------------------------
# Build the executable
# ---------------------------------------------------------------------------

# Everything below needs two facts about the workspace — where Cargo put the
# build, and what the version is — and both have to come from Cargo rather than
# be assumed. `target/release/` is only cargo's default: a `.cargo/config.toml`
# `build.target-dir` or an exported `CARGO_TARGET_DIR` (a CI cache on another
# volume, a container) moves it, and a script that guessed wrong would go looking
# for an executable that is not there. One `cargo metadata` call answers both.
#
# `sed` over JSON rather than `jq`: jq is not a build dependency of anything here
# (the payload is committed precisely so the packages build on machines without
# extra tooling), and the two values are the crate's own version string and a
# filesystem path — no quoting to get wrong.
metadata="$(cargo metadata --no-deps --format-version 1)"
readonly TARGET_DIR="$(printf '%s' "$metadata" |
    sed -n 's|.*"target_directory":"\([^"]*\)".*|\1|p' | head -1)"
readonly VERSION="$(printf '%s' "$metadata" |
    sed -n "s|.*\"name\":\"$CRATE\",\"version\":\"\([^\"]*\)\".*|\1|p" | head -1)"

[[ -n "$TARGET_DIR" ]] || die "could not read target_directory from cargo metadata"
[[ -n "$VERSION" ]] || die "could not read the version of $CRATE from cargo metadata"

# Under the target dir rather than a hardcoded `target/`, for the same reason.
readonly OUT="$TARGET_DIR/package"
readonly APPDIR="$OUT/$CRATE.AppDir"

# `--locked` everywhere: the release pipeline has to be reproducible from a
# commit, and a packaging run that silently updated `Cargo.lock` would not be.
step "Building $CRATE $VERSION (release)"
cargo build --release --locked -p "$CRATE"

readonly BIN="$TARGET_DIR/release/$CRATE"
[[ -f "$BIN" ]] || die "$BIN was not produced"

# `CURSEFORGE_API_KEY` is read by `crates/curseforge/build.rs` at compile time.
# Unset is not an error — the official CurseForge API is simply unauthenticated
# — but it is worth saying out loud, because the symptom otherwise shows up much
# later as an empty search.
if [[ -z "${CURSEFORGE_API_KEY:-}" ]]; then
    note "CURSEFORGE_API_KEY is unset; the official CurseForge API will be unauthenticated."
fi

# ---------------------------------------------------------------------------
# The payload
# ---------------------------------------------------------------------------

# Lays the desktop entry, the icon ladder and the licence out under a prefix, the
# same way for every consumer. `install -D` rather than `mkdir && cp` so an empty
# intermediate directory cannot be left behind on a partial run.
stage_payload() {
    local prefix="$1"
    local size

    install -Dm644 "$PAYLOAD/conic-launcher.desktop" \
        "$prefix/usr/share/applications/conic-launcher.desktop"
    install -Dm644 "$ROOT/LICENSE" "$prefix/usr/share/licenses/$CRATE/LICENSE"

    for size in "${ICON_SIZES[@]}"; do
        [[ -f "$PAYLOAD/icons/${size}x${size}.png" ]] ||
            die "$PAYLOAD/icons/${size}x${size}.png is missing; run tools/generate-icons.py"
        install -Dm644 "$PAYLOAD/icons/${size}x${size}.png" \
            "$prefix/usr/share/icons/hicolor/${size}x${size}/apps/$CRATE.png"
    done
    install -Dm644 "$PAYLOAD/icons/scalable.svg" \
        "$prefix/usr/share/icons/hicolor/scalable/apps/$CRATE.svg"
}

mkdir -p "$OUT"

# ---------------------------------------------------------------------------
# deb
# ---------------------------------------------------------------------------

if wants deb; then
    step "Building .deb"
    # `--no-build`: the release build above is the one that gets packaged, so
    # this cannot rebuild it differently (or fail halfway through, after the
    # slow part).
    cargo deb -p "$CRATE" --no-build
    # cargo-deb writes into `<target-dir>/debian/`. Note the filename: it maps the
    # Cargo version's `-` to `~`, because a Debian version may only carry one `-`
    # (the revision separator). `0.1.0-alpha.2` therefore packages as
    # `0.1.0~alpha.2-1`, which is *not* the string in `app/Cargo.toml`.
    find "$TARGET_DIR/debian" -maxdepth 1 -name "${CRATE}_*.deb" -exec cp {} "$OUT/" \;
    ls -1 "$OUT"/${CRATE}_*.deb
fi

# ---------------------------------------------------------------------------
# rpm
# ---------------------------------------------------------------------------

if wants rpm; then
    step "Building .rpm"
    # `cargo rpm` reads its extra files from `app/.rpm/` and its spec from the
    # same place, and `--no-cargo-build` keeps it from rebuilding what was just
    # built. The spec is committed; only `usr/` below it is staged.
    rm -rf "$STAGE/usr"
    stage_payload "$STAGE"

    # Run from `app/`, and there is no `-p`: cargo-rpm has no crate selector, it
    # reads the `Cargo.toml` in the *current directory* and needs a `[package]`
    # section in it. From the workspace root that file is a virtual manifest with
    # no `[package]`, and cargo-rpm exits with "no [package] section in
    # Cargo.toml!". This is also what makes its `.rpm/` lookup land on `app/.rpm/`
    # rather than `.rpm/` at the root.
    (cd "$ROOT/app" && cargo rpm build --no-cargo-build)

    # rpmbuild writes under `<target-dir>/<profile>/rpmbuild/RPMS/<arch>/` — the
    # profile component is cargo-rpm's, not something to predict, so the path is
    # globbed. Scoped to `RPMS/` so the `SRPMS/` source rpm next to it is not
    # picked up.
    find "$TARGET_DIR" -path '*/rpmbuild/RPMS/*' -name "${CRATE}-*.rpm" \
        -exec cp {} "$OUT/" \;
    ls -1 "$OUT"/${CRATE}-*.rpm
fi

# ---------------------------------------------------------------------------
# AppImage
# ---------------------------------------------------------------------------

if wants appimage; then
    step "Building .AppImage"

    # Built here rather than by `cargo appimage`, because an AppDir is a
    # documented directory layout and `appimagetool` takes one as its only
    # argument. The layout below is what the AppImage spec asks for, and doing it
    # by hand is what puts the *icon theme tree* inside the image: `cargo
    # appimage` copies a single flat `<name>.png` to the AppDir root and never
    # writes a `.DirIcon`, which appimagetool then complains about. Its
    # `auto_link` option is worse: it follows `ldd` only one level deep, so it
    # would bundle `libfontconfig` without the `libfreetype` and `libexpat` it
    # needs to work at all.
    #
    # The shared libraries are deliberately NOT bundled. The AppImage spec itself
    # says an image must not carry `libc`, `libgcc_s` and friends, and what is
    # left here (fontconfig, xkbcommon, GL, ALSA) is on every desktop that can run
    # a Slint app at all.
    rm -rf "$APPDIR"
    # `stage_payload` lays out `<prefix>/usr/share/...`, so the prefix is the
    # AppDir itself — not `$APPDIR/usr`, which would give the image a
    # `usr/usr/share`. It is the same prefix `app/.rpm/` gets, which is what
    # keeps the two payload layouts from drifting.
    stage_payload "$APPDIR"

    install -Dm755 "$BIN" "$APPDIR/usr/bin/$CRATE"
    install -Dm644 "$PAYLOAD/conic-launcher.desktop" "$APPDIR/$CRATE.desktop"

    # Two icon files at the AppDir root, and the difference matters:
    #   conic-launcher.png — the one `Icon=conic-launcher` in the desktop entry
    #                        resolves against. appimagetool fails the build
    #                        without it ("... defined in desktop file but not
    #                        found"), because an AppDir whose launcher has no
    #                        icon is not a usable AppDir.
    #   .DirIcon           — what a file manager shows for the *image itself*,
    #                        before anything is installed. Usually a symlink to
    #                        the same file; a copy here because a symlink would
    #                        have to survive being packed into a squashfs.
    install -Dm644 "$PAYLOAD/icons/256x256.png" "$APPDIR/$CRATE.png"
    install -Dm644 "$PAYLOAD/icons/256x256.png" "$APPDIR/.DirIcon"

    # The entry point. Kept a shell script rather than a copied binary: it is four
    # lines, and `$APPDIR` is a mount point whose path is only known at runtime.
    cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/sh
# Conic Launcher AppRun. Resolves the mount point, then hands over to the binary.
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/conic-launcher" "$@"
APPRUN
    chmod 755 "$APPDIR/AppRun"

    # `ARCH` and `VERSION` are what appimagetool names the image and records in its
    # metadata. `uname -m` rather than a field out of `rustc -vV`: appimagetool
    # expects its own architecture vocabulary (`x86_64`, `aarch64`), which is
    # `uname -m`'s, not a Rust target triple's — and an `ARCH` it does not
    # recognise makes it quietly ignore the output path passed on the command
    # line and pick a name of its own.
    #
    # `$VERSION` is the Cargo version. The rpm splits it into Version/Release and
    # the deb maps its `-` to `~`; the AppImage takes it as written.
    export ARCH="$(uname -m)"
    export VERSION

    # `APPIMAGE_EXTRACT_AND_RUN=1` because `appimagetool` is *itself* an AppImage,
    # and a container or CI runner has no FUSE to mount it with — without this it
    # exits with "dlopen(): error loading libfuse.so.2". It applies to the tool
    # only: the artifact is a normal squashfs image and mounts as one on a
    # desktop. Set it here rather than in the workflow so a local container run
    # and CI behave identically.
    APPIMAGE_EXTRACT_AND_RUN=1 appimagetool "$APPDIR" "$OUT/${CRATE}-${VERSION}-${ARCH}.AppImage"
    ls -lh "$OUT"/*.AppImage
fi

step "Done"
note "packages in ${OUT#"$ROOT"/}/"