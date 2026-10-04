// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How code, diffs and the words around them are set (FEAT-090).
 *
 * Settings › Reading. The choices are published as tokens on the document —
 * `--code-font`, `--fs-code`, `--code-lh`, `--code-ls`, `--font-ui` and the
 * diff colours — so Diff, Working copy, File history and Review all read the
 * same ones, and none of them knows there is a setting.
 *
 * Kept in `localStorage` beside the theme and the zoom, for the reason
 * `scale.svelte.ts` gives: they decide how the first frame looks, and the boot
 * path cannot wait on the backend to find out.
 *
 * Every face offered is bundled under `assets/fonts/` — the application works
 * offline, so nothing is fetched from a font service.
 */

import { scale } from './scale.svelte';

export type CodeFontId = 'atkinson-mono' | 'opendyslexic-mono' | 'lexend' | 'jetbrains-mono' | 'system';
export type UiFontId = 'system' | 'atkinson' | 'lexend';
export type RulerMode = 'off' | 'line' | 'chunk';
export type DiffColours = 'calm' | 'classic';

export interface FontChoice<Id extends string> {
	id: Id;
	label: string;
	/** The CSS font stack. */
	stack: string;
	/** What it is for, in one line, shown under the row. */
	note?: string;
}

const MONO_FALLBACK = "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace";
const UI_FALLBACK = "system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif";

export const CODE_FONTS: FontChoice<CodeFontId>[] = [
	{
		id: 'atkinson-mono',
		label: 'Atkinson Hyperlegible Mono',
		stack: `'Atkinson Hyperlegible Mono', ${MONO_FALLBACK}`,
		note: 'Made for low vision: b d p q and I l 1 all have different shapes.'
	},
	{
		id: 'opendyslexic-mono',
		label: 'OpenDyslexic Mono',
		stack: `'OpenDyslexic Mono', 'Atkinson Hyperlegible Mono', ${MONO_FALLBACK}`,
		note: 'Heavier letter bottoms stop letters flipping.'
	},
	{
		id: 'lexend',
		label: 'Lexend',
		stack: `'Lexend', ${UI_FALLBACK}`,
		note: 'Wide and calm. Not monospaced, so columns do not line up.'
	},
	{
		id: 'jetbrains-mono',
		label: 'JetBrains Mono',
		stack: `'JetBrains Mono', ${MONO_FALLBACK}`,
		note: 'A common coding font.'
	},
	{
		id: 'system',
		label: 'System monospace',
		stack: MONO_FALLBACK,
		note: 'Whatever your desktop uses.'
	}
];

export const UI_FONTS: FontChoice<UiFontId>[] = [
	{ id: 'system', label: 'System', stack: UI_FALLBACK },
	{ id: 'atkinson', label: 'Atkinson Hyperlegible', stack: `'Atkinson Hyperlegible', ${UI_FALLBACK}` },
	{ id: 'lexend', label: 'Lexend', stack: `'Lexend', ${UI_FALLBACK}` }
];

export const RULERS: { id: RulerMode; label: string; note: string }[] = [
	{ id: 'off', label: 'Off', note: 'Every line at full strength.' },
	{ id: 'line', label: 'One line', note: 'A soft band sits under the line you are reading.' },
	{ id: 'chunk', label: 'Whole chunk', note: 'Chunks other than yours fade back.' }
];

export const COLOURS: { id: DiffColours; label: string; note: string }[] = [
	{ id: 'calm', label: 'Calm', note: 'Thin side markers and faint tints; removed lines go quiet.' },
	{ id: 'classic', label: 'Classic', note: 'Full red and green rows, as on GitHub.' }
];

export const SIZE_MIN = 12;
export const SIZE_MAX = 20;
export const SIZE_STEP = 0.5;
export const LH_MIN = 1.3;
export const LH_MAX = 2.2;
export const LH_STEP = 0.05;
export const LS_MIN = 0;
export const LS_MAX = 0.08;
export const LS_STEP = 0.01;

export interface Reading {
	codeFont: CodeFontId;
	uiFont: UiFontId;
	/** Code size in CSS pixels, before zoom and text scale. */
	size: number;
	lineHeight: number;
	/** Letter spacing in `em`. */
	letterSpacing: number;
	ruler: RulerMode;
	colours: DiffColours;
	words: boolean;
}

