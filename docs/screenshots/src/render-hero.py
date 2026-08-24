# Re-render hero.png from hero-layout.svg.
#
# rsvg-convert refuses external file references, so this script inlines the
# theme screenshots as base64 data URIs before handing the SVG to librsvg.
#
# Usage: python3 render-hero.py   (run inside docs/screenshots/src)

import base64
import re
import subprocess
from pathlib import Path

HERE = Path(__file__).parent
OUT = HERE.parent / "hero.png"

svg = (HERE / "hero-layout.svg").read_text()

def embed(match: re.Match) -> str:
    path = HERE / match.group(1)
    data = base64.b64encode(path.read_bytes()).decode()
    return f'xlink:href="data:image/png;base64,{data}"'

svg = re.sub(r'xlink:href="(\.\./themes/[^"]+)"', embed, svg)

tmp = Path("/tmp/conic-hero-render.svg")
tmp.write_text(svg)
subprocess.run(
    ["rsvg-convert", "-w", "2400", str(tmp), "-o", str(OUT)],
    check=True,
)
print(f"rendered {OUT}")
