# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

"""Render the `.dmg` window background (`packaging/macos/dmg/background.svg`).

Finder paints a folder window's background from a raster image, so the disk image
carries a PNG. The `.svg` next to it is the design those pixels come from, and
both are committed for the same reason `conic-launcher.icns` is: the `.dmg` is
assembled by `tools/package-macos.sh` on a machine that has neither an SVG
rasterizer nor this project's fonts, and a committed artifact is one that cannot
be missing.

**The text becomes outlines here rather than being resolved by the rasterizer.**
`background.svg` asks for `"Comfortaa Nunito"`, which exists nowhere on a build
machine: `tools/merge-digit-font.py` bakes it out of Comfortaa and a Nunito digit
subset, and the result is embedded in the launcher *binary*, not installed into
the system. A rasterizer handed a `font-family` it cannot resolve substitutes
whatever it has and reports nothing, which would ship a disk image whose heading
is in the wrong face with no error anywhere. So the glyphs are laid out here --
from the same file, with the `wght` axis instanced per `font-weight` and the GPOS
`kern` pairs applied -- and handed over as `<path>`.

Run:  python3 tools/render-dmg-background.py
"""

from __future__ import annotations

import argparse
import pathlib
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

from fontTools.misc.transform import Transform
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = pathlib.Path(__file__).resolve().parents[1]
DMG = ROOT / "packaging/macos/dmg"
SOURCE = DMG / "background.svg"
OUTPUT = DMG / "background.png"

# The font the launcher embeds, which is also `Theme.font-family`
# (`app/ui/theme.slint`). It is one variable font, so the heading's two weights
# are two instances of this file rather than two files.
FONT = ROOT / "app/ui/fonts/ComfortaaNunito.ttf"

# Device pixels per point. 2, so that the one-point grid rules and the
# gradient-filled heading stay sharp on a Retina display, where Finder asks the
# picture for twice the pixels it is going to draw it into. The window's size in
# points is not repeated here: it is read out of the `.svg`'s own `width` and
# `height`, because Finder paints the picture 1:1 into the content view and two
# files disagreeing about the window is the one bug that cannot be seen in a
# preview.
SCALE = 2

SVG = "{http://www.w3.org/2000/svg}"


def die(message: str) -> "None":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


# ---------------------------------------------------------------------------
# Text
# ---------------------------------------------------------------------------


