// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Colour arithmetic, for the palettes that are not written down (FEAT-080).
 *
 * The eight built-in families are data: every value in them was chosen by hand
 * and checked by `themes.test.ts` before it shipped. A palette taken from the
 * desktop is neither — it arrives at runtime, from a generator nobody here
 * controls, and it has to be turned into fourteen readable tokens on the spot.
 * That needs the same arithmetic the test does, available at runtime.
 *
 * # Why this is not shared with `themes.test.ts`
 *
 * That file has its own `parse`, `luminance` and `contrast`, and they stay
 * there. A test that checked the palettes using the application's own contrast
 * function could not catch the contrast function being wrong — it would agree
 * with itself and pass. Two implementations that must produce the same numbers
 * is the arrangement worth having here, and `colour.test.ts` pins this one to
 * the published WCAG worked examples rather than to the other copy.
 *
 * Ratios are WCAG 2.2's: relative luminance, `(lighter + 0.05) / (darker +
 * 0.05)`, measured on the **composited** colour rather than the declared one,
 * because a translucent label is only as readable as what shows through it.
 */

export interface Rgb {
	r: number;
	g: number;
	b: number;
}

/**
 * Parse `#rgb`, `#rrggbb` or `rgb()`/`rgba()`, or return null.
 *
 * Null rather than a throw: every caller here is deriving a palette from data
 * that may be malformed, and the answer to one bad value is to fall back to
 * another candidate, not to lose the theme.
 */
export function parse(colour: string): Rgb | null {
	const text = colour.trim();

	const hex = text.match(/^#?([0-9a-f]{3}|[0-9a-f]{6})$/i);
	if (hex) {
		const digits = hex[1];
		if (digits.length === 3) {
			// `#abc` is `#aabbcc`, not `#0a0b0c`.
			const [r, g, b] = [...digits].map((digit) => parseInt(digit + digit, 16));
			return { r, g, b };
		}
		const value = parseInt(digits, 16);
		return { r: (value >> 16) & 255, g: (value >> 8) & 255, b: value & 255 };
	}

	const rgb = text.match(/^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)/i);
	if (rgb) {
		return { r: Number(rgb[1]), g: Number(rgb[2]), b: Number(rgb[3]) };
	}

	return null;
}

/** `#rrggbb`, lowercase, clamped and rounded. */
export function hex({ r, g, b }: Rgb): string {
	const channel = (value: number) =>
		Math.max(0, Math.min(255, Math.round(value)))
			.toString(16)
			.padStart(2, '0');
	return `#${channel(r)}${channel(g)}${channel(b)}`;
}

/** `rgba(r, g, b, a)` — the spelling the palettes already use for their washes. */
export function rgba({ r, g, b }: Rgb, alpha: number): string {
	const channel = (value: number) => Math.max(0, Math.min(255, Math.round(value)));
	return `rgba(${channel(r)}, ${channel(g)}, ${channel(b)}, ${Number(alpha.toFixed(3))})`;
}

/** WCAG relative luminance. */
export function luminance({ r, g, b }: Rgb): number {
	const channel = (value: number) => {
		const scaled = value / 255;
		return scaled <= 0.03928 ? scaled / 12.92 : ((scaled + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** The contrast ratio between two opaque colours, 1 to 21. */
export function contrast(front: Rgb, back: Rgb): number {
	const a = luminance(front);
	const b = luminance(back);
	const [lighter, darker] = a > b ? [a, b] : [b, a];
	return (lighter + 0.05) / (darker + 0.05);
}

/** `amount` of `top` over `bottom`, in sRGB. The `color-mix` this file needs. */
export function mix(top: Rgb, bottom: Rgb, amount: number): Rgb {
	const at = Math.max(0, Math.min(1, amount));
	return {
		r: top.r * at + bottom.r * (1 - at),
		g: top.g * at + bottom.g * (1 - at),
		b: top.b * at + bottom.b * (1 - at)
	};
}

/** Whether a colour is dark enough that light text belongs on it. */
export function isDark(colour: Rgb): boolean {
	// The midpoint of the luminance range rather than of the channel range:
	// `#808080` has a luminance of 0.216, so a channel-average threshold calls
	// mid grey light and puts white text on it.
	return luminance(colour) < 0.18;
}

/**
 * Whichever of the candidates reads best on `background`.
 *
 * Used where a palette offers more than one plausible answer and the only thing
 * that decides between them is whether the result can be read — text on a
 * filled accent, most often. Ties go to the earlier candidate, so a palette's
 * own preference wins where it makes no difference.
 */
export function mostReadable(background: Rgb, candidates: Rgb[]): Rgb {
	let best = candidates[0];
	let bestRatio = contrast(best, background);

	for (const candidate of candidates.slice(1)) {
		const ratio = contrast(candidate, background);
		if (ratio > bestRatio) {
			best = candidate;
			bestRatio = ratio;
		}
	}

	return best;
}

/**
 * The first candidate that clears `ratio`, or the most readable if none does.
 *
 * Different from [`mostReadable`] in the way that matters here: it keeps the
 * palette's **own** colour as soon as that colour is good enough, rather than
 * always reaching for the highest number. Picking the maximum sounds safer and
 * is not — on a mid-dark accent it returns pure black over the desktop's own
 * near-black ground, which clears by more and belongs to nobody's palette.
 * Contrast is a threshold to pass, not a score to win.
 */
export function firstReadable(background: Rgb, candidates: Rgb[], ratio: number): Rgb {
	for (const candidate of candidates) {
		if (contrast(candidate, background) >= ratio) return candidate;
	}
	return mostReadable(background, candidates);
}

/**
 * Walk `colour` toward `toward` until it clears `ratio` against `background`.
 *
 * This is the one function that makes a desktop palette usable rather than
 * merely faithful. Omarchy's `muted` on the machine this was written against is
 * `#5f5e63` on `#191724`: **2.75:1**, which is not readable text by any
 * standard and is the colour every secondary label in the application would
 * have taken. A literal copy of somebody's terminal palette is not a UI theme.
 *
 * It walks in twenty steps and returns the first that clears, or the fully
 * mixed colour if none does — never null, because a caller mid-repaint has to
 * have *a* colour, and the closest legible thing is better than the original.
 * Twenty is enough that the step is invisible and few enough that this stays
 * cheap on a theme change.
 *
 * The hue is preserved as far as the target allows: `toward` is normally the
 * palette's own foreground, so the result is the desktop's own text colour
 * pulled up to readable rather than a neutral grey imported from nowhere.
 */
export function untilReadable(
	colour: Rgb,
	toward: Rgb,
	background: Rgb,
	ratio: number,
	steps = 20
): Rgb {
	if (contrast(colour, background) >= ratio) return colour;

	for (let step = 1; step <= steps; step++) {
		const candidate = mix(toward, colour, step / steps);
		if (contrast(candidate, background) >= ratio) return candidate;
	}

	return toward;
}
