# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

"""Write the `.dmg`'s Finder layout: `tools/package-macos.sh` calls this on the
staging directory and it drops in `.DS_Store` and `.background.png`.

**Why a script and not Finder.** `hdiutil create -srcfolder` bakes in whatever
the staging directory already holds, and Finder cannot be scripted to lay out a
folder that does not exist yet: driving it would need an Automation grant and a
window on screen, neither of which a release build has. So the layout is written
directly, which is what `create-dmg` and `dmgbuild` do too, and why
`tools/package-macos.sh` used to ship an image whose window opened as a plain
list of two names.

**Why no `ds_store` or `dmgbuild` dependency.** Both are `pip install`, and this
script's contract with its build machines is that they need nothing beyond macOS
itself: `package-macos.sh` already refuses to run without `hdiutil` and
`codesign`. So the two formats involved are implemented here against the
standard library, which for a file holding six fixed records is a couple of
hundred lines rather than a dependency tree:

* `.DS_Store` is a Buddy allocator wrapping a B-tree of records (TN1150, plus
  the reverse engineering in Alastair Houghton's `ds_store`).
* `icvp`'s `backgroundImageAlias` is a Carbon Alias record (TN1150, Alias
  Records) naming the `.background.png` beside it.

**One source of truth for the geometry.** The window's size and the position of
the `Applications` icon are read out of `packaging/macos/dmg/background.svg`'s
`#applications-icon-cell` marker — that icon's own 128pt cell, and not the dashed
frame around it, which wraps the icon *and* the name Finder draws under it and so
has a different middle. Finder paints the background at 1:1 from the content
view's top-left corner, so a window the wrong size shows a misaligned
background, and a picture the wrong size shows scrollbars. Reading the `.svg`
means the artwork and the placement cannot disagree; the app icon is then the
mirror image of the `Applications` one about the window's centre line, which is
what puts the two on either side of the middle.

Run:  python3 tools/dmg-ds-store.py --volume <name> <staging-directory>
      python3 tools/dmg-ds-store.py --check
"""

from __future__ import annotations

import argparse
import pathlib
import plistlib
import shutil
import struct
import sys
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parents[1]
DMG = ROOT / "packaging/macos/dmg"
BACKGROUND_SVG = DMG / "background.svg"
BACKGROUND_PNG = DMG / "background.png"

# What `package-macos.sh` puts on the volume. Both are checked for below, so a
# rename on either side is an error rather than a layout that quietly points at
# nothing.
APP_BUNDLE = "conic-launcher.app"
APPLICATIONS_LINK = "Applications"
# Finder looks for a folder's background under this name at its root and works
# the format out of the extension, which is why the picture has to be something
# it can decode: PNG is.
BACKGROUND_NAME = ".background.png"

# The icon size the layout asks for, in points. It is a layout fact rather than a
# drawing one because the `.svg`'s marker rectangle for the `Applications` icon is
# this big, and the two have to agree.
ICON_SIZE = 128

# The size Finder draws an icon's name at.
#
# There is no way to make that name any colour: `backgroundColor*` in `icvp` only
# tints a solid fill, and no text colour key exists in the format at all, so over
# this backdrop the name comes out near-black however it is set. The size is the
# only lever, and it is worth pulling down: 10 is the smallest Finder accepts, and
# it rejects the whole `icvp` — silently, leaving a default window with no
# background — at 9 or below.
LABEL_SIZE = 10

# Where the window opens. The x is a plain screen coordinate; the y is measured
# from the *bottom* of the screen, which is Finder's own convention and not a typo
# (a `bwsp` whose top is 360 on a 1117pt display opens 257pt down).
#
# It is a single number for displays of different heights, because nothing at build
# time knows the height of the display the image will be opened on. 360 is the
# compromise that stays on screen from a 900pt laptop to a large display, and the
# position is cosmetic: Finder clamps it, and the user moves the window once.
WINDOW_LEFT = 200
WINDOW_TOP_FROM_BOTTOM = 360

