// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Spagitty's palettes.
 *
 * Eight families, each with a light and a dark variant. They are **data**, not
 * stylesheets: sixteen `:root[data-theme=…]` blocks would be the same fourteen
 * tokens written sixteen times with no way to test them, so the table lives
 * here and `theme.svelte.ts` applies the chosen palette to the root element as
 * custom properties — the same mechanism `panels.svelte.ts` and `metrics.ts`
 * already use for the structural tokens.
 *
 * The keys are the token names in `src/app.css` with the `--` dropped. Nothing
 * else in the application has to learn a new name, and no component carries a
 * colour of its own.
 *
 * **The palettes are the published ones**, not approximations: Catppuccin
 * Latte and Mocha, Dracula and its official light counterpart Alucard, Tokyo
 * Night Day and Tokyo Night, Gruvbox light and dark, Nord's Snow Storm and
 * Polar Night, Rosé Pine Dawn and Rosé Pine, Solarized light and dark, and
 * Everforest light and dark. Where a token has no direct equivalent — `line`,
 * `soft`, `stripe`, `selection`, `placeholder` — it is derived from that
 * palette's own foreground and accent at a fixed opacity, so the family stays
 * internally consistent rather than borrowing a neutral from somewhere else.
 *
 * # Why every family now has its own accent
 *
 * It did not. Every light variant was accented with the brand amber darkened
 * for light surfaces (`#976317`) and every dark variant with the brand amber
 * itself (`#eeb04d`), on the argument that the identity should run through the
 * interface as well as the icon. What that actually produced was one hue
 * pasted onto seven palettes that were built around a different one: an amber
 * primary button and an amber active rail item in the middle of Dracula's
 * purples, Tokyo Night's blues and Nord's frost — and, in the light variants,
 * a brown that read as muddy against every one of their own backgrounds.
 *
 * A palette is a set of hues chosen to sit together. Overriding the one colour
 * the interface uses most is the fastest way to break that, so each family now
 * accents with a colour of its own — Catppuccin with its peach, Dracula with
 * its purple, Nord with its frost, Everforest with its aqua. The brand amber
 * stays what it always was in the mark, and stays the accent of Catppuccin,
 * which is the default family and the one it was picked to sit with.
 *
 * # Why the light accents are darker than the published swatch
 *
 * The accent is a border, a link and a filled button, so it is held to 3:1
 * against the background and its label to 4.5:1 against the fill. Most
 * published light palettes pick their accents for *text on white*, where
 * neither of those applies; used as a fill they are pale enough that a white
 * label disappears. Where that happened the colour is walked down its own hue
 * — never to a neutral, and never to another family's colour — until it clears
 * both. `themes.test.ts` is what decides when it has.
 */

export type Mode = 'light' | 'dark';

export type FamilyId =
	| 'pomodoro'
	| 'catppuccin'
	| 'dracula'
	| 'tokyo-night'
	| 'gruvbox'
	| 'nord'
	| 'rose-pine'
	| 'solarized'
	| 'everforest';

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

/** Catppuccin Latte. */
const LATTE: Palette = {
	bg: '#eff1f5',
	panel: '#e6e9ef',
	ink: '#4c4f69',
	muted: 'rgba(76, 79, 105, 0.72)',
	line: 'rgba(76, 79, 105, 0.26)',
	soft: 'rgba(76, 79, 105, 0.12)',
	placeholder: 'rgba(76, 79, 105, 0.3)',
	// Latte's own peach (#fe640b) walked down its hue until a filled button
	// carries a white label. It is the light-surface reading of the same amber
	// the brand mark uses, which is why Catppuccin is the default family.
	accent: '#c05621',
	onAccent: '#ffffff',
	danger: '#d20f39',
	// Latte's own yellow is #df8e1d, which sits at 2.3:1 on its background —
	// the same problem its pink and peach have as lane colours. Darkened along
	// its own hue until a warning can be seen on both bg and panel.
	warn: '#bc6a00',
	// Latte's green, darkened for the same reason and to the same shade its
	// fifth lane already uses — one green in this palette, not two.
	ok: '#2e7d1f',
	selection: 'rgba(192, 86, 33, 0.14)',
	stripe: 'rgba(76, 79, 105, 0.05)',
	// Latte's pink, peach and yellow sit at 2.3–2.6:1 on its own background —
	// invisible as a one-pixel lane. Blue, mauve, red and teal are Latte's own;
	// the green is Latte's darkened until it clears 3:1.
	lanes: ['#1e66f5', '#8839ef', '#d20f39', '#179299', '#2e7d1f']
};

