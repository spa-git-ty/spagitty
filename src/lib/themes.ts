// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Spagitty's palette.
 *
 * One family, Pomodoro, in a light and a dark variant (FEAT-086). There were
 * nine — eight published palettes beside Spagitty's own — and none of the
 * eight was drawn for the spatial shell or shared a colour with the mark, so
 * together they read as nine applications (TASK-051). What is left is the one
 * built from the brand.
 *
 * The palette is **data**, not a stylesheet: `theme.svelte.ts` applies it to
 * the root element as custom properties — the same mechanism `panels.svelte.ts`
 * and `metrics.ts` use for the structural tokens. The keys are the token names
 * in `src/app.css` with the `--` dropped. Nothing else in the application has
 * to learn a new name, and no component carries a colour of its own.
 *
 * The family shape stays, with one entry, so the theme store and following the
 * desktop's own palette (Omarchy) keep their one way of asking for colours.
 */

export type Mode = 'light' | 'dark';

export type FamilyId = 'pomodoro';

/**
 * One complete set of colour tokens.
 *
 * Every field is required. A palette that leaves one out would fall through to
 * whatever the previous theme had set, which is how a half-applied theme
 * happens — and it is exactly the kind of thing nobody notices until a screen
 * they rarely open looks wrong.
 */
export interface Palette {
	bg: string;
	panel: string;
	ink: string;
	muted: string;
	line: string;
	soft: string;
	placeholder: string;
	accent: string;
	/** Text on top of a filled accent surface. Contrast-checked against it. */
	onAccent: string;
	/**
	 * What a result means, in this family's own colours.
	 *
	 * Before these existed, anything that had to read as "not routine" borrowed
	 * `lanes[2]` — the graph's third lane, which is red in Latte, pink in
	 * Mocha and cyan in Dracula. A delete button that turns cyan is not a
	 * delete button. Checked against the background at 3:1 like the accent.
	 */
	danger: string;
	warn: string;
	ok: string;
	selection: string;
	stripe: string;
	/** The lane colour cycle. Five, because a sixth lane reuses the first. */
	lanes: [string, string, string, string, string];
}

export interface Variant {
	/** What this family calls it — "Latte", "Mocha", "Alucard". */
	name: string;
	palette: Palette;
}

export interface Family {
	id: FamilyId;
	name: string;
	light: Variant;
	dark: Variant;
}

/**
 * Pomodoro, by day — Spagitty's own family (FEAT-086), built from the brand
 * (FEAT-085): the tomato as the accent, basil, saffron, aubergine and sky for
 * the lanes. Danger is crimson rather than red, so a destructive button is
 * never read as the accent.
 *
 * On paper white since FEAT-156. The ground was warm cream, and with the
 * environment's tomato washes over it the whole window read as yellow. The
 * warmth is now a trace in the greys, and the colour is left to the accent.
 */
const GIORNO: Palette = {
	bg: '#fdfdfc',
	panel: '#f1f0ee',
	ink: '#24201e',
	muted: 'rgba(36, 32, 30, 0.68)',
	line: 'rgba(36, 32, 30, 0.2)',
	soft: 'rgba(36, 32, 30, 0.09)',
	placeholder: 'rgba(36, 32, 30, 0.34)',
	accent: '#b8321f',
	onAccent: '#ffffff',
	danger: '#b3124a',
	warn: '#9a5b00',
	ok: '#2f7d45',
	selection: 'rgba(184, 50, 31, 0.12)',
	stripe: 'rgba(36, 32, 30, 0.035)',
	lanes: ['#c23b22', '#2f8a52', '#a86a00', '#6c4fa3', '#2f6fb0']
};

/**
 * Pomodoro, by night: the same hues, lit, on charcoal.
 *
 * Less brown and a step softer since FEAT-156: the ground was near-black
 * umber and the text near-white cream, which is a lot of contrast to read all
 * day. The ink is a little dimmer and the ground a little lighter and greyer.
 */
const NOTTE: Palette = {
	bg: '#1e1b1a',
	panel: '#171514',
	ink: '#ebe4dc',
	muted: 'rgba(235, 228, 220, 0.62)',
	line: 'rgba(235, 228, 220, 0.2)',
	soft: 'rgba(235, 228, 220, 0.09)',
	placeholder: 'rgba(235, 228, 220, 0.3)',
	accent: '#f2715a',
	onAccent: '#2a0e07',
	danger: '#ff5c7c',
	warn: '#f0b54a',
	ok: '#7cc68d',
	selection: 'rgba(242, 113, 90, 0.18)',
	stripe: 'rgba(235, 228, 220, 0.04)',
	lanes: ['#f2715a', '#7cc68d', '#f0b54a', '#a58bd8', '#6aa7e0']
};

