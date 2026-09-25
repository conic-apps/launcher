#!/usr/bin/env python3
# Conic Launcher
# Copyright (C) 2026 Conic Launcher Contributors
# SPDX-License-Identifier: GPL-3.0-only
#
# Merge the Nunito digit glyphs into the variable Comfortaa font and save the
# result as "Comfortaa Nunito", the TTF the Slint frontend embeds.
#
# The Vue original rendered digits with a digits-only Nunito subset selected
# per character via CSS `unicode-range: U+0030-0039`, on top of Comfortaa for
# everything else. Slint has no unicode-range fallback, so this script bakes
# the Nunito digit outlines into Comfortaa itself: a single family whose
# '0'..'9' glyphs are Nunito's while every other glyph stays Comfortaa's.
#
# The digit outlines get their own gvar deltas along the 'wght' axis so they
# interpolate like the original variable fonts. Nunito advances for the digits
# are constant (600) across all weights, so no HVAR extension is needed; the
# stale Comfortaa digit HVAR rows are zeroed instead.
#
# Inputs:
#   slint/tools/fonts/Comfortaa-Latin.woff2 (variable, wght 300-700)
#   slint/tools/fonts/Nunito-Digits.woff2   (variable, wght 200-1000, digits only)
# Output:
#   slint/app/ui/fonts/ComfortaaNunito.ttf  (family "Comfortaa Nunito")
#
# Usage: python3 merge-digit-font.py [--verify]

import argparse
import copy
import math
import os
import sys
import tempfile

import numpy as np

from fontTools.ttLib import TTFont
from fontTools.ttLib.tables.TupleVariation import TupleVariation
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.varLib.models import VariationModel

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
MERGE_OUT = os.path.join(ROOT, "slint", "app", "ui", "fonts", "ComfortaaNunito.ttf")
COMFORTAA = os.path.join(HERE, "fonts", "Comfortaa-Latin.woff2")
NUNITO = os.path.join(HERE, "fonts", "Nunito-Digits.woff2")

MERGE_FAMILY = "Comfortaa Nunito"

DIGIT_CODEPOINTS = [0x30 + i for i in range(10)]

# 'wght' axis range of each font (user space, per its own fvar).
COMFORTAA_MIN, COMFORTAA_DEFAULT, COMFORTAA_MAX = 300, 400, 700
NUNITO_MIN, NUNITO_DEFAULT, NUNITO_MAX = 200, 400, 1000

# Sample weights sampled on the merged comfortaa axis. Knots land on Nunito's
# own masters (300/500/600/700) so those weights reproduce Nunito exactly.
SAMPLE_WEIGHTS = [300, 350, 400, 450, 500, 550, 600, 650, 700]


def avar_segments(font):
    avar = font.get("avar")
    if avar is None:
        return {}
    return avar.segments


def avar_lookup(seg, x):
    """Piecewise-linear lookup over an avar segment dict (mapping input->output)."""
    if not seg:
        return x
    keys = sorted(seg)
    if x <= keys[0]:
        return seg[keys[0]]
    if x >= keys[-1]:
        return seg[keys[-1]]
    for lo, hi in zip(keys, keys[1:]):
        if x <= hi:
            t = (x - lo) / (hi - lo)
            return seg[lo] + (seg[hi] - seg[lo]) * t
    return seg[keys[-1]]


def user_to_design(font, axis, value, default, min_v, max_v, segments):
    """Map a user-space axis value to normalized design space (post-avar).

    The axis is asymmetric here (default 400, min 300, max 700), so the
    positive and negative half-ranges differ, matching how engines normalize
    non-centered axes: (value-default)/(max-default) above the default and
    (value-default)/(default-min) below it."""
    norm = (value - default) / (max_v - default) if value >= default else (value - default) / (default - min_v)
    return avar_lookup(segments.get(axis, {}), norm)


def f2dot14(v):
    return round(v * 16384) / 16384


def instance_nunito(font, wght):
    """Return the digits-only font instanced (renamed) at a user weight."""
    return instantiateVariableFont(
        font, {"wght": wght}, inplace=False, updateFontNames=False
    )


def coords_of(inst, cp):
    glyf = inst["glyf"]
    name = inst.getBestCmap()[cp]
    g = glyf[name]
    pts, _ends, _flags = g.getCoordinates(glyf)
    return [(x, y) for x, y in pts]


def zero_value_record(value):
    if value is None:
        return
    for attr in ("XPlacement", "YPlacement", "XAdvance", "YAdvance"):
        if hasattr(value, attr):
            setattr(value, attr, 0)