/** Catppuccin Mocha. */
const MOCHA: Palette = {
	bg: '#1e1e2e',
	panel: '#181825',
	ink: '#cdd6f4',
	muted: 'rgba(205, 214, 244, 0.6)',
	line: 'rgba(205, 214, 244, 0.24)',
	soft: 'rgba(205, 214, 244, 0.11)',
	placeholder: 'rgba(205, 214, 244, 0.26)',
	// The brand amber, which sits between Mocha's own peach and yellow and is
	// the reason those two were chosen for the mark in the first place.
	accent: '#eeb04d',
	onAccent: '#1e1e2e',
	danger: '#f38ba8',
	warn: '#f9e2af',
	ok: '#a6e3a1',
	selection: 'rgba(238, 176, 77, 0.18)',
	stripe: 'rgba(205, 214, 244, 0.05)',
	lanes: ['#89b4fa', '#cba6f7', '#f5c2e7', '#fab387', '#a6e3a1']
};

/** Dracula's official light counterpart, Alucard. */
const ALUCARD: Palette = {
	bg: '#fffbeb',
	panel: '#f5f2e4',
	ink: '#1f1f1f',
	muted: 'rgba(31, 31, 31, 0.6)',
	line: 'rgba(31, 31, 31, 0.24)',
	soft: 'rgba(31, 31, 31, 0.1)',
	placeholder: 'rgba(31, 31, 31, 0.28)',
	// Alucard's purple, which is Dracula's purple made for paper. The family is
	// known by that hue; accenting it with anything else makes it a stranger.
	accent: '#644ac9',
	onAccent: '#fffbeb',
	danger: '#cb3a2a',
	warn: '#a34d14',
	ok: '#14710a',
	selection: 'rgba(100, 74, 201, 0.14)',
	stripe: 'rgba(31, 31, 31, 0.045)',
	lanes: ['#644ac9', '#a3144d', '#cf6a00', '#14710a', '#036a96']
};

/** Dracula. */
const DRACULA: Palette = {
	bg: '#282a36',
	panel: '#21222c',
	ink: '#f8f8f2',
	muted: 'rgba(248, 248, 242, 0.62)',
	line: 'rgba(248, 248, 242, 0.24)',
	soft: 'rgba(248, 248, 242, 0.11)',
	placeholder: 'rgba(248, 248, 242, 0.26)',
	accent: '#bd93f9',
	onAccent: '#282a36',
	danger: '#ff5555',
	warn: '#ffb86c',
	ok: '#50fa7b',
	selection: 'rgba(189, 147, 249, 0.2)',
	stripe: 'rgba(248, 248, 242, 0.05)',
	lanes: ['#bd93f9', '#ff79c6', '#8be9fd', '#f1fa8c', '#50fa7b']
};

/** Tokyo Night Day. */
const TOKYO_DAY: Palette = {
	bg: '#e1e2e7',
	// Lighter than the background, which is Tokyo Night Day's own arrangement —
	// and what keeps its blue foreground readable on a panel.
	panel: '#e9e9ec',
	ink: '#3760bf',
	muted: 'rgba(55, 96, 191, 0.85)',
	line: 'rgba(55, 96, 191, 0.28)',
	soft: 'rgba(55, 96, 191, 0.13)',
	placeholder: 'rgba(55, 96, 191, 0.3)',
	// Tokyo Night Day's own blue is #2e7de9, which is too light to put white
	// text on — a filled primary button fails at 4.0:1. Darkened along its own
	// hue until it carries text; the lighter original stays as the first lane.
	accent: '#2b5bb8',
	onAccent: '#ffffff',
	danger: '#c64343',
	warn: '#8f5e15',
	ok: '#587539',
	selection: 'rgba(43, 91, 184, 0.16)',
	stripe: 'rgba(55, 96, 191, 0.06)',
	lanes: ['#2e7de9', '#9854f1', '#b15c00', '#007197', '#587539']
};

