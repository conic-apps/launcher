# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

"""Generate the macOS icon set (`packaging/macos/conic-launcher.icns`).

The counterpart of `generate-icons.py` for the hicolor ladder, and with the same
two constraints:

* **Committed, not generated at package time.** `iconutil` exists on macOS and
  nowhere else, so a build machine that is not macOS cannot produce the `.icns`
  — and the `.app` bundle is assembled from files in this directory.
* **One master, downsampled.** `app/ui/assets/logo.png` is the 4267px artwork.
  The 512px `app/ui/assets/images/app-icon.png` is what the *window* draws (it is
  embedded in the binary by `app/src/main.rs`), so it is also the worst possible
  source for a `.icns`: macOS asks for 1024px, and upscaling produces the
  blurriest possible result at the one size a Dock icon is judged at.

`iconutil` takes a directory of pre-rendered PNGs in its own naming scheme rather
than a master image, so the ladder is rendered here and handed over. The
`@2x` entries are why the 1024px render is needed at all: the 512px slot is a
`512x512@2x` one.

Run:  python3 tools/generate-icns.py
"""

from __future__ import annotations

import pathlib
import shutil
import subprocess
import sys
import tempfile

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[1]
MASTER = ROOT / "app/ui/assets/logo.png"
OUT = ROOT / "packaging/macos"
ICONSET = OUT / "conic-launcher.iconset"

# `iconutil`'s naming scheme, and the reason it is written out rather than
# generated: it encodes both the pixel size *and* the scale factor in the file
# name, and the two do not have the same set of combinations. These ten are the
# ones macOS actually looks up, from the 16pt Dock icon to the 512pt one.
#
#   (base px, scale) -> file name
SPECS = [
    (16, 1),
    (16, 2),
    (32, 1),
    (32, 2),
    (128, 1),
    (128, 2),
    (256, 1),
    (256, 2),
    (512, 1),
    (512, 2),
]


def render(source: Image.Image, size: int, scale: int, target: pathlib.Path) -> None:
    pixels = size * scale
    # `Image.resize` with LANCZOS, as in `generate-icons.py`: the artwork is a
    # flat-coloured mark, and at 16px bilinear drops whole edge pixels instead of
    # averaging them.
    source.resize((pixels, pixels), Image.Resampling.LANCZOS).save(target, format="PNG")


def main() -> int:
    if not MASTER.is_file():
        print(f"{MASTER.relative_to(ROOT)} is missing; cannot generate the icon")
        return 1

    if shutil.which("iconutil") is None:
        print("iconutil not found; it ships with macOS and is required here")
        return 1

    largest = max(size * scale for size, scale in SPECS)
    with Image.open(MASTER) as master:
        master = master.convert("RGBA")
        if master.width < largest:
            print(
                f"{MASTER.name} is {master.width}px wide, but the {largest}x{largest} "
                f"render the 512x512@2x entry needs"
            )
            return 1

        OUT.mkdir(parents=True, exist_ok=True)
        # `iconutil` refuses to run on a pre-existing directory in some macOS
        # versions and silently merges stale files in others, so it always gets a
        # fresh one.
        if ICONSET.exists():
            shutil.rmtree(ICONSET)
        ICONSET.mkdir(parents=True)

        for size, scale in SPECS:
            name = (
                f"icon_{size}x{size}.png"
                if scale == 1
                else f"icon_{size}x{size}@{scale}x.png"
            )
            render(master, size, scale, ICONSET / name)

    # Stale renders are removed *before* `iconutil` runs rather than by wiping
    # the directory: the ladder is fixed, so nothing here can legitimately go
    # missing between runs, and a leftover file with an old logo in it is worse
    # than no file at all. Anything not in `SPECS` goes.
    expected = {
        f"icon_{size}x{size}.png" if scale == 1 else f"icon_{size}x{size}@{scale}x.png"
        for size, scale in SPECS
    }
    for stale in ICONSET.iterdir():
        if stale.name not in expected:
            stale.unlink()

    icns = OUT / "conic-launcher.icns"
    # `-c icns` picks the output *type*; the output path is positional. Without
    # `-c`, iconutil infers it from the extension and this still works, but being
    # explicit costs nothing.
    subprocess.run(["iconutil", "-c", "icns", str(ICONSET), "-o", str(icns)], check=True)

    print(f"{icns.relative_to(ROOT)}  ({icns.stat().st_size} bytes)")
    print(f"from {len(SPECS)} renders in {ICONSET.relative_to(ROOT)}")

    # The `.iconset` is left in place: it is the input to the committed `.icns`,
    # so keeping it is what lets someone see (and diff) what actually went in.
    # Add it to `.gitignore` if the diff noise outweighs that.
    print("\nCommit the .icns. The .iconset is build input kept for reference.")
    return 0


if __name__ == "__main__":
    sys.exit(main())