def materialize_pairpos(slot, glyph_order):
    """Yield (first, second, xadv) tuples for every nonzero kern in a PairPos
    subtable (Format 1 or 2)."""
    digits_dummy = set()
    if slot.Format == 1:
        cov = slot.Coverage.glyphs
        for gi, first in enumerate(cov):
            for rec in slot.PairSet[gi].PairValueRecord:
                x = rec.Value1.XAdvance if rec.Value1 else 0
                if x:
                    yield (first, rec.SecondGlyph, x)
    elif slot.Format == 2:
        inv1, inv2 = {}, {}
        for g, c in slot.ClassDef1.classDefs.items():
            inv1.setdefault(c, []).append(g)
        for g, c in slot.ClassDef2.classDefs.items():
            inv2.setdefault(c, []).append(g)
        classed1 = set(slot.ClassDef1.classDefs)
        classed2 = set(slot.ClassDef2.classDefs)
        firsts = {
            i: inv1.get(i, []) if i else [g for g in glyph_order if g not in classed1]
            for i in range(slot.Class1Count)
        }
        seconds = {
            i: inv2.get(i, []) if i else [g for g in glyph_order if g not in classed2]
            for i in range(slot.Class2Count)
        }
        for i1 in range(slot.Class1Count):
            for i2 in range(slot.Class2Count):
                vr = slot.Class1Record[i1].Class2Record[i2]
                if vr is None or vr.Value1 is None:
                    continue
                x = vr.Value1.XAdvance or 0
                if not x:
                    continue
                for f in firsts[i1]:
                    for s in seconds[i2]:
                        yield (f, s, x)


def strip_digit_kerning(font):
    """Rebuild every PairPos lookup as an explicit Format 1 table without the
    pairs whose first or second glyph is a digit.

    The original app rendered digits from the Nunito subset, which has no
    kerning, so the merged font must not kern around the Nunito digits either.
    Non-digit kerning (D-period, V-V, ...) is preserved."""
    from copy import deepcopy
    from fontTools.ttLib.tables import otTables as ot

    digits = {"zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"}
    gpos = font.get("GPOS")
    if gpos is None:
        return
    glyph_order = font.getGlyphOrder()
    for lookup in gpos.table.LookupList.Lookup:
        if lookup.LookupType != 2:
            continue
        pairs = []
        for slot in lookup.SubTable:
            pairs.extend(materialize_pairpos(slot, glyph_order))
        kept = [(f, s, x) for f, s, x in pairs if f not in digits and s not in digits]

        if kept:
            firsts = sorted({f for f, _, _ in kept}, key=lambda g: glyph_order.index(g))
            by_first = {}
            for f, s, x in kept:
                by_first.setdefault(f, []).append((s, x))

            template = lookup.SubTable[0]
            new_slot = deepcopy(template)
            cov = new_slot.Coverage
            cov.glyphs = firsts

            pair_sets = []
            for f in firsts:
                ps = ot.PairSet()
                records = []
                for s, x in by_first[f]:
                    rec = ot.PairValueRecord()
                    rec.SecondGlyph = s
                    vr = ot.ValueRecord()
                    vr.XAdvance = x
                    rec.Value1 = vr
                    rec.Value2 = None
                    records.append(rec)
                ps.PairValueRecord = records
                pair_sets.append(ps)
            new_slot.PairSet = pair_sets
            new_slot.ValueFormat1 = 4
            new_slot.ValueFormat2 = 0
            lookup.SubTable = [new_slot]
        else:
            lookup.SubTable = []


def retag_font(font, family):
    """Regive the merged font its own family identity instead of masquerading
    as "Comfortaa". Only the identifying name records change; version/license/
    copyright records are inherited from the sources."""
    ps = family.replace(" ", "")
    replace = {
        1: family,
        2: "Regular",
        3: "3.105;CONIC;ComfortaaNunito-Regular",
        4: f"{family} Regular",
        6: f"{ps}-Regular",
        16: family,
        17: "Regular",
    }
    # fvar instance post-script names: Comfortaa-Light, -Regular, ... become
    # ComfortaaNunito-...
    instance_ps = {
        262: f"{ps}-Light",
        263: f"{ps}-Regular",
        264: f"{ps}-Medium",
        265: f"{ps}-SemiBold",
        266: f"{ps}-Bold",
    }
    for rec in font["name"].names:
        if rec.nameID in instance_ps:
            rec.string = instance_ps[rec.nameID]
    name = font["name"]
    name.names = [r for r in name.names if r.nameID not in replace]
    platforms = sorted({(r.platformID, r.platEncID, r.langID) for r in name.names})
    if not platforms:
        platforms = [(3, 1, 0x409)]
    for plat, enc, lang in platforms:
        for rid, val in replace.items():
            name.setName(val, rid, plat, enc, lang)