# How far outside the window the background picture's own icon is parked, for a
# Finder that is showing hidden files and would otherwise draw it in the middle of
# the layout. A Finder that hides dotfiles, which is the default and the case on
# every machine this ships to, never reads the record at all.
PARKED = 4_000

SVG = "{http://www.w3.org/2000/svg}"


def die(message: str) -> "None":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


# ---------------------------------------------------------------------------
# The geometry, read out of the background
# ---------------------------------------------------------------------------


class Geometry:
    """The window and the two icon placements, in points.

    Both icon positions are derived rather than stated: `Applications` sits at the
    centre of the marker rectangle the `.svg` draws for it, and the app is that
    one's mirror image about the window's centre line. Move the marker in the `.svg`
    and both icons follow it, which is the only way the artwork and the layout
    stay in step.
    """

    def __init__(self, svg: pathlib.Path) -> None:
        root = ET.parse(svg).getroot()
        self.width = self._length(root, "width")
        self.height = self._length(root, "height")

        # The marker rectangle, not the dashed frame: the frame is drawn to wrap
        # the icon *and* its name, so its middle is not the icon's. The marker is
        # the icon's own cell and nothing else, which leaves the two a plain
        # centre-of-a-rectangle away from agreeing.
        cell = root.find(f".//{SVG}rect[@id='applications-icon-cell']")
        if cell is None:
            die(f"{svg} has no #applications-icon-cell rectangle marking where the Applications icon goes")
        self.applications = (
            self._length(cell, "x") + self._length(cell, "width") / 2.0,
            self._length(cell, "y") + self._length(cell, "height") / 2.0,
        )
        self.app = (self.width - self.applications[0], self.applications[1])

        size = self._length(cell, "width")
        if abs(size - ICON_SIZE) > 0.5:
            die(f"the Applications icon cell is {size:g}pt wide but the layout asks for {ICON_SIZE}pt")

    @staticmethod
    def _length(element: ET.Element, name: str) -> float:
        raw = element.get(name)
        if raw is None or not raw.strip().isdigit():
            die(f"<{element.tag.rsplit('}', 1)[-1]}> has no unitless {name}")
        return float(raw)

    @staticmethod
    def iloc(centre: tuple[float, float]) -> tuple[int, int]:
        """A cell's centre in points as the `(x, y)` Finder wants in an `Iloc`.

        With no offset at all: Finder draws the icon itself centred on this point
        and hangs the name underneath it, so there is nothing to give back.
        """
        return round(centre[0]), round(centre[1])

    def window_bounds(self) -> str:
        """`bwsp`'s `WindowBounds`, as the string Finder parses.

        The second pair is the window's *size*, not its bottom-right corner.
        Corners work too, in the sense that Finder opens the window and then
        resizes it, which is the same thing said the long way round.
        """
        return (
            f"{{{{{WINDOW_LEFT}, {WINDOW_TOP_FROM_BOTTOM}}}, "
            f"{{{self.width:g}, {self.height:g}}}}}"
        )


# ---------------------------------------------------------------------------
# A Carbon alias to the background picture
# ---------------------------------------------------------------------------


