# Spagitty Brand & Identity Guide

> **Audience:** Human contributors, designers, and AI coding agents working on Spagitty.
> **Canonical Status:** Settled brand authority. All assets are generated deterministically from `assets/brand/mark.svg`.

---

## 1. Brand Concept & Story

Spagitty is a cross-platform, local-first desktop Git client for repositories
where people and coding agents commit side by side.

### The Words We Lead With

These are the approved forms. They exist because a product describes itself the
same way everywhere or it does not have a description at all.

| Use | Text |
| --- | --- |
| **Tagline** | Untangle the work — yours, and your agents'. |
| **Product description** | Spagitty your gateway to a Git managed agent farm. |
| **One-line descriptor** | A local-first desktop Git client for repositories where people and coding agents commit side by side. |
| **Short descriptor** | A desktop Git client built for the review that follows fast work. |
| **Category** | Desktop Git client — gateway to a Git-managed agent farm (farm in progress). |

Rules that the wording has to keep:

- **"Agents" are collaborators today, and a farm tomorrow.** Spagitty today is
  the gateway: it reads, reviews and lands work agents already commit. The
  agent farm — running and shepherding them from inside Spagitty — is still
  being planned. Do not write copy that claims the farm already ships; do not
  write copy that pretends the gateway is all there will ever be.
- **Never "AI-powered".** Nothing in Spagitty is a model. Saying otherwise
  trades the one thing a local-first tool has, which is that you can tell what
  it is doing.
- **The possessive is on the agents, not the user.** "yours, and your agents'"
  — plural possessive, apostrophe after the s. It is the most commonly
  mistyped part of the tagline.
- **The em dash is an em dash**, not a hyphen, and the tagline ends in a full
  stop. It is a sentence.
- **Keep the metaphor literal.** Tangled and straightened, pasta and strands.
  Do not extend it into cooking, chefs or kitchens in product copy — that
  register belongs to the delight layer, where it is deliberately a joke, and
  mixing the two makes the serious claims read as jokes too.

### The Name & Metaphor
The name **Spagitty** (*spa-gi-ty*) is a deliberate portmanteau of **spaghetti** and **Git**. 
A complex Git repository without clear visual lane graphs, branch divergence tracking, and structured review trails quickly becomes a messy plate of tangled pasta. Spagitty's mission is to untangle the pasta bowl into clear, straight strands of intent that developers can navigate with absolute confidence.

### Brand Tone of Voice
- **Honest & Direct:** We state what actions do without patronizing marketing speak or artificial excitement.
- **Crafted & Precise:** Geometry is deliberate, flat, and legible down to 16-pixel icons.
- **Respectful of the Machine:** Zero bloat, instant response, local-first offline operation.

---

## 2. The Mark

Three cream strands on a tomato plate. They start side by side at the top,
cross — one passes over the other two — and then run straight and parallel,
each ending in a commit. Tangled, then untangled: the name, drawn.

```
       ┌──────────────────┐
       │   \ \  /          │  <- the strands cross (the tangle)
       │    \ \/           │
       │     /\ \          │
       │    │  │  │        │  <- then run straight (the lanes)
       │    ●  ●  ●        │  <- each ends in a commit
       └──────────────────┘
          Tomato plate #CC3B2C · cream strands #FFF3E4
```

### Anatomy
1. **The plate:** a rounded square in tomato, `#CC3B2C`, corner radius about a
   quarter of its side.
2. **Three strands:** cream, `#FFF3E4`, with round caps. The crossing strand
   carries a plate-coloured halo so it reads as passing *over* the others;
   on the plate-less tray marks the halo is a cut, not a colour.
3. **Three commits:** a dot at the foot of each strand.
4. **Source:** `assets/brand/mark.svg`, a 100 × 100 viewBox, and its identical
   twin `src-tauri/icons/mark.svg`. The file uses a deliberately small subset of
   SVG — one `<rect>`, stroked `<path>`s marked `data-part="strand"` or
   `data-part="gap"`, and `<circle>`s — so `tools/make-icons.py` can render it
   without an SVG library, byte-for-byte the same on every machine.

### Why tomato and not Git's orange
Git's own logo is an orange-red, `#F05133`, and a Git client must not look like
it. The brand tomato is deeper and redder; keep it that way.

### Flat
The mark has no gradients, shadows, bevels or outlines. Depth, where the
interface wants it, comes from the glass around the mark, never from the mark.

---

## 3. Colour

### 3.1 Brand colours

