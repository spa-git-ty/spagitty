#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate Spagitty's application icon set from the vector mark.

The source of the identity is `assets/brand/mark.svg` (FEAT-085): a tomato
plate on a 100x100 viewBox with three cream strands that cross at the top and
run straight below it, each ending in a commit. That file is copied into the
tree verbatim and is the only geometry this module reads; nothing here
re-draws the mark, so the icon cannot drift from the drawing.

The SVG is kept to a small subset on purpose, so that this module can read it
without an SVG library and render it byte-for-byte the same everywhere:

    <rect x y width height rx fill/>                  the plate
    <path data-part="strand" d stroke stroke-width/>  a strand, round caps
    <path data-part="gap" d stroke stroke-width/>     the plate-coloured
                                                      halo that lets one
                                                      strand pass over another
    <circle cx cy r fill/>                            a commit

`d` holds M, C and L commands in absolute coordinates. Elements are drawn in
document order. For the plate-less marks (tray, menu bar) a gap erases what is
under it rather than painting the plate's colour, so the strands still cross.

Rendering is Pillow-only. Output is supersampled then downscaled with LANCZOS,
which makes regeneration byte-deterministic — the `--check` mode recomputes
every committed file in memory and diffs the bytes.

Requires Pillow (gate 2 already installs it). Everything this writes lands in
`src-tauri/icons/`.
"""

from __future__ import annotations

import io
import pathlib
import re
import sys

from PIL import Image, ImageDraw

REPO = pathlib.Path(__file__).resolve().parent.parent
ICON_DIR = REPO / "src-tauri" / "icons"
MARK = REPO / "assets" / "brand" / "mark.svg"

# Rasterising parameters. Supersample `ss` then LANCZOS-downscale; each cubic
# is subdivided into `steps` straight segments, joined round, which at the
# supersampled size is finer than a pixel of the result.
SS = 8
STEPS = 48

SVG_CACHE = {"text": None}


def svg_text() -> str:
    if SVG_CACHE["text"] is None:
        SVG_CACHE["text"] = MARK.read_text(encoding="utf-8")
    return SVG_CACHE["text"]


def view_box(svg: str) -> tuple[float, float]:
    """The viewBox's width and height."""
    m = re.search(r'viewBox="\s*[-\d.]+\s+[-\d.]+\s+([\d.]+)\s+([\d.]+)\s*"', svg)
    if not m:
        raise ValueError("mark.svg has no viewBox")
    return float(m.group(1)), float(m.group(2))


VIEW_W, VIEW_H = view_box(svg_text())


def _attrs(text: str) -> dict:
    out = {}
    for key, value in re.findall(r'([\w:-]+)="([^"]*)"', text):
        try:
            out[key] = float(value)
        except ValueError:
            out[key] = value
    return out


def parse_elements(svg: str) -> list[tuple[str, dict]]:
    """Every rect, path and circle, in document order, with its attributes."""
    return [(tag, _attrs(body)) for tag, body in
            re.findall(r'<(rect|path|circle)\s([^>]*?)/?>', svg)]


def _cubic(p0, p1, p2, p3, steps: int) -> list[tuple[float, float]]:
    out = []
    for j in range(1, steps + 1):
        t = j / steps
        u = 1 - t
        out.append((u**3 * p0[0] + 3 * u**2 * t * p1[0] + 3 * u * t**2 * p2[0] + t**3 * p3[0],
                    u**3 * p0[1] + 3 * u**2 * t * p1[1] + 3 * u * t**2 * p2[1] + t**3 * p3[1]))
    return out


def flatten(d: str, steps: int) -> list[list[tuple[float, float]]]:
    """An M/C/L path in absolute coordinates as polylines, one per subpath."""
    tokens = re.findall(r'([A-Za-z])|([-+]?\d*\.?\d+)', d)
    lines: list[list[tuple[float, float]]] = []

    def run(cmd: str, nums: list[float]) -> None:
        if cmd == "M":
            lines.append([(nums[0], nums[1])])
            for i in range(2, len(nums) - 1, 2):
                lines[-1].append((nums[i], nums[i + 1]))
        elif cmd == "L":
            for i in range(0, len(nums) - 1, 2):
                lines[-1].append((nums[i], nums[i + 1]))
        elif cmd == "C":
            for i in range(0, len(nums) - 5, 6):
                p0 = lines[-1][-1]
                lines[-1].extend(_cubic(p0, (nums[i], nums[i + 1]), (nums[i + 2], nums[i + 3]),
                                        (nums[i + 4], nums[i + 5]), steps))
        else:
            raise ValueError(f"mark.svg: unsupported path command {cmd}")

    cmd, nums = None, []
    for kind, value in tokens:
        if kind:
            if cmd:
                run(cmd, nums)
            cmd, nums = kind, []
        else:
            nums.append(float(value))
    if cmd:
        run(cmd, nums)
    return lines


def _hex(value, fallback=(0, 0, 0, 255)) -> tuple[int, int, int, int]:
    if isinstance(value, str) and re.fullmatch(r"#[0-9A-Fa-f]{6}", value):
        return (int(value[1:3], 16), int(value[3:5], 16), int(value[5:7], 16), 255)
    return fallback


CLEAR = (0, 0, 0, 0)


def plate_colour() -> tuple[int, int, int, int]:
    for tag, a in parse_elements(svg_text()):
        if tag == "rect":
            return _hex(a.get("fill"))
    raise ValueError("mark.svg has no plate")