def background_alias(volume: str, filename: str) -> bytes:
    """An Alias record for `filename` at the root of the volume called `volume`.

    Finder names a folder's background with an alias rather than a path, and the
    record `dmgbuild` writes is one it gets by calling into CoreServices against
    the *mounted* volume. That is not available here: the image does not exist yet
    when the staging directory is written, and the file id such a call would
    capture is only assigned when the filesystem is created.

    So the record carries the paths and no file id, and Finder resolves it that
    way: the POSIX path (tag 18) is relative to the volume, and the record names
    the volume. That is why the volume name has to match the `-volname` the image
    is built with.
    """
    tagged = b""

    def add(tag: int, payload: bytes) -> None:
        nonlocal tagged
        tagged += struct.pack(">hh", tag, len(payload)) + payload
        if len(payload) & 1:
            tagged += b"\0"

    def add_text(tag: int, text: str) -> None:
        # Tags 14 and 15 carry a byte count and a character count in their own two
        # fields rather than a length in the shared one, which is why they are not
        # written through `add`.
        nonlocal tagged
        raw = text.encode("utf-16-be")
        tagged += struct.pack(">hhh", tag, len(raw) + 2, len(raw) // 2) + raw

    add(0, volume.encode("utf-8"))
    add(2, volume.encode("utf-8") + b":" + filename.encode("utf-8"))
    add_text(14, filename)
    add_text(15, volume)
    add(18, ("/" + filename).encode("utf-8"))
    add(19, ("/Volumes/" + volume).encode("utf-8"))
    tagged += struct.pack(">hh", -1, 0)

    # The volume and file creation dates are Mac epoch seconds. They are left at
    # zero because the volume does not exist yet, and nothing consults them when
    # the record is resolved by path.
    fixed = struct.pack(
        ">h28pI2shI64pII4s4shhI2s10s",
        0,                       # alias kind: a file, not a folder
        volume.encode("utf-8"), 0, b"H+", 0,
        2,                       # the root directory's file id, which is fixed
        filename.encode("utf-8"),
        0,                       # file id: unknown until the image exists
        0,
        b"\0\0\0\0", b"\0\0\0\0",  # creator and type codes
        -1, -1,                   # levels from and to
        0, b"\0\0", b"\0" * 10,
    )
    return struct.pack(">4shh", b"\0\0\0\0", 8 + len(fixed) + len(tagged), 2) + fixed + tagged


# ---------------------------------------------------------------------------
# `.DS_Store`: a Buddy allocator wrapping a B-tree of records
# ---------------------------------------------------------------------------

PAGE = 4096
# The allocator's own root block, which holds the block address table, the name
# table and the free lists, sits here and is this big in a file the format creates
# empty. Both numbers are load bearing: they are in the file's header.
ALLOCATOR_ROOT = 2048
# The size of the root block in the file the format creates empty, and the size
# its header claims. It is the claim that matters: a reader sizes the block from
# the header, and the real size is re-derived from the address table on the way
# out.
ALLOCATOR_ROOT_HEADER = 1264
# Sixteen bytes of state the header carries that this script has no opinion about.
# They are copied from a file the format creates, because a reader that has an
# opinion about them will be reading the values it expects.
ALLOCATOR_SEED = b"\x00\x00\x10\x0c\x00\x00\x00\x87\x00\x00\x20\x0b\x00\x00\x00\x00"
# The logical file starts four bytes into the physical one: those first four are a
# version word that the header's own offsets are relative to.
PREFIX = 4


class Buddy:
    """The allocator `.DS_Store` keeps its blocks in.

    Only what writing a fresh file needs: a first-fit power-of-two allocator over
    the whole address space, plus the ability to rewrite the root block that
    records it. A block's address carries its width in the low five bits, which is
    why offsets are masked and shifted rather than used directly.
    """

    def __init__(self) -> None:
        # A new file is the 2GB region the format splits up. Its allocator root
        # block starts life at 2**11 holding only the start of its own address
        # table, and is resized to whatever the table needs the first time the
        # file is written out; every other width in between is free.
        self.blocks: list[int] = [ALLOCATOR_ROOT | width_of(ALLOCATOR_ROOT_HEADER)]
        self.free: list[list[int]] = [[] for _ in range(32)]
        self.free[5] = [32, 64]
        for width in range(6, 11):
            self.free[width] = [1 << width]
        for width in range(12, 31):
            self.free[width] = [1 << width]

    def allocate(self, size: int, block: int | None = None) -> int:
        """Reserve a block of at least `size` bytes, and return its index."""
        if block is None:
            try:
                block = self.blocks.index(0)
            except ValueError:
                self.blocks.append(0)
                block = len(self.blocks) - 1

        width = width_of(size)
        address = self.blocks[block]
        if address:
            if (address & 0x1F) == width:
                return block
            self.release(offset_of(address), address & 0x1F)
            self.blocks[block] = 0

        self.blocks[block] = self.reserve(width) | width
        return block

    def resize(self, block: int, size: int) -> None:
        """Grow or shrink a block, which the root block needs every time it fills."""
        width = width_of(size)
        address = self.blocks[block]
        if address and (address & 0x1F) == width:
            return
        if address:
            self.release(offset_of(address), address & 0x1F)
            self.blocks[block] = 0
        self.blocks[block] = self.reserve(width) | width

    def reserve(self, width: int) -> int:
        """An offset for a block of exactly `width`, halving a larger free one."""
        while not self.free[width]:
            width += 1
        target = width
        while width > target:
            offset = self.free[width].pop(0)
            width -= 1
            self.free[width] = [offset, offset ^ (1 << width)]
        return self.free[target].pop(0)

    def release(self, offset: int, width: int) -> None:
        """Give a block back, merging it with its buddy where that is free too."""
        while width < 31:
            buddy = offset ^ (1 << width)
            if buddy not in self.free[width]:
                break
            self.free[width].remove(buddy)
            offset &= buddy
            width += 1
        self.free[width].append(offset)
        self.free[width].sort()

    def root_block_size(self) -> int:
        """How many bytes the allocator's own root block needs.

        Its layout is the block address table, the name table, then one count and
        one list per width. Addresses are padded out to a multiple of 256, so the
        answer cannot be read off the number of blocks and has to be summed.
        """
        return (
            8
            + 4 * ((len(self.blocks) + 255) & ~255)
            + 4
            + 5 + 4
            + sum(4 + 4 * len(free) for free in self.free)
        )

    def root_block(self, names: dict[bytes, int]) -> bytes:
        """Serialise the allocator's root block."""
        out = bytearray()
        out += struct.pack(">II", len(self.blocks), 0)
        for address in self.blocks:
            out += struct.pack(">I", address)
        out += b"\0" * (4 * (((len(self.blocks) + 255) & ~255) - len(self.blocks)))
        out += struct.pack(">I", len(names))
        for name, block in sorted(names.items()):
            out += struct.pack(">B", len(name)) + name + struct.pack(">I", block)
        for free in self.free:
            out += struct.pack(">I", len(free))
            out += struct.pack(f">{len(free)}I", *free)
        return bytes(out)


def width_of(size: int) -> int:
    """The address width for a block of `size` bytes; 32 bytes is the floor."""
    return max(size.bit_length(), 5)


def offset_of(address: int) -> int:
    return address & ~0x1F


def size_of(address: int) -> int:
    return 1 << (address & 0x1F)


def encode_record(filename: str, code: bytes, typecode: bytes, value: object) -> bytes:
    """One record: its name, its four-character code, and its value.

    The value's encoding is named by the four bytes of type code that follow the
    record's own; there is no other tag, so the reader switches on that.
    """
    name = filename.encode("utf-16-be")
    out = struct.pack(">I", len(name) // 2) + name + code + typecode
    if typecode == b"bool":
        out += struct.pack(">?", value)
    elif typecode in (b"long", b"shor"):
        out += struct.pack(">I", value)
    elif typecode == b"blob":
        out += struct.pack(">I", len(value)) + value
    elif typecode == b"ustr":
        text = value.encode("utf-16-be")
        out += struct.pack(">I", len(text) // 2) + text
    elif typecode == b"type":
        out += struct.pack(">4s", value)
    elif typecode in (b"comp", b"dutc"):
        out += struct.pack(">Q", value)
    else:
        die(f"unknown .DS_Store value type {typecode!r}")
    return out


def build_ds_store(records: list[tuple[str, bytes, bytes, object]]) -> bytes:
    """A whole `.DS_Store` holding `records`, as a single B-tree leaf.

    A disk image window needs six records and never needs to grow, so this builds
    the one-leaf tree that holds them and says so rather than implementing split
    and merge. A record that did not fit is a bug worth hearing about: a silently
    truncated layout is worse than either.
    """
    ordered = sorted(records, key=lambda record: (record[0].lower(), record[1]))

    leaf = struct.pack(">II", 0, len(ordered)) + b"".join(encode_record(*r) for r in ordered)
    if len(leaf) > PAGE:
        die(f"the layout needs {len(leaf)} bytes of .DS_Store node, which does not fit in {PAGE}")

    buddy = Buddy()
    if offset_of(buddy.blocks[0]) != ALLOCATOR_ROOT:
        die("the allocator's root block is not where the format puts it")
    # Index 0 is the allocator's own root block, which the file header points at;
    # the B-tree's superblock and its root node come after it. The root block is
    # resized to fit its own address table before it is written, which is also
    # what decides the offset and size the header ends up claiming.
    superblock = buddy.allocate(20)
    node = buddy.allocate(PAGE)

    # The root block is sized to its own address table first, because that size
    # is what the file header ends up claiming for it.
    root = buddy.root_block({b"DSDB": superblock})
    buddy.resize(0, len(root))

    blocks = {index: bytearray(size_of(address)) for index, address in enumerate(buddy.blocks)}
    blocks[node][: len(leaf)] = leaf
    # The superblock: the tree's root node, its depth, the record and node
    # counts, and the node size.
    #
    # The three counts are written as zero, which is not a typo and not laziness.
    # A `.DS_Store` written by `ds_store` — and so every image `dmgbuild` has
    # shipped — leaves them zeroed even when it knows the real numbers, and
    # Finder reads such a file; filling them in makes it reject the whole file
    # and fall back to a default window. A one-leaf tree has no levels to walk and
    # nothing to count, so the counts carry no information either way.
    blocks[superblock][:20] = struct.pack(">IIIII", node, 0, 0, 0, PAGE)
    blocks[0][: len(root)] = root

    end = max(PREFIX + offset_of(address) + size_of(address) for address in buddy.blocks)
    header = struct.pack(
        ">I4sIII16s",
        1,
        b"Bud1",
        offset_of(buddy.blocks[0]),
        size_of(buddy.blocks[0]),
        offset_of(buddy.blocks[0]),
        ALLOCATOR_SEED,
    )
    out = bytearray(end)
    out[: len(header)] = header
    for index, address in enumerate(buddy.blocks):
        start = PREFIX + offset_of(address)
        out[start : start + len(blocks[index])] = blocks[index]
    return bytes(out)


# ---------------------------------------------------------------------------
# The layout itself
# ---------------------------------------------------------------------------


def plist_blob(value: dict[str, object]) -> bytes:
    """A `bwsp`/`icvp` payload: a binary plist, as Finder stores them.

    Sorted, because a binary plist's key order is whatever the writer felt like
    and an unsorted one makes every rebuild of this file a spurious diff.
    """
    return plistlib.dumps(value, fmt=plistlib.FMT_BINARY, sort_keys=True)


def layout(geometry: Geometry, background: bytes) -> list[tuple[str, bytes, bytes, object]]:
    """Every record the window's layout is made of."""
    browser = {
        # Finder's own spelling of "no furniture but the title bar". Without these
        # the window opens with a sidebar, a toolbar and a path bar, and the
        # picture is painted behind all three.
        "ContainerShowSidebar": False,
        "PreviewPaneVisibility": False,
        "ShowPathbar": False,
        "ShowSidebar": False,
        "ShowStatusBar": False,
        "ShowTabView": False,
        "ShowToolbar": False,
        "SidebarWidth": 163,
        "WindowBounds": geometry.window_bounds(),
    }
    icon_view = {
        "arrangeBy": "none",
        # White, and inert: these only colour a window whose background is a solid
        # fill, and this one is a picture. They are written because a reader may
        # look at them before it looks at the type, and because `dmgbuild` writes
        # them; the strip of window below the picture, if a display ever leaves
        # one, is the only thing they would ever affect.
        "backgroundColorBlue": 1.0,
        "backgroundColorGreen": 1.0,
        "backgroundColorRed": 1.0,
        # 2 is "a picture", and the picture is named by an alias rather than by a
        # path, which is what `background_alias` above is for.
        "backgroundImageAlias": background,
        "backgroundType": 2,
        "gridOffsetX": 0.0,
        "gridOffsetY": 0.0,
        "gridSpacing": 100.0,
        "iconSize": float(ICON_SIZE),
        "labelOnBottom": True,
        "scrollPositionX": 0.0,
        "scrollPositionY": 0.0,
        "showIconPreview": False,
        "showItemInfo": False,
        "textSize": float(LABEL_SIZE),
        "viewOptionsVersion": 1,
    }
    return [
        (".", b"bwsp", b"blob", plist_blob(browser)),
        (".", b"icvp", b"blob", plist_blob(icon_view)),
        # "long" 1 is the icon view, and it is a record of its own: a window with
        # the picture but no `vSrn` opens as a list.
        (".", b"vSrn", b"long", 1),
        (APP_BUNDLE, b"Iloc", b"blob", struct.pack(">II", *geometry.iloc(geometry.app))),
        (APPLICATIONS_LINK, b"Iloc", b"blob", struct.pack(">II", *geometry.iloc(geometry.applications))),
        (
            BACKGROUND_NAME,
            b"Iloc",
            b"blob",
            struct.pack(">II", round(geometry.width + PARKED), round(geometry.height + PARKED)),
        ),
    ]


# ---------------------------------------------------------------------------


def check(geometry: Geometry) -> int:
    """Print the derived layout, and verify the rendered picture against it."""
    from PIL import Image

    print(f"window         {geometry.width:g} x {geometry.height:g} pt, at {WINDOW_LEFT}, {WINDOW_TOP_FROM_BOTTOM} from the bottom")
    print(f"Applications   cell centre   {geometry.applications[0]:g}, {geometry.applications[1]:g} -> Iloc {geometry.iloc(geometry.applications)}")
    print(f"app icon       mirrored      {geometry.app[0]:g}, {geometry.app[1]:g} -> Iloc {geometry.iloc(geometry.app)}")

    with Image.open(BACKGROUND_PNG) as image:
        size = image.size
        dpi = image.info.get("dpi", (0.0, 0.0))[0]
    if not dpi:
        die(f"{BACKGROUND_PNG.name} has no resolution; re-render it with tools/render-dmg-background.py")
    points = (size[0] * 72.0 / dpi, size[1] * 72.0 / dpi)
    if (round(points[0]), round(points[1])) != (geometry.width, geometry.height):
        die(
            f"{BACKGROUND_PNG.name} is {size[0]}x{size[1]} px at {dpi:g} dpi, which is "
            f"{points[0]:.2f}x{points[1]:.2f} pt, but the window is "
            f"{geometry.width:g}x{geometry.height:g} pt; re-render it with "
            "tools/render-dmg-background.py"
        )
    print(f"background     {size[0]}x{size[1]} px at {dpi:g} dpi = {points[0]:g}x{points[1]:g} pt")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "staging",
        nargs="?",
        type=pathlib.Path,
        help="the directory to write .DS_Store and .background.png into",
    )
    parser.add_argument(
        "--volume",
        help="the volume name `package-macos.sh` passes to hdiutil's -volname",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="print the derived layout and verify the rendered picture against it",
    )
    arguments = parser.parse_args()

    geometry = Geometry(BACKGROUND_SVG)
    if arguments.staging is None:
        return check(geometry)

    # Required rather than defaulted: a guess here is a window with no background,
    # which looks like a rendering bug and is not one.
    if not arguments.volume:
        die("--volume is required: Finder resolves the background through an alias that names it")
    staging = arguments.staging
    for item in (APP_BUNDLE, APPLICATIONS_LINK):
        if not (staging / item).exists():
            die(f"{staging}/{item} is missing; the layout is written for a directory that has it")

    shutil.copyfile(BACKGROUND_PNG, staging / BACKGROUND_NAME)
    (staging / ".DS_Store").write_bytes(
        build_ds_store(layout(geometry, background_alias(arguments.volume, BACKGROUND_NAME)))
    )
    print(f"     wrote {staging / BACKGROUND_NAME} and {staging / '.DS_Store'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())