/**
 * The design's reading set: the face made for low vision, a size and spacing
 * that give each line room, the ruler on one line, and calm colours with the
 * changed words marked. The interface keeps the desktop's face until it is
 * chosen, because that changes every screen rather than the reading ones.
 */
export const DEFAULTS: Reading = {
	codeFont: 'atkinson-mono',
	uiFont: 'system',
	size: 15,
	lineHeight: 1.85,
	letterSpacing: 0.02,
	ruler: 'line',
	colours: 'calm',
	words: true
};

/**
 * Code as it was set before any of this: the desktop's monospace, at the
 * size the type scale gives it, tight. The review room's `Aa` switches
 * between this and the reading set.
 */
export const PLAIN = {
	codeFont: 'system' as CodeFontId,
	size: 14.4,
	lineHeight: 1.45,
	letterSpacing: 0
};

const KEY = 'spagitty.reading';

let current = $state<Reading>({ ...DEFAULTS });

function clamp(value: number, low: number, high: number): number {
	return Math.min(Math.max(value, low), high);
}

function round(value: number, step: number): number {
	return Math.round(Math.round(value / step) * step * 1000) / 1000;
}

function oneOf<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
	return typeof value === 'string' && (allowed as readonly string[]).includes(value)
		? (value as T)
		: fallback;
}

/** Whatever was stored, as reading preferences; each field on its own. */
export function normalise(value: unknown): Reading {
	const raw = typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : {};
	const number = (field: unknown, fallback: number, low: number, high: number) =>
		typeof field === 'number' && Number.isFinite(field) ? clamp(field, low, high) : fallback;
	return {
		codeFont: oneOf(raw.codeFont, CODE_FONTS.map((f) => f.id), DEFAULTS.codeFont),
		uiFont: oneOf(raw.uiFont, UI_FONTS.map((f) => f.id), DEFAULTS.uiFont),
		size: number(raw.size, DEFAULTS.size, SIZE_MIN, SIZE_MAX),
		lineHeight: number(raw.lineHeight, DEFAULTS.lineHeight, LH_MIN, LH_MAX),
		letterSpacing: number(raw.letterSpacing, DEFAULTS.letterSpacing, LS_MIN, LS_MAX),
		ruler: oneOf(raw.ruler, ['off', 'line', 'chunk'] as const, DEFAULTS.ruler),
		colours: oneOf(raw.colours, ['calm', 'classic'] as const, DEFAULTS.colours),
		words: typeof raw.words === 'boolean' ? raw.words : DEFAULTS.words
	};
}

export function codeStack(id: CodeFontId): string {
	return (CODE_FONTS.find((font) => font.id === id) ?? CODE_FONTS[CODE_FONTS.length - 1]).stack;
}

export function uiStack(id: UiFontId): string {
	return (UI_FONTS.find((font) => font.id === id) ?? UI_FONTS[0]).stack;
}

/** Publish the tokens. The interface face is only set when it is not the system's. */
function apply(): void {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;
	root.style.setProperty('--code-font', codeStack(current.codeFont));
	root.style.setProperty('--code-lh', String(current.lineHeight));
	root.style.setProperty('--code-ls', `${current.letterSpacing}em`);
	if (current.uiFont === 'system') root.style.removeProperty('--font-ui');
	else root.style.setProperty('--font-ui', uiStack(current.uiFont));
	root.dataset.diff = current.colours;
	root.dataset.words = current.words ? 'on' : 'off';
	scale.setCodeSize(current.size);
}

function save(): void {
	try {
		localStorage.setItem(KEY, JSON.stringify(current));
	} catch {
		// It just will not persist.
	}
}

export const reading = {
	get current(): Reading {
		return current;
	},

	/** Change some preferences, publish them, keep them. */
	set(change: Partial<Reading>): void {
		const next = normalise({ ...current, ...change });
		next.size = round(next.size, SIZE_STEP / 5);
		next.lineHeight = round(next.lineHeight, LH_STEP / 5);
		next.letterSpacing = round(next.letterSpacing, 0.001);
		current = next;
		apply();
		save();
	},

	/** Back to the defaults. */
	reset(): void {
		current = { ...DEFAULTS };
		apply();
		save();
	},

	/** Restore what was chosen last. Called once at boot, after the scale. */
	init(): void {
		let stored: unknown = null;
		try {
			const raw = localStorage.getItem(KEY);
			stored = raw ? JSON.parse(raw) : null;
		} catch {
			stored = null;
		}
		current = normalise(stored);
		apply();
	}
};
