#!/usr/bin/env python3
# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

"""Generate the Windows icon (`packaging/windows/conic-launcher.ico`).

The third member of the same family as `generate-icons.py` (the freedesktop
hicolor ladder) and `generate-icns.py` (the macOS icon set), and with the same
two constraints:

* **Committed, not generated at package time.** `wix` builds an MSI from the
  `.ico` next to the `.wxs`, and the runner has no Pillow — let alone a reason
  to install one — so a package that could only be built where the generator runs
  is not a package.
* **One master, downsampled.** `app/ui/assets/logo.png` is the 4267px artwork.
  The 512px `app/ui/assets/images/app-icon.png` is what the *window* draws, so it
  is the worst possible source here for the same reason it is the wrong source
  for the `.icns`.

## Why the frames are BMP and not PNG

An `.ico` may store each frame either as a PNG or as a DIB, and Pillow writes
PNGs by default — they are a third of the size. **The MSI's `Icon` table is only
defined for the DIB.** Its `Data` column is a `BITMAPINFOHEADER` followed by the
XOR bitmap and the AND mask, because Windows Installer reads that table through
`msi.dll` and has no PNG decoder anywhere in that path.

WiX does not check any of this. Handed a PNG-frame `.ico` it copies the frames
into the table verbatim and reports success — measured here by the `.msi` coming
out 237KB smaller, which is the PNG frames stored unconverted. That is worse than
a failed build: there is nothing in the build log to point at, and the only
symptom is an installer whose icon is blank or wrong on the user's machine.

Hence `bitmap_format="bmp"`, and hence the `verify` below: the check reads the
`.ico` back and fails here, where the message can be about *this*, rather than in
a release whose packages cannot draw their own icon.

Run:  python3 tools/generate-ico.py
"""

from __future__ import annotations

import pathlib
import struct
import sys

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[1]
MASTER = ROOT / "app/ui/assets/logo.png"
OUT = ROOT / "packaging/windows"
TARGET = OUT / "conic-launcher.ico"

# What Windows actually asks an application icon for, and nothing else:
#
#   16  title bar, small icons view
#   32  medium icons view, the Start menu at 100% scaling, Add/Remove Programs
#   48  large icons view, the Start menu at 200% scaling
#   256 the extra-large views (Apps on Windows 11 shows the package icon here)
#
# 64 and 128 are interpolations of neighbours, and Windows does scale a
# neighbouring frame itself, so they would only make the file bigger. Unlike the
# hicolor ladder, these four are all BMP, which costs 4 bytes per pixel *plus* the
# 1-bit AND mask — about 270KB for the whole file, against 37KB for the same four
# frames as PNGs. That is the price of the `Icon` table accepting them at all,
# and it is the same trade the `.icns` makes in reverse.
SIZES = (16, 32, 48, 256)

# `ICONDIR`: reserved, type (1 = icon), then the frame count.
ICONDIR = struct.Struct("<HHH")
# One `ICONDIRENTRY`. `bWidth`/`bHeight` are `0` for 256 rather than a byte that
# can hold it, and `dwBytesInRes`/`dwImageOffset` are what `verify` walks.
ICONDIRENTRY = struct.Struct("<BBBBHHII")
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
BITMAPINFOHEADER_SIZE = 40


def verify(path: pathlib.Path, expected: tuple[int, ...]) -> None:
    """Fail unless every frame in `path` is a DIB of one of `expected`'s sizes.

    Read straight out of the file rather than through Pillow: Pillow is the thing
    being checked, and it decodes both frame formats into the same `Image`.
    """
    data = path.read_bytes()
    reserved, kind, count = ICONDIR.unpack_from(data)
    if reserved != 0 or kind != 1:
        print(f"{path.name} has a bad ICONDIR (reserved={reserved}, type={kind})")
        sys.exit(1)
    if count != len(expected):
        print(f"{path.name} has {count} frames, expected {len(expected)}")
        sys.exit(1)

    for index in range(count):
        width, height, _, _, _, _, size, offset = ICONDIRENTRY.unpack_from(
            data, ICONDIR.size + ICONDIRENTRY.size * index
        )
        # The two size fields are bytes, so 256 arrives as 0.
        side = width or 256
        frame = data[offset : offset + size]
        if frame[:8] == PNG_MAGIC:
            print(
                f"{path.name}: the {side}x{side} frame is PNG-compressed, and the MSI "
                f"Icon table cannot hold one (regenerate with bitmap_format='bmp')"
            )
            sys.exit(1)
        header = struct.unpack_from("<I", frame)[0]
        if header != BITMAPINFOHEADER_SIZE:
            print(f"{path.name}: the {side}x{side} frame is not a BITMAPINFOHEADER")
            sys.exit(1)
        # The DIB is written bottom-up, which is what `biHeight` doubling means:
        # an XOR bitmap and an AND mask stacked in one buffer.
        height_field = struct.unpack_from("<i", frame, 8)[0]
        if height_field != side * 2:
            print(
                f"{path.name}: the {side}x{side} frame declares a height of "
                f"{height_field}, expected {side * 2} (XOR bitmap + AND mask)"
            )
            sys.exit(1)
        print(f"  {side}x{side}  {size} bytes  BMP")


def main() -> int:
    if not MASTER.is_file():
        print(f"{MASTER.relative_to(ROOT)} is missing; cannot generate the icon")
        return 1

    OUT.mkdir(parents=True, exist_ok=True)

    with Image.open(MASTER) as master:
        # RGBA for the same reason `generate-icons.py` converts: the mark has
        # rounded corners that have to stay see-through, and dropping to "RGB"
        # would put black there.
        master = master.convert("RGBA")
        if master.width < max(SIZES):
            print(
                f"{MASTER.name} is {master.width}px wide, smaller than the "
                f"{max(SIZES)}px the largest frame needs"
            )
            return 1

        # `bitmap_format="bmp"` is the whole point of this script — see the module
        # docstring. LANCZOS, as in the other two generators: the artwork is a
        # flat-coloured mark, and at 16px bilinear drops whole edge pixels instead
        # of averaging them.
        master.save(
            TARGET,
            format="ICO",
            sizes=[(size, size) for size in SIZES],
            bitmap_format="bmp",
        )

    print(f"{TARGET.relative_to(ROOT)}  ({TARGET.stat().st_size} bytes)")
    verify(TARGET, SIZES)

    print("\nCommit the .ico: the MSI is built on a runner with no Pillow.")
    return 0


if __name__ == "__main__":
    sys.exit(main())