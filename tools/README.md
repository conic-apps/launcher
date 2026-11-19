# tools

Scripts that maintain what the build consumes: the icon sets, the embedded font, the `.po`
catalogues, the disk image's layout. None of them runs during `cargo build`, and every file they
write is **committed** — a clone builds and packages the launcher without any of this tooling.
Nothing verifies that output either, so a regenerated file is a change to review like any other.

## Packaging

`package-linux.sh`, `package-macos.sh` and `package-windows.ps1` are the three entry points the
release jobs call, and they write the artifacts rather than anything committed.
[`../packaging/README.md`](../packaging/README.md) documents them along with the payload they
install.

## Icons

`generate-icons.py` writes `packaging/linux/icons/`, `generate-icns.py` writes
`packaging/macos/conic-launcher.icns` and `generate-ico.py` writes
`packaging/windows/conic-launcher.ico`. All three downsample `ui/assets/logo.png` — the 4267px
master, which is installed nowhere — into the ladder each platform asks for; Pillow is what they
share, and `generate-icns.py` additionally needs `iconutil`, so it only runs on macOS.

Two traps the generators keep out of a human's hands are written up in the platform READMEs: the
`.icns` must not come from `ui/assets/images/app-icon.png`
([`macos/`](../packaging/macos/README.md)), and the `.ico`'s frames have to be BMPs rather than PNGs
([`windows/`](../packaging/windows/README.md)).

## The merged font

Slint has no CSS `unicode-range` fallback, so a digits-only face cannot be layered under the text
face at runtime — Comfortaa and Nunito's digits are welded into one variable font instead:

```bash
python3 tools/merge-digit-font.py            # writes ui/assets/fonts/ComfortaaNunito.ttf
python3 tools/merge-digit-font.py --verify   # then render-compares every weight
```

Inputs are `fonts/Comfortaa-Latin.woff2` and `fonts/Nunito-Digits.woff2` beside this README; the
merge needs `fontTools` and `numpy`, and `--verify` (through `verify_merge.py`) adds Pillow.

## The `.dmg` window

`package-macos.sh` calls `dmg-ds-store.py` to lay the Finder window out, and
`render-dmg-background.py` produces the picture it copies in — the latter needs `fontTools`, Pillow
and an SVG rasterizer (`rsvg-convert` or `resvg`). What the format limits, such as the 10pt label
size and the `.svg` owning the window's geometry, is in
[`../packaging/macos/README.md`](../packaging/macos/README.md).

## i18n catalogues

```bash
find ui -name '*.slint' | xargs slint-tr-extractor -o messages.pot
python3 tools/update-i18n.py messages.pot
```

`update-i18n.py` takes the `(context, msgid)` pairs from the extractor's `.pot` and the translations
from a table inside itself, and rewrites the twelve catalogues; a pair the `.pot` has and the table
does not is reported and left out rather than written as an identity. The script is standard library
only — the external piece is `slint-tr-extractor`, released alongside `slint`. The crash dialog's
labels sit next to it in `crash-i18n.tsv`, one row per string and one column per language in the
order the settings picker lists them; `en_US` has no column because an ASCII msgid is its own
English translation.

## Icon names in the Slint tree

`python3 tools/check-icon-names.py` exits 1 when an `AppIcon { name: … }` or an `icon: …` binding
names an SVG that is not under `ui/assets/icons/`. That is the one way an icon goes missing with
nothing logged — `Icons` returns `""` and the `Path` draws nothing — so it is worth running after
moving or renaming an asset. Standard library only, and nothing runs it in CI.
