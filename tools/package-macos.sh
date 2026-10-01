#!/usr/bin/env bash
# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

# Build the macOS artifacts: the `.app` bundle, a universal binary, and a `.dmg`.
#
# Usage:
#   tools/package-macos.sh                    # .app + .dmg, native arch only
#   tools/package-macos.sh --universal        # both arches in one binary
#   tools/package-macos.sh --dmg-only         # the .dmg only; the .app is not kept
#   tools/package-macos.sh --no-dmg           # the .app only, no disk image
#
# Everything lands in `<target-dir>/package/`.
#
# What Tauri used to do and this does instead: Tauri's bundler assembled the
# `.app`, produced a universal binary with `lipo`, ad-hoc signed it and ran
# `create-dmg`. None of that is reachable now, so it is done here — with the same
# system tools, so the result is the same shape of artifact.

set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly CRATE=conic-launcher
readonly APP="$CRATE.app"
readonly PAYLOAD="$ROOT/packaging/macos"
readonly TEMPLATE="$PAYLOAD/Info.plist"
readonly ICONS="$PAYLOAD/$CRATE.icns"

cd "$ROOT"

die() {
    printf '\033[31merror:\033[0m %s\n' "$*" >&2
    exit 1
}

step() { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }

# ---------------------------------------------------------------------------
# Options
# ---------------------------------------------------------------------------

declare -a TARGETS=()          # Rust target triples to build
declare UNIVERSAL=0
# Both are "leave this artifact out" flags, and they are *not* the same: the
# `.app` is an input to the `.dmg`, so `--dmg-only` has to build it, seal the image
# from it, and only then drop it from the staging directory. `--no-dmg` skips the
# image entirely and keeps the bundle.
declare NO_DMG=0
declare DMG_ONLY=0

while [[ $# -gt 0 ]]; do
    case "$1" in
    --universal)
        UNIVERSAL=1
        shift
        ;;
    --dmg-only)
        DMG_ONLY=1
        shift
        ;;
    --no-dmg)
        NO_DMG=1
        shift
        ;;
    -h | --help)
        # The header comment is the usage. Stopping at the first non-comment line
        # keeps it from drifting out of sync with the flags above.
        sed -n '/^#/!q; s/^# \{0,1\}//p' "$0" | sed -n '2,20p'
        exit 0
        ;;
    *)
        die "unknown option '$1' (expected: --universal, --dmg-only, --no-dmg)"
        ;;
    esac
done

# The host triple. `--universal` builds this one too, so a developer on Apple
# Silicon who passes it gets a binary they can still run locally.
HOST="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "$HOST" || "$HOST" != *apple-darwin* ]]; then
    die "this only runs on macOS (rustc's host is '$HOST', not an apple-darwin target)"
fi

if [[ "$UNIVERSAL" -eq 1 ]]; then
    TARGETS=("aarch64-apple-darwin" "x86_64-apple-darwin")
    # `aarch64` first so `lipo -create` lists it as the native slice, which is
    # what keeps `lipo -info`'s first line and Rosetta's own reports readable.
else
    TARGETS=("$HOST")
fi

# ---------------------------------------------------------------------------
# Preconditions
# ---------------------------------------------------------------------------

# Every one of these is either in `/usr/bin` on macOS or installed by the Xcode
# command line tools. `iconutil` and `sips` are only needed to *regenerate* the
# `.icns`, not to package it — the `.icns` is committed — but `hdiutil` and
# `codesign` are on the critical path.
for tool in codesign hdiutil plutil lipo; do
    command -v "$tool" > /dev/null 2>&1 ||
        die "$tool is missing (it ships with macOS; install the Xcode command line tools)"
done

[[ -f "$TEMPLATE" ]] || die "$TEMPLATE is missing"
[[ -f "$ICONS" ]] ||
    die "$ICONS is missing; regenerate it with tools/generate-icns.py"

# ---------------------------------------------------------------------------
# Build the executable(s)
# ---------------------------------------------------------------------------

# From Cargo, for the same reason as the Linux script: `target/` is only the
# default, and a `.cargo/config.toml` or an exported `CARGO_TARGET_DIR` moves it.
# `sed` over the JSON rather than `jq`, which is not a build dependency here.
metadata="$(cargo metadata --no-deps --format-version 1)"
readonly TARGET_DIR="$(printf '%s' "$metadata" |
    sed -n 's|.*"target_directory":"\([^"]*\)".*|\1|p' | head -1)"
readonly VERSION="$(printf '%s' "$metadata" |
    sed -n "s|.*\"name\":\"$CRATE\",\"version\":\"\([^\"]*\)\".*|\1|p" | head -1)"