class Shaper:
    """Lays runs of a `<text>` element out as positioned glyph outlines.

    Two things here are not optional if the result is to look like the app:

    * **The `wght` axis is instanced per run.** `ComfortaaNunito` is variable
      over 300..700 and the heading mixes 400 and 700 inside one string; asking
      for the outlines at the default instance and scaling them afterwards would
      make the bold run a stretched regular.
    * **The GPOS `kern` pairs are applied.** Comfortaa is a spaced-out geometric
      face and it does kern: this exact string carries `r`/`a`, `C`/`o`, `y`/`o`,
      `p`/`y`, `f`/`o`, `l`/`d` and `r`/`e` pairs, and dropping them leaves the
      words visibly gappy beside the same words rendered by the app.
    """

    # Tables that are dropped from the in-memory font before it is instanced.
    #
    # `GPOS`/`GSUB`/`GDEF` go because this script does the kerning itself (see
    # `_read_kerning`) and the runs are plain Latin, so there is nothing in them
    # to use; `HVAR`/`avar`/`STAT` go because the instancer recomputes `hmtx`
    # from the `gvar` advance deltas anyway, and leaving them in only adds a
    # second answer to the same question.
    #
    # The reason this has to happen at all is `tools/merge-digit-font.py`: it
    # bakes Nunito's digit glyphs into Comfortaa as extra `gvar` masters, and the
    # two disagree about glyph order, which is exactly what
    # `varLib.merger.InconsistentGlyphOrder` is. Dropping the layout tables keeps
    # the outlines, the metrics and the axis, and removes the merge.
    DROPPED = ("GDEF", "GPOS", "GSUB", "HVAR", "avar", "STAT")

    def __init__(self, path: pathlib.Path) -> None:
        self._font = TTFont(path)
        self._units_per_em = self._font["head"].unitsPerEm
        axis, = self._font["fvar"].axes
        if axis.axisTag != "wght":
            die(f"{path} has no 'wght' axis; the render script assumes one")
        self._axis_range = (axis.minValue, axis.defaultValue, axis.maxValue)
        self._kerning = self._read_kerning()
        for tag in self.DROPPED:
            if tag in self._font:
                del self._font[tag]
        self._instances: dict[int, TTFont] = {}

    def _read_kerning(self) -> dict[tuple[str, str], int]:
        """Every `kern` pair adjustment in the font, as a lookup table.

        Only the PairPos format this file uses is handled, and that is *checked*
        rather than assumed: a second format would otherwise produce text with no
        kerning at all, which reads as a font bug rather than a script bug.
        """
        table = self._font["GPOS"].table
        pairs: dict[tuple[str, str], int] = {}
        if table is None:
            return pairs

        wanted: set[int] = set()
        for script in table.ScriptList.ScriptRecord:
            langsys = script.Script.DefaultLangSys
            if langsys is not None:
                wanted.update(langsys.FeatureIndex)

        for index in sorted(wanted):
            feature = table.FeatureList.FeatureRecord[index]
            if feature.FeatureTag != "kern":
                continue
            for lookup_index in feature.Feature.LookupListIndex:
                for subtable in table.LookupList.Lookup[lookup_index].SubTable:
                    if subtable.Format != 1:
                        die(
                            "the embedded font uses a PairPos format this script "
                            "does not read; regenerate the font, or teach "
                            "tools/render-dmg-background.py the format"
                        )
                    for first, pair_set in zip(subtable.Coverage.glyphs, subtable.PairSet):
                        for pair in pair_set.PairValueRecord:
                            if pair.Value1 is not None and pair.Value1.XAdvance:
                                pairs[(first, pair.SecondGlyph)] = pair.Value1.XAdvance
        return pairs

    def _instance(self, weight: int) -> TTFont:
        if weight not in self._instances:
            low, _, high = self._axis_range
            self._instances[weight] = instancer.instantiateVariableFont(
                self._font,
                {"wght": min(max(weight, low), high)},
                inplace=False,
                updateFontNames=False,
            )
        return self._instances[weight]

    def measure(self, text: str, weight: int, size: float) -> float:
        """The advance width of `text`, in user units."""
        font = self._instance(weight)
        cmap, hmtx = font.getBestCmap(), font["hmtx"]
        scale = size / self._units_per_em
        width = 0.0
        previous: str | None = None
        for character in text:
            if previous is not None:
                width += self._kern(previous, character, cmap)
            glyph = cmap.get(ord(character))
            if glyph is not None:
                width += hmtx[glyph][0]
            previous = character
        return width * scale

    def outline(self, text: str, weight: int, size: float, x: float, y: float) -> str:
        """The `d` for one run, with `(x, y)` at its baseline start.

        Each glyph goes through a `TransformPen` carrying the scale, the flip
        from the font's y-up space into SVG's y-down, and the pen position. Doing
        it as one matrix rather than rewriting coordinates keeps the outline data
        exactly as the font stores it and covers every pen method, including the
        quadratics that TrueType glyphs are actually made of.
        """
        font = self._instance(weight)
        glyph_set, cmap, hmtx = font.getGlyphSet(), font.getBestCmap(), font["hmtx"]
        scale = size / self._units_per_em

        pen = SVGPathPen(glyph_set, ntos=lambda value: f"{value:.1f}")
        cursor = x
        previous: str | None = None
        for character in text:
            if previous is not None:
                cursor += self._kern(previous, character, cmap) * scale
            glyph = cmap.get(ord(character))
            if glyph is not None:
                glyph_set[glyph].draw(TransformPen(pen, Transform(scale, 0, 0, -scale, cursor, y)))
                cursor += hmtx[glyph][0] * scale
            previous = character

        return pen.getCommands()

    def _kern(self, left: str, right: str, cmap: dict) -> int:
        first, second = cmap.get(ord(left)), cmap.get(ord(right))
        if first is None or second is None:
            return 0
        return self._kerning.get((first, second), 0)


def runs_of(element: ET.Element, default_weight: int) -> list[tuple[str, int]]:
    """The `(string, weight)` runs of one `<text>` element.

    A `<tspan>` is a weight change rather than a shape: the heading is a single
    string with one bold word in the middle, and laying it out as runs is what
    lets `text-anchor="middle"` still centre the whole line rather than each run.
    """
    runs: list[tuple[str, int]] = []
    if element.text:
        runs.append((element.text, default_weight))
    for child in element:
        weight = int(child.get("font-weight", default_weight))
        if child.text:
            runs.append((child.text, weight))
        if child.tail:
            runs.append((child.tail, default_weight))
    return runs


def inherited(element: ET.Element, parent: ET.Element, name: str) -> str | None:
    """`name` from `element`, else from `parent`. Presentation attributes only."""
    if name in element.attrib:
        return element.get(name)
    return parent.get(name)


def outline_text(root: ET.Element, shaper: Shaper) -> int:
    """Replace every `<text>` in the tree with `<path>` outlines. Returns the count.

    Parents are snapshotted first: the walk replaces nodes as it goes, and a
    `<text>` inside a `<text>` would otherwise be visited twice.
    """
    ET.register_namespace("", SVG[1:-1])
    replaced = 0

    for parent in list(root.iter()):
        for index, child in enumerate(list(parent)):
            if child.tag != f"{SVG}text":
                continue

            size = float(inherited(child, parent, "font-size") or 16)
            x = float(child.get("x", "0"))
            y = float(child.get("y", "0"))
            anchor = inherited(child, parent, "text-anchor") or "start"
            weight = int(inherited(child, parent, "font-weight") or 400)

            runs = runs_of(child, weight)
            width = sum(shaper.measure(text, run_weight, size) for text, run_weight in runs)
            if anchor == "middle":
                x -= width / 2.0
            elif anchor == "end":
                x -= width

            # The presentation attributes move to a wrapping `<g>` rather than
            # onto each path, so a `url(#gradient)` fill with
            # `gradientUnits="userSpaceOnUse"` still spans the whole line
            # continuously instead of restarting per run.
            group = ET.Element(f"{SVG}g")
            for name in ("fill", "opacity", "fill-opacity", "stroke", "transform", "filter"):
                value = inherited(child, parent, name)
                if value is not None:
                    group.set(name, value)

            cursor = x
            for text, run_weight in runs:
                data = shaper.outline(text, run_weight, size, cursor, y)
                if data:
                    path = ET.SubElement(group, f"{SVG}path")
                    path.set("d", data)
                cursor += shaper.measure(text, run_weight, size)

            parent.remove(child)
            parent.insert(index, group)
            replaced += 1

    return replaced