def strand_colour() -> tuple[int, int, int, int]:
    for tag, a in parse_elements(svg_text()):
        if tag == "path" and a.get("data-part") == "strand":
            return _hex(a.get("stroke"))
    raise ValueError("mark.svg has no strand")


PLATE_COLOUR = plate_colour()
STRAND_COLOUR = strand_colour()


def _stroke(draw: ImageDraw.ImageDraw, points: list, width: float, fill) -> None:
    """A polyline with round joins and round caps.

    A disc at every vertex, not only at the ends: Pillow draws each segment of
    a wide line as its own quad, and the slivers left between two quads showed
    as hairlines across the strand after the downscale.
    """
    draw.line(points, fill=fill, width=max(1, round(width)))
    r = width / 2
    for x, y in points:
        draw.ellipse([x - r, y - r, x + r, y + r], fill=fill)


def _render_raw(px: int, strands_only: bool = False, colour: tuple = None,
                ss: int = SS, steps: int = STEPS) -> Image.Image:
    """Render the mark at px*ss, centred by xMidYMid meet on a square canvas.

    `strands_only` leaves the plate out, and a gap then erases rather than
    paints; `colour` draws every strand and commit in one tone.
    """
    canvas = max(VIEW_W, VIEW_H)
    scale = px * ss / canvas
    xoff = (canvas - VIEW_W) / 2 * scale
    yoff = (canvas - VIEW_H) / 2 * scale
    size = px * ss
    image = Image.new("RGBA", (size, size), CLEAR)
    draw = ImageDraw.Draw(image)

    def at(x: float, y: float) -> tuple[float, float]:
        return (xoff + x * scale, yoff + y * scale)

    for tag, a in parse_elements(svg_text()):
        if tag == "rect":
            if strands_only:
                continue
            x, y = at(a.get("x", 0), a.get("y", 0))
            w, h = a.get("width", 0) * scale, a.get("height", 0) * scale
            draw.rounded_rectangle([x, y, x + w, y + h], radius=a.get("rx", 0) * scale,
                                   fill=_hex(a.get("fill")))
        elif tag == "path":
            if a.get("data-part") == "gap":
                fill = CLEAR if strands_only else _hex(a.get("stroke"))
            else:
                fill = colour or _hex(a.get("stroke"))
            for line in flatten(str(a["d"]), steps):
                _stroke(draw, [at(x, y) for x, y in line], a.get("stroke-width", 1) * scale, fill)
        elif tag == "circle":
            cx, cy = at(a["cx"], a["cy"])
            r = a["r"] * scale
            draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=colour or _hex(a.get("fill")))
    return image


def render_mark(px: int, strands_only: bool = False, colour: tuple = None,
                ss: int = SS, steps: int = STEPS) -> Image.Image:
    """The mark as an RGBA image of `px` x `px` (full plate unless strands_only)."""
    raw = _render_raw(px, strands_only=strands_only, colour=colour, ss=ss, steps=steps)
    return raw.resize((px, px), Image.LANCZOS) if ss > 1 else raw


def _io(image: Image.Image, fmt: str, **kwargs) -> bytes:
    buf = io.BytesIO()
    image.save(buf, format=fmt, **kwargs)
    return buf.getvalue()


def regeneration() -> dict:
    """Every shipped icon, keyed by relative path, as PNG/ICO/ICNS bytes."""
    master = render_mark(1024)
    images = {
        "16x16.png": master.resize((16, 16), Image.LANCZOS),
        "32x32.png": master.resize((32, 32), Image.LANCZOS),
        "128x128.png": master.resize((128, 128), Image.LANCZOS),
        "128x128@2x.png": master.resize((256, 256), Image.LANCZOS),
        "256x256.png": master.resize((256, 256), Image.LANCZOS),
        "512x512.png": master.resize((512, 512), Image.LANCZOS),
        "icon.png": master.resize((512, 512), Image.LANCZOS),
    }
    out = {name: _io(img, "PNG") for name, img in images.items()}
    out["icon.ico"] = _io(images["256x256.png"], "ICO",
                          sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    try:
        out["icon.icns"] = _io(master, "ICNS")
    except Exception:  # pragma: no cover - platform/format availability
        out["icon.icns"] = None
    return out


def write_set(outdir: pathlib.Path, print_lines: bool = False) -> None:
    for name, data in regeneration().items():
        if data is None:
            if print_lines:
                print(f"  {name} skipped (this Pillow cannot write ICNS)")
            continue
        (outdir / name).write_bytes(data)
        if print_lines:
            print(f"  {name}")


def check_set(outdir: pathlib.Path) -> list:
    drift = []
    for name, data in regeneration().items():
        target = outdir / name
        if data is None:
            if not target.exists():
                drift.append(f"{name}: missing")
            continue
        if not target.exists() or target.read_bytes() != data:
            drift.append(f"{name}: differs from the committed icon")
    return drift


def main(argv) -> int:
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="verify the committed set matches")
    args = parser.parse_args(argv)

    if args.check:
        drift = check_set(ICON_DIR)
        svg = svg_text()
        if (ICON_DIR / "mark.svg").read_text(encoding="utf-8") != svg:
            drift.append("mark.svg: differs from the committed vector source")
        if drift:
            print("drift in the icon set:")
            for line in drift:
                print(f"  {line}")
            print("run `python3 tools/make-icons.py` and commit the result")
            return 1
        print("icon set matches the committed sources")
        return 0

    ICON_DIR.mkdir(parents=True, exist_ok=True)
    write_set(ICON_DIR, print_lines=True)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
