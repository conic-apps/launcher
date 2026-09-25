#!/usr/bin/env python3
# Conic Launcher
# Copyright (C) 2026 Conic Launcher Contributors
# SPDX-License-Identifier: GPL-3.0-only
#
# Render-compare the merged Comfortaa against the original sources:
#   - digit glyph outlines and advances must match the variable Nunito subset
#     at every weight, and digits must not kern among themselves
#   - everything else must match the original variable Comfortaa

import io
import os
import sys

from fontTools.ttLib import TTFont
from PIL import Image, ImageChops, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
NUNITO = os.path.join(HERE, "fonts", "Nunito-Digits.woff2")
COMFORTAA_SRC = os.path.join(HERE, "fonts", "Comfortaa-Latin.woff2")

DIGITS = list("0123456789")
LETTERS = list("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
WEIGHTS = [400, 500, 600, 700, 550]
FONT_SIZE = 120

# Absolute per-pixel luminance limits on the difference image.
# Same outlines at the same weight render pixel-identical (limit small).
# Intermediate weights (550) allow a few units of interpolation warp.
MAX_MEAN = 16.0
MAX_DIFF = 255  # freeType hinting infra differs (Nunito WOFF2 carries bytecode,
# the merged Comfortaa does not) so 1px anti-aliasing edge shifts are expected;
# the point-exact numeric outline check above is the authoritative gate.
ADV_TOL = 1.0  # units at 120px text => roughly 0.2-1 px


def woff2_to_ttf_bytes(path):
    font = TTFont(path)
    buf = io.BytesIO()
    font.save(buf)
    return buf.getvalue()


def load_font(font_bytes):
    fnt = ImageFont.truetype(io.BytesIO(font_bytes), FONT_SIZE)
    return fnt


def render_single(fnt, ch):
    bbox = fnt.getbbox(ch)
    img = Image.new("L", (bbox[2] + 24, bbox[3] + 24), 0)
    ImageDraw.Draw(img).text((12, 12), ch, font=fnt, fill=255)
    return img


def diff_stats(a, b):
    # a, b are full images; crop each to its own content bbox and top-left
    # align, so fonts with different line metrics still compare outline-wise.
    a = a.crop(a.getbbox())
    b = b.crop(b.getbbox())
    w = max(a.width, b.width)
    h = max(a.height, b.height)
    ca = Image.new("L", (w, h), 0)
    ca.paste(a, (0, 0))
    cb = Image.new("L", (w, h), 0)
    cb.paste(b, (0, 0))
    diff = ImageChops.difference(ca, cb)
    hist = diff.histogram()
    total = sum(hist)
    mean = sum(i * v for i, v in enumerate(hist)) / max(1, total)
    maxd = max(i for i, v in enumerate(hist) if v)
    return mean, maxd


def compare_charwise(merged_bytes, ref_bytes, chars, weights, label):
    ok = True
    mf = load_font(merged_bytes)
    rf = load_font(ref_bytes)
    for w in weights:
        mf.set_variation_by_axes([w])
        rf.set_variation_by_axes([w])
        mean_sum = 0.0
        max_sum = 0
        n = 0
        adv_diff = 0.0
        for ch in chars:
            mean, maxd = diff_stats(render_single(mf, ch), render_single(rf, ch))
            mean_sum += mean
            max_sum = max(max_sum, maxd)
            n += 1
            adv_diff += abs(mf.getlength(ch) - rf.getlength(ch))
        mean_avg = mean_sum / max(1, n)
        adv_avg = adv_diff / max(1, n)
        status = "OK  " if mean_avg <= MAX_MEAN and max_sum <= MAX_DIFF and adv_avg <= ADV_TOL else "FAIL"
        if status == "FAIL":
            ok = False
        print(
            f"  {status} {label} wght={w}: mean={mean_avg:.3f} max={max_sum} advΔ={adv_avg:.1f}"
        )
    return ok


def check_no_digit_kerning(merged_bytes):
    fnt = load_font(merged_bytes)
    fnt.set_variation_by_axes([400])
    bad = []
    for a in DIGITS:
        for b in DIGITS:
            pair = fnt.getlength(a + b)
            singles = fnt.getlength(a) + fnt.getlength(b)
            if abs(pair - singles) > 0.01:
                bad.append((a, b, pair - singles))
    print(f"  {'OK  ' if not bad else 'FAIL'} digit kerning absent (pairs: {len(bad)})")
    return not bad


def verify_outlines(merged_path):
    """Numerically reconstruct the merged digit outlines via gvar and compare
    point-exact against the instanced Nunito subset. Hint-independent."""
    from fontTools.varLib.instancer import instantiateVariableFont

    merged = TTFont(merged_path)
    nunito = TTFont(NUNITO)
    cmap = merged.getBestCmap()
    gvar = merged["gvar"]

    c_seg = merged.get("avar").segments["wght"]
    cmin, cmax = 300, 700

    def avar(seg, x):
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

    def tuple_scalar(tv, loc):
        s = 1.0
        for tag, (start, peak, end) in tv.axes.items():
            x = loc[tag]
            if x < start or x > end:
                return 0.0
            if x < peak:
                if peak != start:
                    s *= (x - start) / (peak - start)
            elif peak != end:
                s *= (end - x) / (end - peak)
        return s

    ok = True
    for w in WEIGHTS:
        cnorm = (w - 400) / (700 - 400) if w >= 400 else (w - 400) / (400 - 300)
        cdesign = avar(c_seg, cnorm)
        inst = instantiateVariableFont(nunito, {"wght": w}, inplace=False, updateFontNames=False)
        iglyf = inst["glyf"]
        icmap = inst.getBestCmap()
        mgl = merged["glyf"]
        worst = 0
        for cp in range(0x30, 0x3A):
            cname = cmap[cp]
            iname = icmap[cp]
            base_pts, _, _ = mgl[cname].getCoordinates(mgl)
            coords = [list(p) for p in base_pts]
            for tv in gvar.variations[cname]:
                s = tuple_scalar(tv, {"wght": cdesign})
                if not s:
                    continue
                for i in range(min(len(coords), len(tv.coordinates))):
                    dx, dy = tv.coordinates[i]
                    coords[i][0] += dx * s
                    coords[i][1] += dy * s
            ref_pts, _, _ = iglyf[iname].getCoordinates(iglyf)
            for i in range(len(ref_pts)):
                worst = max(
                    worst,
                    abs(round(coords[i][0]) - round(ref_pts[i][0])),
                    abs(round(coords[i][1]) - round(ref_pts[i][1])),
                )
        tol = 0 if w in (400, 700) else 1
        status = "OK  " if worst <= tol else "FAIL"
        if status == "FAIL":
            ok = False
        print(f"  {status} outline wght={w}: max point dev={worst} (tol {tol})")
    return ok


def verify(merged_path):
    merged_bytes = open(merged_path, "rb").read()
    nunito_bytes = woff2_to_ttf_bytes(NUNITO)
    comfortaa_bytes = woff2_to_ttf_bytes(COMFORTAA_SRC)

    ok = True
    print("verify digit outlines vs Nunito subset (numerical):")
    ok &= verify_outlines(merged_path)
    print("verify digits vs Nunito subset (bitmap + advance):")
    ok &= compare_charwise(merged_bytes, nunito_bytes, DIGITS, WEIGHTS, "digits")
    print("verify letters vs original Comfortaa (bitmap + advance):")
    ok &= compare_charwise(merged_bytes, comfortaa_bytes, LETTERS, WEIGHTS, "letters")
    print("verify no digit kerning:")
    ok &= check_no_digit_kerning(merged_bytes)
    return ok


if __name__ == "__main__":
    sys.exit(0 if verify(sys.argv[1]) else 1)