| Token | Hex | Role |
| --- | --- | --- |
| Tomato | `#CC3B2C` | The plate. |
| Cream | `#FFF3E4` | The strands and commits. |
| `--brand` on light | `#B8321F` | The wordmark's "git" on light surfaces. |
| `--brand` on dark | `#F2715A` | The wordmark's "git" on dark surfaces. |
| Ink on light | `#2A1F1A` | The wordmark on light surfaces. |
| Ink on dark | `#F3E9DD` | The wordmark on dark surfaces. |

`--brand` is the brand's, not the theme's: it stays tomato whichever theme
family is chosen. `--accent` is the theme's.

### 3.2 The Pomodoro theme

The default theme family is built from the brand palette: tomato accent,
basil, saffron, aubergine and sky for the lanes, on warm cream or warm
charcoal. Its values live in `src/lib/themes.ts` and are shown, with the lane
cycle, in `assets/brand/preview.html`.

| Token | Light | Dark |
| --- | --- | --- |
| `bg` | `#FBF7F1` | `#1C1613` |
| `panel` | `#F3ECE2` | `#161110` |
| `ink` | `#2A1F1A` | `#F3E9DD` |
| `accent` | `#B8321F` | `#F2715A` |
| `danger` | `#B3124A` | `#FF5C7C` |
| `warn` | `#9A5B00` | `#F0B54A` |
| `ok` | `#2F7D45` | `#7CC68D` |
| lanes | `#C23B22 #2F8A52 #A86A00 #6C4FA3 #2F6FB0` | `#F2715A #7CC68D #F0B54A #A58BD8 #6AA7E0` |

Danger is crimson rather than red, so a destructive button is never mistaken
for the accent.

---

## 4. Typography and Wordmark

### 4.1 The wordmark
`spagitty`, lowercase, set in **Sora SemiBold (600)**, tracking `-0.02em`, with
the letters **git** in `--brand`.

- **Typeface:** Sora (SIL Open Font License 1.1), bundled at
  `assets/brand/font/Sora.ttf` with its licence in `assets/brand/font/OFL.txt`.
- In the application it is `src/lib/ui/Wordmark.svelte`, the only component
  that names Sora. The interface itself stays on the system font stack.

### 4.2 The lockup
Mark on the left, wordmark on the right, on one optical centreline. The gap is
0.42 em; the mark is 1.9 em tall.

- `lockups/lockup-ink-light.png` and `lockups/lockup.svg`: for dark surfaces.
- `lockups/lockup-ink-dark.png`: for light surfaces.

### 4.3 Clearspace
Keep a margin of at least a quarter of the mark's height clear on every side of
the mark and of the lockup.

---

## 5. Platform Asset Matrix

Every asset is generated from `mark.svg` by `tools/make-icons.py` and
`tools/make-brand.py`:

| Output | File | Size | Treatment |
| --- | --- | --- | --- |
| Master mark | `assets/brand/brand-mark.png` | 512² | Plate and strands |
| README banner | `assets/brand/hero.png` | 1600 × 400 | Mark, wordmark and tagline, inked for a dark page |
| App icon (PNG) | `src-tauri/icons/{16,32,128,256,512}x….png` | 16² to 512² | With `@2x` |
| Windows icon | `src-tauri/icons/icon.ico` | 16–256 | Multi-size |
| macOS icon | `src-tauri/icons/icon.icns` | — | Apple bundle |
| macOS menu bar | `src-tauri/icons/menubar-mono.png` | 18² (`@2x` 36²) | Alpha-only template, strands only |
| Dark tray | `src-tauri/icons/tray-white.png` | 22² (`@2x` 44²) | White strands |
| Light tray | `src-tauri/icons/tray-black.png` | 22² (`@2x` 44²) | Dark strands |
| Favicon | `assets/brand/favicon/favicon.ico` and PNGs | 16–64 | Also the in-app `BrandMark` |

---

## 6. Rules for contributors and agents

1. **Never use the Git logo or Git's orange** (`#F05133`). Spagitty is an
   independent client.
2. **Never redraw or approximate the mark.** Reference `assets/brand/mark.svg`
   or the generated images; change the mark only by changing that file and
   regenerating.
3. **Keep the brand flat.** No gradients or drop shadows on brand assets.
4. **Use the tokens.** `var(--brand)` for the name, `var(--accent)` for the
   interface. Never hard-code the tomato into a component.
5. **Regenerate, then commit.** After any change to the mark or the brand
   script, run `python3 tools/make-icons.py` and `python3 tools/make-brand.py`.
   Gate 2 in CI rejects drifted bytes.
6. **Menu bar and tray marks are strands only,** without the plate, as the
   platforms' guidelines ask.

---

## 7. Licensing and Attribution

- **Spagitty's brand artwork and code:** GPL-3.0-or-later.
- **Sora:** SIL Open Font License 1.1, © The Sora Project Authors, bundled at
  `assets/brand/font/OFL.txt`.