def update_head_bbox(font):
    glyf = font["glyf"]
    xs_max = ys_max = -sys.maxsize
    xs_min = ys_min = sys.maxsize
    for name in font.getGlyphOrder():
        g = glyf[name]
        if g.numberOfContours == 0:
            continue
        if g.numberOfContours < 0:
            g.recalcBounds(glyf)
        xs_max = max(xs_max, g.xMax)
        xs_min = min(xs_min, g.xMin)
        ys_max = max(ys_max, g.yMax)
        ys_min = min(ys_min, g.yMin)
    head = font["head"]
    head.xMin, head.yMin = xs_min, ys_min
    head.xMax, head.yMax = xs_max, ys_max


def build_merged():
    comfortaa = TTFont(COMFORTAA)
    nunito = TTFont(NUNITO)

    c_seg = avar_segments(comfortaa).get("wght", {})

    # design-space (post-avar, normalized) coordinate of each sample weight on
    # the merged comfortaa axis.
    knot_map = {}
    for w in SAMPLE_WEIGHTS:
        knot_map[w] = user_to_design(
            comfortaa, "wght", w, COMFORTAA_DEFAULT, COMFORTAA_MIN, COMFORTAA_MAX, {"wght": c_seg}
        )

# Nunito is instanced at each sample *user* weight (instancer applies its
    # own avar internally), once per weight and reused across glyphs.
    insts = {w: instance_nunito(nunito, w) for w in SAMPLE_WEIGHTS}
    base_inst = insts[NUNITO_DEFAULT]

    # Coordinates at every master knot; per-glyph VariationModel over the knots
    # yields the exact tuple deltas + supports for our gvar.
    model = VariationModel([{"wght": knot_map[w]} for w in SAMPLE_WEIGHTS])

    gvar = comfortaa["gvar"]
    glyf = comfortaa["glyf"]
    hmtx = comfortaa["hmtx"]
    awm = comfortaa["HVAR"].table.AdvWidthMap.mapping
    cmap = comfortaa.getBestCmap()

    for cp in DIGIT_CODEPOINTS:
        cname = cmap[cp]
        nname = base_inst.getBestCmap()[cp]
        adv, lsb = base_inst["hmtx"][nname]
        base = coords_of(base_inst, cp)

        # gvar tuples must cover real points + 4 phantom deltas. Nunito digit
        # advances are constant so the phantom deltas stay 0; the base phantom
        # positions are derived from the hmtx we set below.
        master_all = {k: coords_of(insts[k], cp) for k in SAMPLE_WEIGHTS}
        values = np.array(
            [np.array(master_all[w] + [(0, 0)] * 4, dtype=float) for w in SAMPLE_WEIGHTS]
        )
        base_len = len(base) + 4

        # per-glyph VariationModel -> (delta, support) per master.
        # getDeltasAndSupports only takes scalar values, so drive the vector
        # deltas through getDeltas and take the aligned supports from a
        # scalar call (both follow the model's internal ordering).
        gdeltas = model.getDeltas(values)
        _, gsupports = model.getDeltasAndSupports([1] * len(values))

        tuples = []
        for dl, sup in zip(gdeltas, gsupports):
            if not sup:
                continue  # default master
            reg = sup["wght"]
            reg = tuple(f2dot14(c) for c in reg)
            delta = [tuple(int(round(c)) for c in dl[i]) for i in range(base_len)]
            tuples.append(
                TupleVariation(
                    axes={"wght": reg},
                    coordinates=delta,
                )
            )
        gvar.variations[cname] = tuples

        # Replace the glyph with the instanced (weight-400) Nunito outline.
        glyf[cname] = copy.deepcopy(base_inst["glyf"][nname])
        hmtx[cname] = (adv, lsb)
        awm[cname] = 0  # disable Comfortaa's own digit advance variation

    # Recompute derived metrics.
    update_head_bbox(comfortaa)
    strip_digit_kerning(comfortaa)
    retag_font(comfortaa, MERGE_FAMILY)
    comfortaa["hhea"].recalc(comfortaa)
    comfortaa["OS/2"].recalcAvgCharWidth(comfortaa)

    return comfortaa


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify", action="store_true", help="render-compare against sources")
    args = parser.parse_args()

    merged = build_merged()
    merged.save(MERGE_OUT)
    print(f"wrote {MERGE_OUT}")

    if args.verify:
        from verify_merge import verify

        sys.exit(0 if verify(MERGE_OUT) else 1)


if __name__ == "__main__":
    main()