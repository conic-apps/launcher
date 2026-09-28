#!/usr/bin/env python3
# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

"""Compare `ui/icons.slint` against the SVGs it was transcribed from.

`AppIcon.vue` hands the root `<svg>` `stroke: currentColor; fill: currentColor`,
so every element in a file is stroked, and an element is *filled* only when it
does not say `fill="none"`. `AppIcon` in Slint reproduces that with two `Path`
elements over the same `commands` binding: a stroked pass (`stroke-commands`,
one `stroke-width` for the whole icon) and a filled pass (`fill-commands`,
`stroke: transparent`).

So an icon is right when

    stroke-commands  ==  the `d` of every element with `fill="none"`, in order
    fill-commands    ==  the `d` of every element without it, in order

with a `<circle cx cy r>` folded into the arc pair that strokes the same disc,
and a run of whitespace between sub-paths treated as nothing (it draws the same).

Usage:  python3 slint/tools/check-icons.py [--fix] [name …]

    (no names)   every icon
    --fix        rewrite `icons.slint` so the two passes match (in place)
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ICONS = ROOT / "slint/app/ui/icons.slint"
SVG_DIR = ROOT / "src/assets/icons"

SHAPES = "path|circle|rect|line|polyline|polygon|ellipse"


def command_functions(source: str) -> dict[str, int]:
    """Where each of the two command functions starts in `source`."""
    starts = {}
    for name in ("stroke-commands", "fill-commands"):
        needle = f"function {name}(name: string) -> string {{"
        starts[name] = source.index(needle)
    return starts


def bounds(source: str, start: int) -> tuple[int, int]:
    """`(open, close)` of the braces of the function whose text starts at `start`."""
    open_at = source.index("{", start)
    depth, index = 0, open_at
    while True:
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return open_at, index
        index += 1


def body(source: str, start: int) -> str:
    """The text between the braces of the function whose text starts at `start`."""
    open_at, close_at = bounds(source, start)
    return source[open_at + 1 : close_at]


def entries(function_body: str) -> dict[str, str]:
    return {
        name: path_data.replace("\\\\", "\\")
        for name, path_data in re.findall(
            r'if name == "([^"]*)" \{ return "((?:[^"\\]|\\.)*)"; \}', function_body
        )
    }


def attribute(attrs: str, name: str) -> str | None:
    found = re.search(rf'\b{name}="([^"]*)"', attrs)
    return found.group(1) if found else None


def disc(cx: float, cy: float, r: float) -> str:
    """The arc pair that strokes to the same disc a `<circle r>` does at 2r."""
    return (
        f"M{cx:g} {cy - r:g}"
        f"a{r:g} {r:g} 0 1 0 0 {2 * r:g}"
        f"a{r:g} {r:g} 0 1 0 0 {-2 * r:g}"
    )


def svg_passes(name: str) -> tuple[str, str] | None:
    """`(stroke-commands, fill-commands)` for one SVG, as `AppIcon` sees it."""
    svg = SVG_DIR / f"{name}.svg"
    if not svg.exists():
        return None
    stroked: list[str] = []
    filled: list[str] = []
    for shape in re.finditer(rf"<({SHAPES})\b([^>]*?)/?>", svg.read_text()):
        kind, attrs = shape.group(1), shape.group(2)
        path = attribute(attrs, "d")
        if path is None and kind == "circle" and attribute(attrs, "cx"):
            path = disc(*(float(attribute(attrs, key) or 0) for key in ("cx", "cy", "r")))
        if path is None:
            continue
        (stroked if attribute(attrs, "fill") == "none" else filled).append(path)
    return "".join(stroked), "".join(filled)


def same(left: str, right: str) -> bool:
    return re.sub(r"\s+", "", left) == re.sub(r"\s+", "", right)


def replace_entry(source: str, function: str, name: str, value: str) -> str:
    """Puts `value` back into `function`'s entry for `name`."""
    start = source.index(f"function {function}(name: string) -> string {{")
    region = body(source, start)
    pattern = re.compile(
        rf'if name == "{re.escape(name)}" \{{ return "(?:[^"\\]|\\.)*"; \}}'
    )
    open_at, close_at = bounds(source, start)
    new_region, count = pattern.subn(
        lambda _: f'if name == "{name}" {{ return "{value}"; }}', region, count=1
    )
    if count == 0:
        raise SystemExit(f"{name}: no `{function}` entry to rewrite")
    return source[: open_at + 1] + new_region + source[close_at:]


def main(argv: list[str]) -> int:
    fix = "--fix" in argv
    wanted = [arg for arg in argv if not arg.startswith("--")]
    source = ICONS.read_text()
    starts = command_functions(source)
    passes = {
        "stroke-commands": entries(body(source, starts["stroke-commands"])),
        "fill-commands": entries(body(source, starts["fill-commands"])),
    }

    names = sorted(
        name
        for name in set(passes["stroke-commands"]) | set(passes["fill-commands"])
        if name in {path.stem for path in SVG_DIR.glob("*.svg")}
        and (not wanted or name in wanted)
    )

    wrong = []
    for name in names:
        wanted_passes = svg_passes(name) or ("", "")
        if all(
            same(passes[function].get(name, ""), want)
            for function, want in zip(passes, wanted_passes)
        ):
            continue
        wrong.append(name)
        if fix:
            for function, want in zip(passes, wanted_passes):
                if not same(passes[function].get(name, ""), want):
                    source = replace_entry(source, function, name, want)

    if fix:
        ICONS.write_text(source)

    for name in wrong:
        wanted_passes = svg_passes(name) or ("", "")
        print(f"{name}:")
        for function, want in zip(passes, wanted_passes):
            have = passes[function].get(name, "")
            if same(have, want):
                continue
            print(f"    {function}")
            print(f"        have {have}")
            print(f"        want {want}")

    checked = len(names)
    if not wrong:
        print(f"{checked} icons match their SVG")
    else:
        print(f"{len(wrong)} of {checked} icons differ" + (" (rewritten)" if fix else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
