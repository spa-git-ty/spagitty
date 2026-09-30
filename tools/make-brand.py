#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate Spagitty's brand collateral from the one source mark.

Every asset here is derived from the mark (`assets/brand/mark.svg`) through
`tools/make-icons.py` — the lockup is not a separate drawing but the same
plate-and-strands mark composed with the wordmark, so the identity cannot drift
into a second version of the icon.

    assets/brand/
      brand-mark.png         the mark (tomato plate, cream strands) on transparency
      lockups/               wordmark lockups for dark and light surfaces (PNG + SVG)
      favicon/               favicon.ico + 16/32/64 PNGs
      hero.png               the README banner
      preview.html           the sweep page (open in a browser, per Amendment 4)
      font/Sora.ttf          the wordmark typeface (SIL OFL 1.1), committed so
                             generation is hermetic — no network fetch
    src-tauri/icons/
      menubar-mono.*         macOS menu bar template mark (alpha-only, 18/36)
      tray-black.*           monochrome mark for bright trays (22/44)
      tray-white.*           monochrome mark for dark trays (22/44)

The tray and menu bar marks are the strands alone — no plate — in one tone
that adapts to the tray's colour: a tomato plate would not read at 18–22 px on
a system slot.

The wordmark is "spagitty" in Sora SemiBold, with "git" in the brand's tomato
(FEAT-085). The tomato is a shade lighter on dark surfaces and a shade darker
on light ones, the same pair the Pomodoro theme uses as its accent.

Regenerate with:

    python3 tools/make-brand.py          # writes the collateral
    python3 tools/make-brand.py --check  # verifies the committed files match

