#!/usr/bin/env python3
# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

"""Generate the hicolor icon set the Linux packages install.

`app/ui/assets/logo.png` is the 4267px master artwork, and it is not usable
as-is by a package: a desktop environment asks the icon theme for a fixed
ladder of sizes (`16x16` up to `512x512`), and the master is none of them. So the
rasters are downsampled here into `packaging/linux/icons/`.

`app/ui/assets/images/logo-flat.svg` is the same mark as a vector, and is copied
in as the `scalable` entry. Its framing is not identical to the rasters' -- the
master has a few percent of padding around the mark and the SVG fills its
viewBox -- which is why it is copied rather than re-framed to match. Re-framing
means editing somebody's artwork file to suit a packaging script, and an icon
theme that picks the scalable entry over the rasters shows a slightly larger
mark, which is not a defect.

That output directory is **committed**: `dpkg-deb`, `rpmbuild` and `makepkg` all
run on machines that have neither Python nor Pillow, and a package that could
only be built on the author's machine is not a package. Regenerate with this
script when the logo changes, and commit the result alongside it.

Debian policy wants an icon no smaller than 128x128 for every package that ships
one, and AppImage wants a 256x256 `.DirIcon` at the AppDir root; both are in the
ladder, so one set serves deb, rpm, AppImage and the Arch PKGBUILD.

Run:  python3 tools/generate-icons.py
"""

from __future__ import annotations

import pathlib
import shutil
import sys

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[1]
MASTER = ROOT / "app/ui/assets/logo.png"
VECTOR = ROOT / "app/ui/assets/images/logo-flat.svg"
OUT = ROOT / "packaging/linux/icons"

# The freedesktop "standard" icon sizes, all nine of them. Anything outside this
# list is not looked up by an icon theme, so a tenth raster would be dead weight
# in every package.
SIZES = (16, 22, 24, 32, 48, 64, 128, 256, 512)


def main() -> int:
    for required in (MASTER, VECTOR):
        if not required.is_file():
            print(f"{required.relative_to(ROOT)} is missing; cannot generate icons")
            return 1

    OUT.mkdir(parents=True, exist_ok=True)

    with Image.open(MASTER) as master:
        # An RGBA buffer is what every consumer expects: the mark has rounded
        # corners that have to stay see-through, and dropping to "RGB" would put
        # black there.
        master = master.convert("RGBA")
        if master.width < max(SIZES):
            print(
                f"{MASTER.name} is {master.width}px wide, smaller than the "
                f"{max(SIZES)}px the ladder needs"
            )
            return 1

        for size in SIZES:
            # LANCZOS rather than Pillow's default bilinear: the artwork is a
            # flat-coloured mark, and at 16px bilinear drops whole edge pixels
            # rather than averaging them. Pillow has no box-filter downsampling
            # primitive (that is `reduce`, on a source far larger than 2x), so
            # LANCZOS is the best of what is available here.
            resized = master.resize((size, size), Image.Resampling.LANCZOS)
            target = OUT / f"{size}x{size}.png"
            resized.save(target, format="PNG", optimize=True)
            print(f"{target.relative_to(ROOT)}  ({target.stat().st_size} bytes)")

    # A copy, not a conversion: `logo-flat.svg` *is* the artwork's vector
    # original, and a rasteriser in the middle would only lose detail. It is also
    # why the `scalable` icon is not generated from the rasters.
    scalable = OUT / "scalable.svg"
    shutil.copyfile(VECTOR, scalable)
    print(f"{scalable.relative_to(ROOT)}  (copied from {VECTOR.name})")

    print(f"\n{len(SIZES)} rasters + 1 scalable icon written to {OUT.relative_to(ROOT)}")
    print("Commit the result: the packages are built on machines without Pillow.")
    return 0


if __name__ == "__main__":
    sys.exit(main())