[[ -n "$TARGET_DIR" ]] || die "could not read target_directory from cargo metadata"
[[ -n "$VERSION" ]] || die "could not read the version of $CRATE from cargo metadata"

# The marketing version has to be three dot-separated integers: `CFBundleShortVersionString`
# is the key macOS and the App Store validate, and a pre-release suffix there is
# rejected. The suffix is carried by `CFBundleVersion` instead, which is free-form.
# `0.1.0-alpha.2` therefore installs as `0.1.0` with a build of `0.1.0-alpha.2`,
# which is the same split the rpm makes for its own reasons.
readonly SHORT_VERSION="$(printf '%s' "$VERSION" | sed 's/-.*//')"
if [[ ! "$SHORT_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]]; then
    die "cannot derive a CFBundleShortVersionString from '$VERSION'"
fi

if [[ "$UNIVERSAL" -eq 1 ]]; then
    step "Building $CRATE $VERSION (universal)"
else
    step "Building $CRATE $VERSION (native: $HOST)"
fi
for target in "${TARGETS[@]}"; do
    cargo build --release --locked -p "$CRATE" --target "$target"
done

# `CURSEFORGE_API_KEY` is read by `crates/curseforge/build.rs` at compile time.
# Unset is not an error — the official CurseForge API is simply unauthenticated
# — but the symptom otherwise shows up much later as an empty search.
if [[ -z "${CURSEFORGE_API_KEY:-}" ]]; then
    note "CURSEFORGE_API_KEY is unset; the official CurseForge API will be unauthenticated."
fi

# ---------------------------------------------------------------------------
# Universal binary
# ---------------------------------------------------------------------------

# Assigned now, used by the `--version`-style substitution further down; declared
# readonly together with the other derived facts so that everything computed from
# the workspace is in one place.
readonly STAGE="$TARGET_DIR/package"
readonly CONTENTS="$STAGE/$APP/Contents"
readonly MACOS_DIR="$CONTENTS/MacOS"
readonly RESOURCES="$CONTENTS/Resources"

# The build products are assembled *inside* the staging directory rather than
# left beside it, so a `--universal` run's intermediate `.universal` file cannot
# be mistaken for an artifact — the Linux script's staging directory is
# `target/package` too, and mixing the two conventions is how a stray file ends
# up in an upload glob.
mkdir -p "$STAGE"
readonly WORK="$STAGE/.work"
rm -rf "$WORK"
mkdir -p "$WORK"

if [[ "$UNIVERSAL" -eq 1 ]]; then
    step "Creating the universal binary"
    lipo -create \
        "$TARGET_DIR"/${TARGETS[0]}/release/"$CRATE" \
        "$TARGET_DIR"/${TARGETS[1]}/release/"$CRATE" \
        -output "$WORK/$CRATE.universal"
    lipo -info "$WORK/$CRATE.universal" | sed 's/^/    /'
    BINARY="$WORK/$CRATE.universal"
else
    BINARY="$TARGET_DIR/$HOST/release/$CRATE"
    # `lipo -info` on a thin binary still succeeds (it prints the single arch),
    # so the fallback is only about the wording, not about detection.
    lipo -info "$BINARY" | sed 's/^/    /'
fi

[[ -f "$BINARY" ]] || die "$BINARY was not produced"

# ---------------------------------------------------------------------------
# The .app bundle
# ---------------------------------------------------------------------------

step "Assembling $APP"

# Rebuilt from scratch rather than copied over: a stale file from a previous
# version's Resources is exactly the kind of thing that ships to users.
rm -rf "$STAGE/$APP"
mkdir -p "$MACOS_DIR" "$RESOURCES"

install -m755 "$BINARY" "$MACOS_DIR/$CRATE"
install -m644 "$ICONS" "$RESOURCES/$CRATE.icns"
install -m644 "$ROOT/LICENSE" "$RESOURCES/LICENSE"

# Fill in the two version keys. The template carries literal `@VERSION@`
# placeholders, and they are substituted with `PlistBuddy`'s own string handling
# rather than by rewriting the XML — so the committed template stays the single
# copy of the metadata and a value containing a `<` or an `&` cannot produce a
# malformed plist.
#
# The two keys take *different* values on purpose: `CFBundleShortVersionString` is
# the dotted-numeric marketing version macOS validates, and
# `CFBundleVersion` is the free-form build number that carries the pre-release
# suffix.
#
# The template's placeholder values are only there to make the plist valid XML
# before substitution; both keys are overwritten below.
PLIST="$CONTENTS/Info.plist"
cp "$TEMPLATE" "$PLIST"
for key in CFBundleShortVersionString CFBundleVersion; do
    value="$SHORT_VERSION"
    [[ "$key" == "CFBundleVersion" ]] && value="$VERSION"

    # Replace the placeholder in place. `Add` rather than `Set` because the key
    # already exists in the template and `Set` on a present key would rewrite it
    # — either works, but `Set` fails loudly if the template ever loses the key,
    # which is the failure worth catching.
    if /usr/libexec/PlistBuddy -c "Set :$key $value" "$PLIST" 2> /dev/null; then
        :
    else
        die "Info.plist has no $key to set"
    fi
done
plutil -lint "$PLIST" > /dev/null || die "the rendered Info.plist is invalid"

# The bundle must not claim a *lower* macOS than the binary needs, or Launch
# Services will happily start the app on a system that cannot run it and the
# failure surfaces as a link error about a missing symbol rather than as "this
# Mac is too old". So the check is one-directional: `MINOS` is what the binary
# demands and it must not exceed what the plist promises.
#
# Both load commands are read, because which one a binary carries depends on its
# architecture and on nothing else:
#
#   aarch64-apple-darwin  LC_BUILD_VERSION       minos 11.0
#   x86_64-apple-darwin   LC_VERSION_MIN_MACOSX  version 10.12
#
# Those are each target's *own* `rust-std` default rather than a choice made
# here -- this script passes `--target` and never sets
# `MACOSX_DEPLOYMENT_TARGET` -- so the two builds genuinely disagree, and a
# `--universal` binary carries both commands at once.
#
# `awk`, not `sed`, and that is a fix rather than a preference:
# `sed -n '/re/,/re/{s/../../\1/p;q}'` is a GNU extension that BSD sed -- what
# macOS ships and what the `macos-15-intel` runner runs -- rejects outright with
# `extra characters at the end of q command`. It only ever worked on a
# developer machine because the arm64 binary happens to be the one carrying
# `LC_BUILD_VERSION`, and the Intel runner was the first to reach the other
# branch.
#
# `LC_BUILD_VERSION` wins when both are present: it is the current spelling, and
# in a `--universal` binary the other command belongs to the other slice.
MINOS="$(otool -l "$MACOS_DIR/$CRATE" | awk '
    /^ *cmd LC_BUILD_VERSION$/      { in_build = 1; next }
    /^ *cmd /                       { in_build = 0 }
    in_build && /^ *minos /         && !have_build { sub(/^ *minos /, ""); build = $0; have_build = 1 }

    /^ *cmd LC_VERSION_MIN_MACOSX$/ { in_old = 1; next }
    /^ *cmd /                       { in_old = 0 }
    in_old && /^ *version /         && !have_old   { sub(/^ *version /, ""); old = $0; have_old = 1 }

    END { print (have_build ? build : (have_old ? old : "")) }
')"

PLIST_MINOS="$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$PLIST")"
[[ -n "$MINOS" ]] ||
    die "could not read the binary's minimum macOS version out of its load commands"
# Compared with `sort -V` rather than `!=`, because the two need not be equal:
# the x86_64 slice carries a lower default (10.12) than the plist promises
# (11.0), and that is fine -- the plist is the stricter of the two and macOS
# only ever consults the plist. What must not happen is the reverse.
if [[ "$(printf '%s\n%s\n' "$PLIST_MINOS" "$MINOS" | sort -V | head -1)" != "$MINOS" ]]; then
    die "LSMinimumSystemVersion is $PLIST_MINOS but the binary needs $MINOS"
fi
note "  LSMinimumSystemVersion $PLIST_MINOS covers the binary's $MINOS"

# ---------------------------------------------------------------------------
# Signing
# ---------------------------------------------------------------------------

# Ad-hoc (`codesign -s -`). Not a Developer ID signature: that needs a certificate
# the build machine has to be configured with, and an ad-hoc signature is what a
# local build and an unsigned-CI build can both produce.
#
# What it does and does not buy:
#  * **With** it, arm64 runs at all. Apple Silicon refuses to execute an
#    unsigned binary, and `--force` here overwrites the linker's own ad-hoc
#    signature (which is what `codesign -dv` reports as `linker-signed`) so that
#    the seal covers the *bundle* rather than just the Mach-O.
#  * **Without** a Developer ID + notarization, Gatekeeper still blocks the app on
#    another user's machine ("damaged", or the cannot-verify dialog), and
#    `spctl --assess` rejects it. That is inherent to an unsigned build, not
#    something the packaging script can fix.
step "Signing (ad-hoc)"
codesign --force --sign - --timestamp=none "$STAGE/$APP"
codesign --verify --deep --strict --verbose=2 "$STAGE/$APP" 2>&1 | sed 's/^/    /'
codesign -dv "$STAGE/$APP" 2>&1 | grep -E "Identifier|Format|Signature|Sealed Resources" | sed 's/^/    /'

# ---------------------------------------------------------------------------
# .dmg
# ---------------------------------------------------------------------------

# The image's name says what is *in* it, which is not always the host: a
# `--universal` build is `universal2` no matter which machine assembled it, and
# an `x86_64` one is `x86_64` when built on Intel.
if [[ "$UNIVERSAL" -eq 1 ]]; then
    readonly DMG_ARCH=universal2
elif [[ "$HOST" == x86_64-apple-darwin ]]; then
    readonly DMG_ARCH=x86_64
else
    readonly DMG_ARCH=arm64
fi

if [[ "$NO_DMG" -eq 1 ]]; then
    step "Done"
    note "app bundle at ${STAGE#"$ROOT"/}/$APP"
    exit 0
fi

step "Building the .dmg"
readonly DMG="$STAGE/$CRATE-$VERSION-$DMG_ARCH.dmg"
readonly MOUNT_DIR="$(mktemp -d)"

# `hdiutil create -srcfolder` on a staging directory, rather than Tauri's
# `create-dmg`: `create-dmg` is an npm package the project no longer has a
# toolchain for, and the layout it produces (app + Applications symlink) is a
# handful of lines of shell.
# `WORK` rather than a second scratch directory: it is already inside `$STAGE`,
# it is already emptied above, and reusing it keeps the number of places this
# script can leave something behind down to one.
STAGE_DMG="$WORK/dmgroot"
mkdir -p "$STAGE_DMG"
cp -R "$STAGE/$APP" "$STAGE_DMG/"
# The drag-to-Applications affordance. A symlink, not a copy: Finder's "install"
# gesture is a *move* into it, and a real `/Applications` is where that has to
# land.
ln -s /Applications "$STAGE_DMG/Applications"

rm -f "$DMG"
# `-srcfolder` builds the image in one step from the staged directory, with no
# intermediate volume to mount and unmount. `-volname` is what the volume is
# called once mounted — what the user sees in Finder's title bar, *not* the disk
# image's file name.
#
# `hdiutil create` prints a deprecation warning on current macOS in favour of
# `diskutil image create from`; the hdiutil spelling is kept because it is what
# works on the older runners this also has to build on, and because the warning is
# about a future rename rather than a future removal.
#
# `UDZO` is the compressed read-only format, i.e. a normal `.dmg`.
#
# No Finder window layout: that needs a `.DS_Store`, which is a Finder-private
# format written by Finder itself, not by a shell script. `create-dmg` had the
# same constraint and shipped a binary to work around it. A window that opens with
# both icons in the top-left corner is cosmetic and fine.
hdiutil create \
    -volname "Conic Launcher" \
    -srcfolder "$STAGE_DMG" \
    -ov -format UDZO \
    "$DMG" 2> >(grep -v "is deprecated" >&2 || true)

# Mount it and confirm what is inside, because a `.dmg` that builds without error
# but mounts empty is a real failure mode of `-srcfolder`.
#
# `-f` on the executable, not `-d`: it is a *file*, and `-d` is false for it. That
# is the whole bug this check exists to catch, so writing the wrong test would be
# worse than not checking at all.
hdiutil attach "$DMG" -mountpoint "$MOUNT_DIR" -nobrowse -quiet
if [[ -f "$MOUNT_DIR/$APP/Contents/MacOS/$CRATE" ]]; then
    note "  mounted and contains the executable"
else
    hdiutil detach "$MOUNT_DIR" -quiet || true
    die "the .dmg mounted but $APP/Contents/MacOS/$CRATE is not in it"
fi
hdiutil detach "$MOUNT_DIR" -quiet
# The image is built from `$STAGE_DMG`, which sits inside `$WORK`, and the
# executable came from `$WORK` too — so both are removed *after* the image is
# created, not before.
rm -rf "$MOUNT_DIR"

ls -lh "$DMG" | sed 's/^/    /'

# `--dmg-only` means the `.dmg` and nothing else in the staging directory. The
# bundle is removed *after* the image is sealed, because the image was built from
# it — removing it first would produce an empty disk image.
if [[ "$DMG_ONLY" -eq 1 ]]; then
    rm -rf "$STAGE/$APP"
    note "  --dmg-only: removed the staged $APP"
fi

step "Done"
note "packages in ${STAGE#"$ROOT"/}/"
# `$WORK` holds the intermediate universal binary and the `.dmg` staging tree.
# Removing it here rather than at each exit path means a failure above leaves it
# behind on purpose, to be looked at.
rm -rf "$WORK"