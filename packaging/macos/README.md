# macOS packaging payload

The `.icns`, the `Info.plist` template and the disk image's window background that the `.app` bundle
and its `.dmg` are assembled from, by `tools/package-macos.sh`. All of them are **committed**:
`iconutil` exists only on macOS, a build machine has no SVG rasterizer and no copy of the launcher's
font, and the bundle is built from files in this directory.

| Consumer                     | Reads                                                 |
| ---------------------------- | ----------------------------------------------------- |
| `tools/package-macos.sh`     | the `.icns`, the plist template, `dmg/background.png` |
| the `.app` inside the `.dmg` | a copy of the same assembled bundle                   |

Nothing here is per-architecture: one `.icns`, one `Info.plist` and one background serve every
build, because they describe the app rather than the slice inside it. CI produces two thin images
(one per native runner) rather than one universal one; see "Builds and signing" below for why.

## `conic-launcher.icns`

Regenerate with `python3 tools/generate-icns.py`. It renders the ladder from
`app/ui/assets/logo.png` and hands it to `iconutil`.

**Not** from `app/ui/assets/images/app-icon.png`, which is also in the tree and looks like the
obvious choice: that one is 512px and is embedded in the binary as the macOS Dock icon (see
`app/src/main.rs`). macOS asks for a 1024px render for the 512x512@2x slot, so using it would mean
upscaling the worst possible source at the one size a Dock icon is judged at.

`conic-launcher.iconset/` next to it is the rendered input, kept so the ladder that went in can be
seen and diffed. It is not a place to hand-edit: the script regenerates it into a fresh directory on
each run, so regenerate instead.

## `Info.plist`

A template with `@VERSION@` in the two version keys; the script fills them in with `PlistBuddy`. Two
keys, two different values, and it matters:

- `CFBundleShortVersionString` — three dot-separated integers, no suffix. macOS validates this one,
  and `0.1.0-alpha.2` is rejected.
- `CFBundleVersion` — the free-form build number, which is where the pre-release suffix goes.

Every other key is a decision, and the comment above the `<plist>` explains each one. The one to
check before editing anything else:

- `CFBundleIdentifier` is `app.conicmc.launcher.slint`, matching `single_instance::APP_ID`. macOS
  derives the TCC permission prompts, the notification identity, and whether two installed copies
  count as "the same app" from it. Changing it orphans the existing single-instance socket and any
  permissions the user has already granted.

## `dmg/`

What the disk image's Finder window looks like when a user opens it: a Catppuccin backdrop with the
two icons either side of the middle, an arrow between them, and a dashed frame around
`Applications`.

- `background.svg` — the design, and the file to edit. It is the conicmc.app page backdrop (a
  Catppuccin grid over six hyperbolas sharing one vertex) with the disk image's own furniture on
  top.
- `background.png` — what the image actually carries, at 144 dpi so the grid rules and the
  gradient-filled heading stay sharp on a Retina display. Finder paints a folder background from a
  raster image and cannot paint from an `.svg` at all, which is why there are two files.

Regenerate the PNG with `python3 tools/render-dmg-background.py`, which needs `fontTools`, an SVG
rasterizer (`rsvg-convert` or `resvg`) and `Pillow`. The render is committed so that none of those
are a build dependency.

**The text becomes outlines at render time.** The `.svg` asks for `"Comfortaa Nunito"`, which
`tools/merge-digit-font.py` bakes out of Comfortaa and a Nunito digit subset and which is embedded
in the launcher _binary_ — it is not installed anywhere a rasterizer would look. A rasterizer handed
a family it cannot resolve substitutes what it has and reports nothing, so the script lays the
glyphs out itself, instancing the `wght` axis per `font-weight` and applying the GPOS `kern` pairs,
and hands over `<path>`.

**Two things about the window are Finder's and cannot be set.** The name Finder draws under each
icon has no colour key anywhere in the format — `backgroundColor*` in `icvp` only tints a solid fill
— so over this backdrop it comes out near-black, and the only lever is its size.
`tools/dmg-ds-store.py` sets that to 10pt, which is the smallest Finder accepts: at 9 or below it
discards the whole `icvp` silently, and the window opens with no background and nothing to say why.
The second is the window's own position, which `WindowBounds` states from the bottom of a screen
whose height nothing at build time knows.

**The window's size and the icon placement live in the `.svg`, not in the script.**
`tools/dmg-ds-store.py` reads `width`, `height` and the `#applications-icon-cell` rectangle out of
`background.svg` to write the `.DS_Store`, because Finder paints the background 1:1 from the content
view's top-left corner: a window the wrong size shows a misaligned background, and a picture the
wrong size shows scrollbars. Move the marker and both icons follow, which is the only way the
artwork and the placement stay in step. Run `python3 tools/dmg-ds-store.py --check` to see what it
derived and to catch a rendered PNG that no longer matches the `.svg`.

## Builds and signing

**CI builds each macOS architecture on a native runner** (`macos-26` and `macos-26-intel`), not one
runner cross-compiling the other. A `--target` build compiles for the guest while running every
build script on the host, so a host-only mistake in a `-sys` crate passes and a real Intel build
fails — and Skia and `aws-lc-rs` are both built through `cc`. Two thin `.dmg`s beat one universal
one for that reason; `--universal` is still there for a local build.

**The bundle is signed ad-hoc, not notarized.** `codesign -s -` is all a build machine can do
without a Developer ID certificate, and it is what makes the binary executable at all on arm64.
Gatekeeper still blocks the result on anyone else's machine, so a user has to clear the quarantine
flag themselves. Distributing it properly needs a certificate and notarization credentials this
repository does not have. The consequence for CI: with Gatekeeper assessments off on a runner (as
they are on a dev machine), `spctl --assess` returns success whatever the signature, so it proves
nothing — `codesign --verify --deep --strict` is the check that is actually meaningful.