# ---------------------------------------------------------------------------
# Rasterizing
# ---------------------------------------------------------------------------


def find_rasterizer() -> str:
    """The first SVG rasterizer installed here.

    `rsvg-convert` first because it is the reference implementation of the SVG
    1.1 that `background.svg` is written in, `resvg` second because that is what
    Homebrew installs under that name. ImageMagick is deliberately not a
    candidate: `convert` reaches SVG through whichever delegate happens to be
    linked, which is how a background ends up subtly wrong with nothing reported.
    """
    for candidate in ("rsvg-convert", "resvg"):
        if shutil.which(candidate):
            return candidate
    die(
        "no SVG rasterizer found. Install librsvg (`brew install librsvg`) or "
        "resvg (`brew install resvg`), or re-render where one is available."
    )


def rasterize(rasterizer: str, svg: pathlib.Path, png: pathlib.Path, width: int, height: int) -> None:
    if rasterizer == "rsvg-convert":
        argv = [rasterizer, "-w", str(width), "-h", str(height), "-f", "png", "-o", str(png), str(svg)]
    else:
        argv = [rasterizer, "--width", str(width), "--height", str(height), str(svg), str(png)]
    result = subprocess.run(argv, capture_output=True, text=True)
    if result.returncode != 0:
        die(f"{rasterizer} failed:\n{result.stderr.strip()}")


def pin(png: pathlib.Path, size: tuple[int, int], dpi: float) -> None:
    """Force the PNG to `size` at `dpi`, flattened and opaque.

    The resolution is not decoration. Finder derives the picture's drawn size
    from its pixel dimensions together with the `pHYs` chunk: at 72 dpi a 1440px
    picture is 1440 *points* and overflows the window, and at 144 dpi the same
    file is 720 points and lands exactly on the content view. `dmgbuild`
    compiles a HiDPI `.tiff` for exactly this reason.

    The alpha is dropped because the artwork is opaque everywhere and Finder
    would otherwise composite the picture over the window's own background,
    turning any rounding difference in a gradient into a visible seam.
    """
    from PIL import Image

    with Image.open(png) as image:
        image.load()
        if image.size != size:
            die(f"the rasterizer produced {image.size}, expected {size}")
        image.convert("RGB").save(png, format="PNG", optimize=True, dpi=(dpi, dpi))


# ---------------------------------------------------------------------------


def window_size(svg: ET.Element) -> tuple[int, int]:
    """The window's size in points, from the `.svg`'s own `width`/`height`.

    The units are checked rather than assumed: a stray `px` or `pt` in the file
    would otherwise scale the picture against a window the `.DS_Store` never
    asks for.
    """
    for name in ("width", "height"):
        value = svg.get(name)
        if value is None or value.strip().isdigit() is False:
            die(f"the <svg> element has no unitless {name}")
    return int(float(svg.get("width"))), int(float(svg.get("height")))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--svg", type=pathlib.Path, default=SOURCE, help="the editable source")
    parser.add_argument("--out", type=pathlib.Path, default=OUTPUT, help="the PNG Finder reads")
    parser.add_argument(
        "--scale", type=int, default=SCALE, help="device pixels per point (default: %(default)s)"
    )
    arguments = parser.parse_args()

    if not arguments.svg.is_file():
        die(f"{arguments.svg} is missing")

    tree = ET.parse(arguments.svg)
    width, height = window_size(tree.getroot())
    count = outline_text(tree.getroot(), Shaper(FONT))
    if count == 0:
        die(f"{arguments.svg} has no <text>; the heading is the point of it")

    rasterizer = find_rasterizer()
    arguments.out.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory() as scratch:
        outlined = pathlib.Path(scratch) / "outlined.svg"
        tree.write(outlined, encoding="utf-8", xml_declaration=True)
        rasterize(rasterizer, outlined, arguments.out, width * arguments.scale, height * arguments.scale)

    # The picture is `scale` times the window in pixels and therefore claims to
    # be `scale` times denser; Finder divides the two to get points.
    pin(arguments.out, (width * arguments.scale, height * arguments.scale), 72.0 * arguments.scale)

    relative = arguments.out.relative_to(ROOT) if arguments.out.is_relative_to(ROOT) else arguments.out
    print(f"{relative}: {width}x{height} pt at {arguments.scale}x, {count} text element(s) outlined")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())