/** Tokyo Night. */
const TOKYO_NIGHT: Palette = {
	bg: '#1a1b26',
	panel: '#16161e',
	ink: '#c0caf5',
	muted: 'rgba(192, 202, 245, 0.6)',
	line: 'rgba(192, 202, 245, 0.24)',
	soft: 'rgba(192, 202, 245, 0.11)',
	placeholder: 'rgba(192, 202, 245, 0.26)',
	accent: '#7aa2f7',
	onAccent: '#1a1b26',
	danger: '#f7768e',
	warn: '#e0af68',
	ok: '#9ece6a',
	selection: 'rgba(122, 162, 247, 0.18)',
	stripe: 'rgba(192, 202, 245, 0.05)',
	lanes: ['#7aa2f7', '#bb9af7', '#7dcfff', '#e0af68', '#9ece6a']
};

/** Gruvbox light, hard background. */
const GRUVBOX_LIGHT: Palette = {
	bg: '#fbf1c7',
	panel: '#f2e5bc',
	ink: '#3c3836',
	muted: 'rgba(60, 56, 54, 0.66)',
	line: 'rgba(60, 56, 54, 0.28)',
	soft: 'rgba(60, 56, 54, 0.13)',
	placeholder: 'rgba(60, 56, 54, 0.3)',
	// Gruvbox's own faded orange, which is the colour the theme is known by and
	// the only one of its accents that reads as warm against that cream.
	accent: '#af3a03',
	onAccent: '#fbf1c7',
	danger: '#9d0006',
	warn: '#b57614',
	ok: '#79740e',
	selection: 'rgba(175, 58, 3, 0.14)',
	stripe: 'rgba(60, 56, 54, 0.055)',
	lanes: ['#076678', '#8f3f71', '#af3a03', '#79740e', '#b57614']
};

/** Gruvbox dark, medium background. */
const GRUVBOX_DARK: Palette = {
	bg: '#282828',
	panel: '#1d2021',
	ink: '#ebdbb2',
	muted: 'rgba(235, 219, 178, 0.62)',
	line: 'rgba(235, 219, 178, 0.26)',
	soft: 'rgba(235, 219, 178, 0.12)',
	placeholder: 'rgba(235, 219, 178, 0.28)',
	// Bright yellow, Gruvbox's own — the dark counterpart of the light
	// variant's faded orange, and the hue the theme is recognised by.
	accent: '#fabd2f',
	onAccent: '#282828',
	danger: '#fb4934',
	warn: '#fe8019',
	ok: '#b8bb26',
	selection: 'rgba(250, 189, 47, 0.2)',
	stripe: 'rgba(235, 219, 178, 0.05)',
	lanes: ['#83a598', '#d3869b', '#fe8019', '#b8bb26', '#8ec07c']
};

/**
 * Nord's light half.
 *
 * Nord publishes no light theme, but it does publish Snow Storm — nord4 to
 * nord6 — as the foreground end of its own scale, and the palette is built so
 * that Polar Night reads on Snow Storm as well as the reverse. That is what
 * this is: Nord's own colours with the scale turned over, rather than a
 * different palette wearing the name.
 */
const NORD_LIGHT: Palette = {
	bg: '#eceff4',
	panel: '#e5e9f0',
	ink: '#2e3440',
	muted: 'rgba(46, 52, 64, 0.7)',
	line: 'rgba(46, 52, 64, 0.24)',
	soft: 'rgba(46, 52, 64, 0.11)',
	placeholder: 'rgba(46, 52, 64, 0.3)',
	// Frost, deepened. Nord's #5e81ac is the hue the theme is known by and is
	// two shades too pale to carry a white label.
	accent: '#3b6489',
	onAccent: '#eceff4',
	// Aurora, walked down to the same depth: Nord's aurora is designed to sit
	// on Polar Night, and on Snow Storm every one of those five colours is
	// below 3:1.
	danger: '#a3232f',
	warn: '#96550f',
	ok: '#4e6b23',
	selection: 'rgba(59, 100, 137, 0.15)',
	stripe: 'rgba(46, 52, 64, 0.05)',
	lanes: ['#3b6489', '#7d4b78', '#a3591f', '#276f6c', '#4e6b23']
};

/** Nord, on its Polar Night background. */
const NORD_DARK: Palette = {
	bg: '#2e3440',
	panel: '#272c36',
	ink: '#d8dee9',
	muted: 'rgba(216, 222, 233, 0.64)',
	line: 'rgba(216, 222, 233, 0.24)',
	soft: 'rgba(216, 222, 233, 0.11)',
	placeholder: 'rgba(216, 222, 233, 0.27)',
	accent: '#88c0d0',
	onAccent: '#2e3440',
	danger: '#bf616a',
	warn: '#ebcb8b',
	ok: '#a3be8c',
	selection: 'rgba(136, 192, 208, 0.2)',
	stripe: 'rgba(216, 222, 233, 0.05)',
	lanes: ['#81a1c1', '#b48ead', '#8fbcbb', '#d08770', '#a3be8c']
};