Requires Pillow. The UI itself runs on the system font stack, so the brand
font and the application font are independent.
"""

from __future__ import annotations

import argparse
import importlib.util
import io
import pathlib
import re
import sys

from PIL import Image, ImageDraw, ImageFont

REPO = pathlib.Path(__file__).resolve().parent.parent
BRAND = REPO / "assets" / "brand"
ICONS = REPO / "src-tauri" / "icons"
FONT = BRAND / "font" / "Sora.ttf"

# The shared mark source lives in make-icons.py (the hyphen makes it unimportable
# as a module, so load it by path — this is the single renderer of the geometry).
_SOURCE = importlib.util.spec_from_file_location(
    "make_icons", REPO / "tools" / "make-icons.py")
make_icons = importlib.util.module_from_spec(_SOURCE)
_SOURCE.loader.exec_module(make_icons)

render_mark = make_icons.render_mark
VIEW_W, VIEW_H = make_icons.VIEW_W, make_icons.VIEW_H

WORDMARK = "spagitty"
# The letters drawn in the brand colour: "git", the part of the name that is
# the thing itself.
GIT = range(3, 6)

# Inks. `*_LIGHT` sits on dark surfaces, `*_DARK` on light ones. These are the
# Pomodoro theme's own ink and accent (src/lib/themes.ts), so a lockup on a
# screen matches the screen.
INK_LIGHT = (243, 233, 221, 255)     # #f3e9dd
INK_DARK = (42, 31, 26, 255)         # #2a1f1a
TOMATO_LIGHT = (242, 113, 90, 255)   # #f2715a, on dark
TOMATO_DARK = (184, 50, 31, 255)     # #b8321f, on light
MUTED_LIGHT = (166, 151, 138, 255)   # #a6978a, secondary text on dark

TRACKING = -0.02  # of the em; Sora is set a touch tight at display sizes
WEIGHT = 600      # SemiBold


# --- Typography -------------------------------------------------------------

def load_wordmark_font(em: float, weight: int = WEIGHT) -> ImageFont.FreeTypeFont:
    font = ImageFont.truetype(str(FONT), size=round(em))
    try:
        axes = font.get_variation_axes()
    except Exception:
        return font
    values = []
    for axis in axes:
        name = axis.get("name") if isinstance(axis, dict) else None
        lo, hi = axis["minimum"], axis["maximum"]
        if name in (b"Weight", "Weight") or len(axes) == 1:
            values.append(max(lo, min(hi, weight)))
        else:
            values.append(axis.get("default", lo))
    try:
        font.set_variation_by_axes(values)
    except Exception:
        pass
    return font


def wordmark_width(font: ImageFont.FreeTypeFont, text: str, tracking: float) -> float:
    return sum(font.getlength(c) for c in text) + tracking * (len(text) - 1)


def wordmark_baseline(font: ImageFont.FreeTypeFont, text: str, center_y: float) -> float:
    """The baseline that puts the ink's optical centre on `center_y`.

    Pillow's default text origin is the top-left of the em box, not the
    baseline — treating a centreline as a baseline dropped the wordmark a full
    ascender below the mark. Anchor at the baseline and centre on the ink.
    """
    probe = ImageDraw.Draw(Image.new("RGBA", (1, 1)))
    _l, top, _r, bottom = probe.textbbox((0, 0), text, font=font, anchor="ls")
    return center_y - (top + bottom) / 2


def draw_wordmark(image: Image.Image, font: ImageFont.FreeTypeFont, text: str,
                  start_x: float, center_y: float, tracking: float, fill, accent=None) -> float:
    """Set `text`, with the GIT letters in `accent` when one is given."""
    draw = ImageDraw.Draw(image)
    baseline = wordmark_baseline(font, text, center_y)
    x = start_x
    for i, c in enumerate(text):
        colour = accent if accent is not None and i in GIT else fill
        draw.text((round(x), round(baseline)), c, font=font, fill=colour, anchor="ls")
        x += font.getlength(c) + tracking
    return x


def hexc(colour: tuple) -> str:
    return "#{:02x}{:02x}{:02x}".format(*(c for c in colour[:3]))


def png(image: Image.Image) -> bytes:
    buf = io.BytesIO()
    image.save(buf, format="PNG")
    return buf.getvalue()


# --- Lockup --------------------------------------------------------------

def _lockup_metrics(em: float) -> dict:
    font = load_wordmark_font(em)
    tracking = em * TRACKING
    return {
        "font": font,
        "tracking": tracking,
        "tw": wordmark_width(font, WORDMARK, tracking),
        "mark": round(em * 1.9),
        "gap": round(em * 0.42),
        "margin": round(em * 0.5),
    }


def compose_lockup(ink: tuple, accent: tuple, ss: int = 2) -> Image.Image:
    """Mark and wordmark on one centreline, on transparency."""
    m = _lockup_metrics(110 * ss)
    width = round(m["margin"] + m["mark"] + m["gap"] + m["tw"] + m["margin"])
    height = round(m["mark"] + 2 * m["margin"])
    canvas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    canvas.alpha_composite(render_mark(m["mark"]), (m["margin"], m["margin"]))
    center_y = m["margin"] + m["mark"] / 2
    draw_wordmark(canvas, m["font"], WORDMARK, m["margin"] + m["mark"] + m["gap"],
                  center_y, m["tracking"], ink, accent)
    return canvas.resize((round(width / ss), round(height / ss)), Image.LANCZOS)


def lockup_svg_bytes(ink: tuple, accent: tuple) -> bytes:
    """An SVG twin of the PNG lockup: the vector mark inline plus <text>."""
    m = _lockup_metrics(110)
    total_w = m["margin"] + m["mark"] + m["gap"] + m["tw"] + m["margin"]
    total_h = m["mark"] + 2 * m["margin"]
    probe = ImageDraw.Draw(Image.new("RGBA", (1, 1)))
    _l, top, _r, bottom = probe.textbbox((0, 0), WORDMARK, font=m["font"], anchor="ls")
    text_y = m["margin"] + m["mark"] / 2 - (top + bottom) / 2

    mark_svg = (BRAND / "mark.svg").read_text(encoding="utf-8")
    inner = re.sub(r'<svg[^>]*>', '', mark_svg, count=1)
    inner = re.sub(r'</svg>\s*$', '', inner).strip()
    inner = "\n".join("  " + line.strip() for line in inner.splitlines())
    head, git, tail = WORDMARK[:GIT.start], WORDMARK[GIT.start:GIT.stop], WORDMARK[GIT.stop:]

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {round(total_w)} {round(total_h)}">',
        f'<g transform="translate({m["margin"]} {m["margin"]}) scale({m["mark"] / VIEW_W:.4f})">',
        inner,
        '</g>',
        f'<text x="{m["margin"] + m["mark"] + m["gap"]}" y="{text_y:.1f}" font-size="110" '
        f'font-family="Sora, system-ui, sans-serif" font-weight="{WEIGHT}" '
        f'letter-spacing="{m["tracking"]:.1f}" fill="{hexc(ink)}">'
        f'{head}<tspan fill="{hexc(accent)}">{git}</tspan>{tail}</text>',
        '</svg>',
    ]
    return ("\n".join(parts) + "\n").encode("utf-8")


# --- Hero ---------------------------------------------------------------------

def hero_banner(ss: int = 2) -> Image.Image:
    """README banner: mark and wordmark on one centreline, the tagline under.

    Transparent, and inked for a dark page, which is how GitHub shows the
    README to most of the people who read it.
    """
    w, h = 1600, 400
    base = Image.new("RGBA", (w * ss, h * ss), (0, 0, 0, 0))
    draw = ImageDraw.Draw(base)

    mark_px = 240 * ss
    mark_x = 80 * ss
    mark_y = round((h * ss - mark_px) / 2)
    base.alpha_composite(render_mark(mark_px, ss=ss), (mark_x, mark_y))

    em = 120 * ss
    font = load_wordmark_font(em)
    tracking = em * TRACKING
    text_x = mark_x + mark_px + round(em * 0.42)
    center_y = mark_y + mark_px / 2 - 18 * ss
    draw_wordmark(base, font, WORDMARK, text_x, center_y, tracking, INK_LIGHT, TOMATO_LIGHT)

    tagline = "Untangle the work — yours, and your agents'."
    tag_font = load_wordmark_font(30 * ss, weight=400)
    _l, top, _r, bottom = draw.textbbox((0, 0), WORDMARK, font=font, anchor="ls")
    word_bottom = center_y - (top + bottom) / 2 + bottom
    draw.text((round(text_x + 4 * ss), round(word_bottom + 26 * ss)), tagline, font=tag_font,
              fill=MUTED_LIGHT, anchor="lt")
    return base.resize((w, h), Image.LANCZOS)


# --- The sweep page ---------------------------------------------------------

# The Pomodoro theme's tokens, as src/lib/themes.ts defines them; the page
# shows them beside the mark so the two are checked together.
THEME = {
    "dark": [("bg", "#1c1613"), ("panel", "#161110"), ("ink", "#f3e9dd"), ("accent", "#f2715a"),
             ("danger", "#ff5c7c"), ("warn", "#f0b54a"), ("ok", "#7cc68d")],
    "light": [("bg", "#fbf7f1"), ("panel", "#f3ece2"), ("ink", "#2a1f1a"), ("accent", "#b8321f"),
              ("danger", "#b3124a"), ("warn", "#9a5b00"), ("ok", "#2f7d45")],
}
LANES = {
    "dark": ["#f2715a", "#7cc68d", "#f0b54a", "#a58bd8", "#6aa7e0"],
    "light": ["#c23b22", "#2f8a52", "#a86a00", "#6c4fa3", "#2f6fb0"],
}


def _swatches(rows: list) -> str:
    return "".join(
        f'<div class="swatch"><span style="background:{hexv}"></span>'
        f"<code>{name}</code><code>{hexv}</code></div>"
        for name, hexv in rows)


def _lanes(colours: list) -> str:
    return "".join(f'<span class="lane" style="background:{c}"></span>' for c in colours)


def brand_preview_html() -> str:
    sizes = "".join(
        f'<figure><img src="../../src-tauri/icons/{name}" width="{px}" height="{px}" alt="">'
        f"<figcaption>{px}</figcaption></figure>"
        for name, px in (("512x512.png", 256), ("128x128.png", 128), ("32x32.png", 64),
                         ("32x32.png", 32), ("16x16.png", 16)))
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Spagitty brand</title>
<style>
@font-face {{ font-family: 'Sora'; src: url('font/Sora.ttf') format('truetype'); font-weight: 100 800; }}
:root {{ color-scheme: light dark; --bg: #fbf7f1; --panel: #f3ece2; --ink: #2a1f1a; --muted: #6e5f54; --line: rgba(42,31,26,.14); }}
@media (prefers-color-scheme: dark) {{
  :root {{ --bg: #1c1613; --panel: #161110; --ink: #f3e9dd; --muted: #a6978a; --line: rgba(243,233,221,.14); }}
}}
* {{ box-sizing: border-box; }}
body {{ margin: 0; background: var(--bg); color: var(--ink); font: 15px/1.5 system-ui, sans-serif; }}
main {{ max-width: 1080px; margin: 0 auto; padding: 48px 20px 80px; display: grid; gap: 48px; }}
h1, h2 {{ font-family: Sora, system-ui, sans-serif; font-weight: 600; letter-spacing: -0.02em; margin: 0 0 12px; }}
h1 {{ font-size: 44px; }} h2 {{ font-size: 22px; }}
p {{ color: var(--muted); margin: 0; max-width: 64ch; }}
.row {{ display: flex; flex-wrap: wrap; gap: 24px; align-items: flex-end; }}
figure {{ margin: 0; display: grid; gap: 6px; justify-items: center; }}
figcaption, code {{ font: 12px ui-monospace, monospace; color: var(--muted); }}
.panel {{ background: var(--panel); border: 1px solid var(--line); border-radius: 20px; padding: 24px; }}
.dark {{ background: #1c1613; }} .light {{ background: #fbf7f1; }}
.lockup {{ max-width: 100%; height: 96px; }}
.swatches {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 12px; }}
.swatch {{ display: grid; gap: 4px; }}
.swatch span {{ height: 48px; border-radius: 12px; border: 1px solid var(--line); }}
.lanes {{ display: flex; gap: 8px; margin-top: 12px; }}
.lane {{ flex: 1; height: 20px; border-radius: 10px; }}
.tray {{ display: flex; gap: 16px; align-items: center; padding: 12px 16px; border-radius: 12px; }}
</style>
</head>
<body>
<main>
<header>
<h1>spa<span style="color:#b8321f">git</span>ty</h1>
<p>The brand at a glance, generated by <code>tools/make-brand.py</code> from
<code>assets/brand/mark.svg</code>. Every image on this page is a committed file.</p>
</header>

<section>
<h2>The mark</h2>
<p>Three strands that cross and then run straight, each ending in a commit, on a tomato plate.</p>
<div class="row" style="margin-top:20px">{sizes}</div>
</section>

<section>
<h2>Lockups</h2>
<div class="row" style="margin-top:12px">
<div class="panel light"><img class="lockup" src="lockups/lockup-ink-dark.png" alt="Spagitty lockup for light surfaces"></div>
<div class="panel dark"><img class="lockup" src="lockups/lockup-ink-light.png" alt="Spagitty lockup for dark surfaces"></div>
</div>
</section>

<section>
<h2>Pomodoro — dark</h2>
<div class="swatches">{_swatches(THEME["dark"])}</div>
<div class="lanes">{_lanes(LANES["dark"])}</div>
</section>

<section>
<h2>Pomodoro — light</h2>
<div class="swatches">{_swatches(THEME["light"])}</div>
<div class="lanes">{_lanes(LANES["light"])}</div>
</section>

<section>
<h2>Tray and menu bar</h2>
<div class="row" style="margin-top:12px">
<div class="tray dark"><img src="../../src-tauri/icons/tray-white@2x.png" width="22" height="22" alt=""><img src="../../src-tauri/icons/tray-white.png" width="22" height="22" alt=""></div>
<div class="tray light"><img src="../../src-tauri/icons/tray-black@2x.png" width="22" height="22" alt=""><img src="../../src-tauri/icons/menubar-mono.png" width="18" height="18" alt=""></div>
</div>
</section>

<section>
<h2>Favicon</h2>
<div class="row" style="margin-top:12px">
<img src="favicon/favicon-64.png" width="64" height="64" alt="">
<img src="favicon/favicon-32.png" width="32" height="32" alt="">
<img src="favicon/favicon-16.png" width="16" height="16" alt="">
</div>
</section>
</main>
</body>
</html>
"""