/** The families Settings could offer; one, now (TASK-051). */
export const FAMILIES: Family[] = [
	{
		id: 'pomodoro',
		name: 'Pomodoro',
		light: { name: 'Giorno', palette: GIORNO },
		dark: { name: 'Notte', palette: NOTTE }
	}
];

/**
 * What a fresh install opens on.
 *
 * `src/app.css` carries this family's two palettes as its boot values, so the
 * first paint — before any JavaScript has run — is already the default theme
 * rather than a flash of something else.
 */
export const DEFAULT_FAMILY: FamilyId = 'pomodoro';

export function isFamily(value: string): value is FamilyId {
	return FAMILIES.some((family) => family.id === value);
}

export function familyOf(id: FamilyId): Family {
	return FAMILIES.find((family) => family.id === id) ?? FAMILIES[0];
}

/** The variant of `id` for `mode`, name and palette. */
export function variantOf(id: FamilyId, mode: Mode): Variant {
	return familyOf(id)[mode];
}

export function paletteOf(id: FamilyId, mode: Mode): Palette {
	return variantOf(id, mode).palette;
}

/**
 * The palette as CSS custom properties, ready to set on an element.
 *
 * The property names are `src/app.css`'s, and this is the only place the two
 * naming schemes meet — everywhere else reads `var(--bg)` and knows nothing
 * about `Palette`.
 */
export function properties(palette: Palette): Record<string, string> {
	const { lanes, onAccent, ...rest } = palette;

	const tokens: Record<string, string> = { '--on-accent': onAccent };
	for (const [name, value] of Object.entries(rest)) {
		tokens[`--${name}`] = value;
	}
	lanes.forEach((colour, index) => {
		tokens[`--lane-${index + 1}`] = colour;
	});

	return tokens;
}

/**
 * The accent: the one colour that says "this is the one", picked in
 * Appearance (FEAT-156).
 *
 * The family's own is the tomato, and it is the default. The others replace
 * three tokens and nothing else — the accent, the text on it and the selection
 * tint made from it — so the lanes, the semantic colours and the brand's own
 * tomato in the wordmark stay as they are. None of them is red, amber or
 * green, which already mean danger, warning and success.
 *
 * Each has a shade for each mode, held to the same bars as a family's accent:
 * 3:1 against the background, 4.5:1 for the text on it.
 */
export type AccentId = 'tomato' | 'ocean' | 'violet' | 'jade' | 'graphite';

export interface AccentShade {
	accent: string;
	onAccent: string;
}

export interface Accent {
	id: AccentId;
	name: string;
	light: AccentShade;
	dark: AccentShade;
}

export const ACCENTS: Accent[] = [
	{
		id: 'tomato',
		name: 'Tomato',
		light: { accent: GIORNO.accent, onAccent: GIORNO.onAccent },
		dark: { accent: NOTTE.accent, onAccent: NOTTE.onAccent }
	},
	{
		id: 'ocean',
		name: 'Ocean',
		light: { accent: '#2563c9', onAccent: '#ffffff' },
		dark: { accent: '#6ea8ff', onAccent: '#0b1a33' }
	},
	{
		id: 'violet',
		name: 'Violet',
		light: { accent: '#7048c8', onAccent: '#ffffff' },
		dark: { accent: '#b39bff', onAccent: '#1c0f3d' }
	},
	{
		id: 'jade',
		name: 'Jade',
		light: { accent: '#0f7a63', onAccent: '#ffffff' },
		dark: { accent: '#4fd1ae', onAccent: '#04261d' }
	},
	{
		id: 'graphite',
		name: 'Graphite',
		light: { accent: '#4a4d55', onAccent: '#ffffff' },
		dark: { accent: '#c9ccd3', onAccent: '#1a1b1f' }
	}
];

/** The family's own: nothing is replaced. */
export const DEFAULT_ACCENT: AccentId = 'tomato';

export function isAccent(value: string): value is AccentId {
	return ACCENTS.some((accent) => accent.id === value);
}

export function accentOf(id: AccentId): Accent {
	return ACCENTS.find((accent) => accent.id === id) ?? ACCENTS[0];
}

/** `#rrggbb` at `alpha`, as the palettes write a tint. */
function tint(hex: string, alpha: number): string {
	const value = parseInt(hex.slice(1), 16);
	return `rgba(${(value >> 16) & 255}, ${(value >> 8) & 255}, ${value & 255}, ${alpha})`;
}

/**
 * `palette` with the accent `id` in place of its own.
 *
 * The default returns the palette as it is, so a family's own selection tint
 * is kept exactly rather than recomputed.
 */
export function withAccent(palette: Palette, id: AccentId, mode: Mode): Palette {
	if (id === DEFAULT_ACCENT) return palette;
	const shade = accentOf(id)[mode];
	return {
		...palette,
		accent: shade.accent,
		onAccent: shade.onAccent,
		selection: tint(shade.accent, mode === 'dark' ? 0.18 : 0.12)
	};
}