/** Rosé Pine Dawn. */
const ROSE_PINE_DAWN: Palette = {
	bg: '#faf4ed',
	// Dawn's "surface" is lighter than its base, which is the palette's own
	// arrangement — panels lift off the page rather than sinking into it.
	panel: '#fffaf3',
	ink: '#575279',
	muted: 'rgba(87, 82, 121, 0.76)',
	line: 'rgba(87, 82, 121, 0.24)',
	soft: 'rgba(87, 82, 121, 0.1)',
	placeholder: 'rgba(87, 82, 121, 0.3)',
	// Dawn's love (#b4637a) deepened. Rosé Pine is a rose-and-iris palette and
	// the accent has to be one of the two; love is the one that stays warm
	// against this background rather than disappearing into the ink.
	accent: '#8c3f5b',
	onAccent: '#faf4ed',
	danger: '#a52f47',
	// Dawn's gold, deepened until it clears its own near-white panel.
	warn: '#8a5a09',
	// Rosé Pine has no green. This is Dawn's pine walked toward one, so that
	// "done" is unmistakably not "in progress" without importing a hue the
	// palette never had.
	ok: '#3f7050',
	selection: 'rgba(140, 63, 91, 0.13)',
	stripe: 'rgba(87, 82, 121, 0.05)',
	lanes: ['#286983', '#7048a0', '#8c3f5b', '#37788a', '#8a5a09']
};

/** Rosé Pine Moon. */
const ROSE_PINE_MOON: Palette = {
	bg: '#232136',
	panel: '#2a273f',
	ink: '#e0def4',
	muted: 'rgba(224, 222, 244, 0.62)',
	line: 'rgba(224, 222, 244, 0.22)',
	soft: 'rgba(224, 222, 244, 0.1)',
	placeholder: 'rgba(224, 222, 244, 0.26)',
	accent: '#ea9a97',
	onAccent: '#232136',
	danger: '#eb6f92',
	warn: '#f6c177',
	// The same borrowed green as Dawn's, lifted for a dark background: the
	// palette's own foam is a cyan and reads as information, not success.
	ok: '#8fc9a1',
	selection: 'rgba(234, 154, 151, 0.18)',
	stripe: 'rgba(224, 222, 244, 0.05)',
	lanes: ['#c4a7e7', '#ea9a97', '#9ccfd8', '#f6c177', '#eb6f92']
};

/** Solarized light. */
const SOLARIZED_LIGHT: Palette = {
	bg: '#fdf6e3',
	panel: '#eee8d5',
	// base01. Solarized's body text on paper is base00 (#657b83), which is
	// below 4.5:1 on its own background — the palette was drawn to a different
	// standard than the one this application holds text to.
	ink: '#43565c',
	muted: 'rgba(67, 86, 92, 0.82)',
	line: 'rgba(67, 86, 92, 0.26)',
	soft: 'rgba(67, 86, 92, 0.12)',
	placeholder: 'rgba(67, 86, 92, 0.32)',
	// Solarized blue, deepened. The published #268bd2 is the hue every
	// Solarized screenshot is recognised by and carries a white label at 3.5:1.
	accent: '#1a6a9c',
	onAccent: '#fdf6e3',
	danger: '#c02a24',
	warn: '#9c7500',
	ok: '#6d7c00',
	selection: 'rgba(26, 106, 156, 0.14)',
	stripe: 'rgba(67, 86, 92, 0.05)',
	lanes: ['#1a6a9c', '#5a5fbc', '#c04a10', '#1f7f77', '#6d7c00']
};

/** Solarized dark. */
const SOLARIZED_DARK: Palette = {
	bg: '#002b36',
	panel: '#073642',
	ink: '#93a1a1',
	muted: 'rgba(147, 161, 161, 0.7)',
	line: 'rgba(147, 161, 161, 0.26)',
	soft: 'rgba(147, 161, 161, 0.12)',
	placeholder: 'rgba(147, 161, 161, 0.3)',
	accent: '#3a9ede',
	onAccent: '#002b36',
	danger: '#e4564f',
	warn: '#c99a11',
	ok: '#96a812',
	selection: 'rgba(58, 158, 222, 0.2)',
	stripe: 'rgba(147, 161, 161, 0.06)',
	lanes: ['#3a9ede', '#8085d6', '#e07a3c', '#2aa198', '#96a812']
};

