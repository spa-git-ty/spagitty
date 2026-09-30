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
 * the lanes, on warm cream. Danger is crimson rather than red, so a
 * destructive button is never read as the accent.
 */
const GIORNO: Palette = {
	bg: '#fbf7f1',
	panel: '#f3ece2',
	ink: '#2a1f1a',
	muted: 'rgba(42, 31, 26, 0.7)',
	line: 'rgba(42, 31, 26, 0.24)',
	soft: 'rgba(42, 31, 26, 0.1)',
	placeholder: 'rgba(42, 31, 26, 0.32)',
	accent: '#b8321f',
	onAccent: '#ffffff',
	danger: '#b3124a',
	warn: '#9a5b00',
	ok: '#2f7d45',
	selection: 'rgba(184, 50, 31, 0.12)',
	stripe: 'rgba(42, 31, 26, 0.04)',
	lanes: ['#c23b22', '#2f8a52', '#a86a00', '#6c4fa3', '#2f6fb0']
};

/** Pomodoro, by night: the same hues, lit, on warm charcoal. */
const NOTTE: Palette = {
	bg: '#1c1613',
	panel: '#161110',
	ink: '#f3e9dd',
	muted: 'rgba(243, 233, 221, 0.62)',
	line: 'rgba(243, 233, 221, 0.22)',
	soft: 'rgba(243, 233, 221, 0.1)',
	placeholder: 'rgba(243, 233, 221, 0.28)',
	accent: '#f2715a',
	onAccent: '#2a0e07',
	danger: '#ff5c7c',
	warn: '#f0b54a',
	ok: '#7cc68d',
	selection: 'rgba(242, 113, 90, 0.18)',
	stripe: 'rgba(243, 233, 221, 0.045)',
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