def build_all() -> dict:
    out = {}

    marque = render_mark(512)
    out["assets/brand/brand-mark.png"] = png(marque)
    out["assets/brand/lockups/lockup-ink-light.png"] = png(compose_lockup(INK_LIGHT, TOMATO_LIGHT))
    out["assets/brand/lockups/lockup-ink-dark.png"] = png(compose_lockup(INK_DARK, TOMATO_DARK))
    out["assets/brand/lockups/lockup.svg"] = lockup_svg_bytes(INK_LIGHT, TOMATO_LIGHT)
    out["assets/brand/favicon/favicon-16.png"] = png(render_mark(16))
    out["assets/brand/favicon/favicon-32.png"] = png(render_mark(32))
    out["assets/brand/favicon/favicon-64.png"] = png(render_mark(64))
    ico = io.BytesIO()
    render_mark(256).save(ico, format="ICO", sizes=[(16, 16), (32, 32), (48, 48), (64, 64)])
    out["assets/brand/favicon/favicon.ico"] = ico.getvalue()
    out["assets/brand/hero.png"] = png(hero_banner())
    out["assets/brand/preview.html"] = brand_preview_html().encode("utf-8")

    mono = render_mark(36, strands_only=True, colour=(0, 0, 0, 255))
    out["src-tauri/icons/menubar-mono@2x.png"] = png(mono)
    out["src-tauri/icons/menubar-mono.png"] = png(render_mark(18, strands_only=True, colour=(0, 0, 0, 255)))
    white = render_mark(44, strands_only=True, colour=(255, 255, 255, 255))
    out["src-tauri/icons/tray-white@2x.png"] = png(white)
    out["src-tauri/icons/tray-white.png"] = png(render_mark(22, strands_only=True, colour=(255, 255, 255, 255)))
    black = render_mark(44, strands_only=True, colour=(20, 22, 28, 255))
    out["src-tauri/icons/tray-black@2x.png"] = png(black)
    out["src-tauri/icons/tray-black.png"] = png(render_mark(22, strands_only=True, colour=(20, 22, 28, 255)))
    return out


def main(argv) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    output = build_all()
    if args.check:
        drift = []
        for rel, data in output.items():
            target = REPO / rel
            if not target.exists() or target.read_bytes() != data:
                drift.append(f"{rel}: differs from the committed file")
        if drift:
            print("drift in the brand collateral:")
            for line in drift:
                print(f"  {line}")
            print("run `python3 tools/make-brand.py` and commit the result")
            return 1
        print("brand collateral matches the committed sources")
        return 0

    for rel, data in output.items():
        target = REPO / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        print(f"  {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