/** Everforest light, soft background. */
const EVERFOREST_LIGHT: Palette = {
	bg: '#f3ead3',
	// Everforest's own bg1, which is *lighter* than its background — the same
	// arrangement Rosé Pine Dawn uses, and what keeps its grey-green foreground
	// readable on a panel.
	panel: '#f4f0d9',
	ink: '#5c6a72',
	muted: 'rgba(92, 106, 114, 0.82)',
	line: 'rgba(92, 106, 114, 0.28)',
	soft: 'rgba(92, 106, 114, 0.12)',
	placeholder: 'rgba(92, 106, 114, 0.34)',
	// Everforest's blue, deepened. Its green is the theme's signature but it is
	// also what "verified" is painted in here, and an accent that cannot be
	// told from a pass is a worse trade than an accent one step round the wheel.
	accent: '#26708f',
	onAccent: '#f4f0d9',
	danger: '#c03a36',
	warn: '#a06f00',
	ok: '#5c7a1e',
	selection: 'rgba(38, 112, 143, 0.14)',
	stripe: 'rgba(92, 106, 114, 0.05)',
	lanes: ['#26708f', '#a3538c', '#c4571b', '#1f7f68', '#5c7a1e']
};

/** Everforest dark, medium background. */
const EVERFOREST_DARK: Palette = {
	bg: '#2d353b',
	panel: '#272f34',
	ink: '#d3c6aa',
	muted: 'rgba(211, 198, 170, 0.66)',
	line: 'rgba(211, 198, 170, 0.24)',
	soft: 'rgba(211, 198, 170, 0.11)',
	placeholder: 'rgba(211, 198, 170, 0.28)',
	accent: '#7fbbb3',
	onAccent: '#2d353b',
	danger: '#e67e80',
	warn: '#dbbc7f',
	ok: '#a7c080',
	selection: 'rgba(127, 187, 179, 0.2)',
	stripe: 'rgba(211, 198, 170, 0.05)',
	lanes: ['#7fbbb3', '#d699b6', '#e69875', '#83c092', '#a7c080']
};

/**
 * In the order Settings shows them, with the default first.
 *
 * The four that came after it are ordered the way the swatches read rather
 * than alphabetically: Nord and Rosé Pine are the two cool-and-quiet ones,
 * Solarized and Everforest the two that are built on a warm paper.
 */
export const FAMILIES: Family[] = [
	{
		id: 'pomodoro',
		name: 'Pomodoro',
		light: { name: 'Giorno', palette: GIORNO },
		dark: { name: 'Notte', palette: NOTTE }
	},
	{
		id: 'catppuccin',
		name: 'Catppuccin',
		light: { name: 'Latte', palette: LATTE },
		dark: { name: 'Mocha', palette: MOCHA }
	},
	{
		id: 'dracula',
		name: 'Dracula',
		light: { name: 'Alucard', palette: ALUCARD },
		dark: { name: 'Dracula', palette: DRACULA }
	},
	{
		id: 'tokyo-night',
		name: 'Tokyo Night',
		light: { name: 'Day', palette: TOKYO_DAY },
		dark: { name: 'Night', palette: TOKYO_NIGHT }
	},
	{
		id: 'gruvbox',
		name: 'Gruvbox',
		light: { name: 'Light', palette: GRUVBOX_LIGHT },
		dark: { name: 'Dark', palette: GRUVBOX_DARK }
	},
	{
		id: 'nord',
		name: 'Nord',
		light: { name: 'Snow Storm', palette: NORD_LIGHT },
		dark: { name: 'Polar Night', palette: NORD_DARK }
	},
	{
		id: 'rose-pine',
		name: 'Rosé Pine',
		light: { name: 'Dawn', palette: ROSE_PINE_DAWN },
		dark: { name: 'Moon', palette: ROSE_PINE_MOON }
	},
	{
		id: 'solarized',
		name: 'Solarized',
		light: { name: 'Light', palette: SOLARIZED_LIGHT },
		dark: { name: 'Dark', palette: SOLARIZED_DARK }
	},
	{
		id: 'everforest',
		name: 'Everforest',
		light: { name: 'Light', palette: EVERFOREST_LIGHT },
		dark: { name: 'Dark', palette: EVERFOREST_DARK }
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
