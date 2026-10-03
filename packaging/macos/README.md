# macOS packaging payload

The `.icns` and the `Info.plist` template the `.app` bundle is assembled from, by
`tools/package-macos.sh`. Both are **committed**: `iconutil` exists only on
macOS, and the bundle is built from files in this directory.

| Consumer                     | Reads                                 |
| ---------------------------- | ------------------------------------- |
| `tools/package-macos.sh`     | both                                  |
| the `.app` inside the `.dmg` | a copy of the same assembled bundle   |

Nothing here is per-architecture: one `.icns` and one `Info.plist` serve every
build, because the bundle describes the app rather than the slice inside it. CI
produces two thin images (one per native runner) rather than one universal one;
see "Builds and signing" below for why.

## `conic-launcher.icns`

Regenerate with `python3 tools/generate-icns.py`. It renders the ladder from
`app/ui/assets/logo.png` and hands it to `iconutil`.

**Not** from `app/ui/assets/images/app-icon.png`, which is also in the tree and
looks like the obvious choice: that one is 512px and is embedded in the binary as
the macOS Dock icon (see `app/src/main.rs`). macOS asks for a 1024px render for the
512x512@2x slot, so using it would mean upscaling the worst possible source at the
one size a Dock icon is judged at.

`conic-launcher.iconset/` next to it is the rendered input, kept so the ladder
that went in can be seen and diffed. It is not a place to hand-edit: the script
regenerates it into a fresh directory on each run, so regenerate instead.

## `Info.plist`

A template with `@VERSION@` in the two version keys; the script fills them in
with `PlistBuddy`. Two keys, two different values, and it matters:

- `CFBundleShortVersionString` — three dot-separated integers, no suffix. macOS
  validates this one, and `0.1.0-alpha.2` is rejected.
- `CFBundleVersion` — the free-form build number, which is where the pre-release
  suffix goes.

Every other key is a decision, and the comment above the `<plist>` explains each
one. The one to check before editing anything else:

- `CFBundleIdentifier` is `app.conicmc.launcher.slint`, matching
  `single_instance::APP_ID`. macOS derives the TCC permission prompts, the
  notification identity, and whether two installed copies count as "the same app"
  from it. Changing it orphans the existing single-instance socket and any
  permissions the user has already granted.

## Builds and signing

**CI builds each macOS architecture on a native runner** (`macos-15` and
`macos-15-intel`), not one runner cross-compiling the other. A `--target` build
compiles for the guest while running every build script on the host, so a
host-only mistake in a `-sys` crate passes and a real Intel build fails — and
Skia and `aws-lc-rs` are both built through `cc`. Two thin `.dmg`s beat one
universal one for that reason; `--universal` is still there for a local build.

**The bundle is signed ad-hoc, not notarized.** `codesign -s -` is all a build
machine can do without a Developer ID certificate, and it is what makes the
binary executable at all on arm64. Gatekeeper still blocks the result on anyone
else's machine, so a user has to clear the quarantine flag themselves.
Distributing it properly needs a certificate and notarization credentials this
repository does not have. The consequence for CI: with Gatekeeper assessments off
on a runner (as they are on a dev machine), `spctl --assess` returns success
whatever the signature, so it proves nothing — `codesign --verify --deep
--strict` is the check that is actually meaningful.
