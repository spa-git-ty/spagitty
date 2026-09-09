// SPDX-License-Identifier: GPL-3.0-or-later

/** The regression contract for restrained spatial depth and selective glass. */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const css = readFileSync('src/app.css', 'utf8');

function componentsUnder(dir: string): string[] {
	return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) return componentsUnder(path);
		return entry.name.endsWith('.svelte') ? [path] : [];
	});
}

/**
 * Every `var(--token)` a component reads is a token that exists.
 *
 * FEAT-068's External Tools section was written against `--fg`, `--dim` and
 * `--bg-2`. None of them is defined anywhere, so each one fell through to a
 * hard-coded fallback — `#eee`, `#888`, `#141416` — and the section rendered a
 * fixed dark palette regardless of the theme. On a light theme it was pale text
 * on pale cards. Nothing failed, nothing warned, and it shipped.
 *
 * The failure is invisible by construction: `var(--nope, #eee)` is valid CSS
 * that works. Only a check like this one can see it.
 *
 * `KNOWN` is a shrinking list, not a permanent exemption. Every entry is a
 * component that still does this and is recorded rather than hidden; adding a
 * *new* one fails, and fixing an old one fails too until its row is deleted, so
 * the list cannot quietly grow or go stale.
 */
describe('the tokens components read', () => {
	/**
	 * Every token that exists: the stylesheet's, plus the ones published from
	 * JavaScript.
	 *
	 * `metrics.ts` and `panels.svelte.ts` set the structural sizes on `:root` at
	 * runtime — `--rail-w`, `--row-pitch`, `--detail-w` and the rest — which is
	 * the arrangement `docs/architecture.md` describes and is entirely correct.
	 * They are read from those two files rather than listed here, so a metric
	 * renamed on one side does not need remembering on the other.
	 */
	const published = ['src/lib/metrics.ts', 'src/lib/panels.svelte.ts', 'src/lib/scale.svelte.ts']
		.map((path) => readFileSync(path, 'utf8'))
		.flatMap((text) => [
			// `'--row-pitch'`, and `variable: 'rail-w'`.
			...[...text.matchAll(/'--([a-z0-9-]+)'/g)].map((match) => match[1]),
			...[...text.matchAll(/variable:\s*'([a-z0-9-]+)'/g)].map((match) => match[1]),
			// The `px` map, whose keys are token names without the `--`.
			...[...text.matchAll(/^\t\t'([a-z0-9-]+)':\s*[A-Z_]/gm)].map((match) => match[1])
		]);

	const defined = new Set([
		...[...css.matchAll(/--([a-z0-9-]+)\s*:/g)].map((match) => match[1]),
		...published
	]);

	/**
	 * Components that still read tokens that do not exist.
	 *
	 * **Empty, as of TASK-039.** It held eleven files — everything that arrived
	 * with FEAT-063, FEAT-067, FEAT-069, FEAT-070 and FEAT-071 — and the list
	 * was written as a shrinking debt rather than an exemption, so this is
	 * where that ends. The array stays because the *mechanism* is what the
	 * three assertions below are: a new offender fails the first, and a row
	 * left here after its file is fixed fails the second. Deleting the array
	 * would delete the only thing stopping the debt being re-opened quietly.
	 */
	const KNOWN: string[] = [];

	/** The tokens a file reads that nothing defines — its own included. */
	function undefinedTokens(path: string): string[] {
		const text = readFileSync(path, 'utf8');
		// A component may declare its own custom properties, and several set one
		// from JavaScript. Those are defined, just not in `app.css`.
		const own = new Set([...text.matchAll(/--([a-z0-9-]+)\s*:/g)].map((match) => match[1]));

		return [
			...new Set(
				[...text.matchAll(/var\(\s*--([a-z0-9-]+)/g)]
					.map((match) => match[1])
					.filter((name) => !defined.has(name) && !own.has(name))
			)
		];
	}

	const components = [...componentsUnder('src/lib'), ...componentsUnder('src/routes')].map(
		(path) => path.split('\\').join('/')
	);

	it('is true of every component but the ones already recorded', () => {
		const offenders = components.filter((path) => undefinedTokens(path).length > 0);

		expect(
			offenders.filter((path) => !KNOWN.includes(path)),
			'these read a token nothing defines, so they render a hard-coded fallback and ignore the theme'
		).toEqual([]);
	});

	it('keeps no stale row in the recorded list', () => {
		// A file that has been fixed must lose its row, or the list stops being
		// a debt and becomes a place things hide.
		const fixed = KNOWN.filter((path) => undefinedTokens(path).length === 0);

		expect(fixed, 'these are themed correctly now; delete their rows').toEqual([]);
	});

	it('and the section that was reported is one of the fixed ones', () => {
		expect(undefinedTokens('src/lib/settings/ExternalToolsSection.svelte')).toEqual([]);
	});

	/**
	 * The half of the defect this file could not see (TASK-039).
	 *
	 * `var(--nope, #eee)` is one way to paint a fixed colour. `#eee` is the
	 * other, and it is the *commoner* one: eleven components read undefined
	 * tokens, and the same eleven also wrote plain literals — a near-black
	 * scrim, an amber that is Catppuccin Mocha's accent and nobody else's, a
	 * `#ffc107` warning, a checkerboard in two fixed greys. Every assertion
	 * above passed while those shipped, because a literal reads no token at
	 * all.
	 *
	 * So the contract is the stronger one: **a component's stylesheet names no
	 * colour.** Every colour in this application is either a palette token or
	 * derived from one with `color-mix`, and there is no case left that needs a
	 * literal — the two that looked like exceptions, a transparency
	 * checkerboard and text on a filled accent, are `--sunken`/`--soft` and
	 * `--on-accent` respectively.
	 *
	 * Comments are stripped first, because the accounts of *why* these were
	 * wrong quote the colours they replaced, and a test that forbade writing
	 * `#eee` in prose would delete its own evidence.
	 */
	it('names no colour of its own, anywhere', () => {
		const offenders = components
			.map((path) => {
				const text = readFileSync(path, 'utf8');
				const style = text.slice(text.indexOf('<style>'));
				const code = style.replace(/\/\*[\s\S]*?\*\//g, '');
				const literals = [
					...new Set(
						[...code.matchAll(/#[0-9a-fA-F]{3,8}\b|rgba?\([^)]*\)/g)]
							.map((match) => match[0])
							// `rgb(from …)` is a relative colour built out of a token,
							// which is the sanctioned way to derive one.
							.filter((value) => !/^rgba?\(\s*(?:var|from)/.test(value))
					)
				];
				return literals.length > 0 ? `${path}: ${literals.join(' ')}` : null;
			})
			.filter((entry) => entry !== null);

		expect(
			offenders,
			'a colour written here is a colour the theme cannot change; use a token or color-mix'
		).toEqual([]);
	});
});

/**
 * The controls that would otherwise be the platform's.
 *
 * A `<select>` ignores `background` and paints the desktop's own widget unless
 * `appearance: none` says not to — which on a dark theme means a white field in
 * a dark window. It was true of all five selects in the application and had
 * been since the first one was written, because the stylesheet's field rules
 * looked as though they covered it.
 */
describe('native widgets are taken over rather than trusted', () => {
	it('stops a select drawing itself', () => {
		expect(css).toMatch(/select\s*{[^}]*appearance:\s*none/s);
		expect(css, 'WebKit needs the prefix too').toMatch(
			/select\s*{[^}]*-webkit-appearance:\s*none/s
		);
	});

	it('draws the arrow in a colour that follows the theme', () => {
		// An inlined SVG chevron cannot read a custom property, so it would be
		// one hard-coded colour in every theme — the exact defect this replaced.
		expect(css).toMatch(/select\s*{[^}]*background-image:[^;]*currentcolor/s);
		expect(css).not.toMatch(/select\s*{[^}]*data:image\/svg/s);
	});

	it('never resets the field background with the shorthand', () => {
		// `background:` would wipe the chevron out on hover and on focus.
		// Anchored at the start of a line, so `::selection` — which merely
		// contains the word "select" — is not mistaken for a field rule.
		const fields = css.match(/^(?:input|select|textarea)[^{]*{[^}]*}/gms) ?? [];
		const shorthand = fields.filter((rule) => /\n\s*background:\s/.test(rule));

		expect(shorthand, 'use background-color, or the select loses its arrow').toEqual([]);
	});

	it('gives the checkbox and the radio the theme accent', () => {
		expect(css).toMatch(/input\[type='checkbox'\][\s\S]*?accent-color:\s*var\(--accent\)/);
	});
});

/**
 * A control does not move while it is being aimed at (TASK-042).
 *
 * Every button, chip and rail row lifted a pixel or two on hover. Each one is
 * a small thing and the sum is not: a pointer crossing a dialog's row of
 * buttons, or running down fourteen rail destinations to reach the eleventh,
 * sets off a shift under itself at every step. It is exactly the "incidental
 * motion" a UI review named, and it makes dense work feel unsteady in a way
 * nobody can point at.
 *
 * **The press keeps its motion.** That is the line: motion for something
 * somebody did, not for where their pointer happens to be. A key going down is
 * a state change; a hover is a position.
 *
 * Scoped to the shared kit and the chrome — the controls that appear on every
 * screen and repeat down a list. A card in the repositories grid or a badge in
 * the gallery still lifts, because those are single objects on a browsing
 * surface rather than targets in a row, and that affordance is doing work.
 */
/**
 * The type scale reaches every component (TASK-039, TASK-042).
 *
 * `scale.svelte.ts` implements the text-size preference by rewriting the
 * `--fs-*` tokens and nothing else. A component that names a pixel size is
 * therefore not merely off-scale, it is **unresizable**: somebody who raises
 * the text to 130% gets a larger application and a handful of components that
 * stay exactly where they were. There were 83 such sizes in eleven dialogs and
 * eight more scattered through the chrome and the badges; there are none.
 *
 * `em` and `calc(var(--fs-…) * n)` are both fine — they follow the scale. It is
 * the absolute unit that does not.
 */
describe('nothing names its own type size', () => {
	const everything = [...componentsUnder('src/lib'), ...componentsUnder('src/routes')];

	it('uses the scale, or a multiple of it, everywhere', () => {
		const offenders = everything
			.map((path) => {
				const text = readFileSync(path, 'utf8');
				const style = text.slice(text.indexOf('<style>')).replace(/\/\*[\s\S]*?\*\//g, '');
				const fixed = [...style.matchAll(/font-size:\s*(\d+(?:\.\d+)?px)/g)].map((m) => m[1]);
				return fixed.length > 0 ? `${path}: ${fixed.join(' ')}` : null;
			})
			.filter((entry) => entry !== null);

		expect(
			offenders,
			'a pixel size does not follow the text-size preference; use --fs-* or an em'
		).toEqual([]);
	});
});

describe('controls stay where they are', () => {
	const KIT = [
		'src/lib/ui/Btn.svelte',
		'src/lib/ui/Chip.svelte',
		'src/lib/ui/RefChip.svelte',
		'src/lib/ui/Menu.svelte',
		'src/lib/chrome/NavRail.svelte',
		'src/lib/chrome/Toolbar.svelte',
		'src/lib/chrome/TitleBar.svelte'
	];

	/** `:hover` rules in a component's stylesheet, comments stripped. */
	function hoverRules(path: string): { selector: string; body: string }[] {
		const text = readFileSync(path, 'utf8');
		const style = text.slice(text.indexOf('<style>')).replace(/\/\*[\s\S]*?\*\//g, '');

		return [...style.matchAll(/([^{}]*:hover[^{}]*)\{([^{}]*)\}/g)].map((match) => ({
			selector: match[1].trim().split('\n').pop()!.trim(),
			body: match[2]
		}));
	}

	it.each(KIT)('%s moves nothing on hover', (path) => {
		const moving = hoverRules(path)
			.filter((rule) => /transform:/.test(rule.body) && !/transform:\s*none/.test(rule.body))
			.map((rule) => rule.selector);

		expect(moving, 'hover changes colour; a press may move').toEqual([]);
	});

	/**
	 * And the other half, so this is a contract rather than a deletion: the
	 * feedback still exists, it has moved to the event that earns it.
	 */
	it('still answers a press', () => {
		const btn = readFileSync('src/lib/ui/Btn.svelte', 'utf8');
		expect(btn).toMatch(/:active:not\(:disabled\) \{[^}]*transform:/s);
	});
});

describe('the soft spatial interface', () => {
	it('keeps ordinary cards opaque', () => {
		expect(css).toMatch(/\.card\s*{[^}]*background-color:\s*var\(--surface\)/s);
		expect(css).not.toMatch(/\.card\s*{[^}]*backdrop-filter/s);
	});

	it('gives floating layers their own geometry and restrained depth', () => {
		expect(css).toContain('--r-floating: 18px');
		expect(css).toMatch(/--shadow-3:\s*[\s\S]*?30px/);
		expect(css).toMatch(/\.floating\s*{[^}]*border-radius:\s*var\(--r-floating\)/s);
	});

	it('uses backdrop blur only on a bounded set of transient components', () => {
		const blurred = [...componentsUnder('src/lib'), ...componentsUnder('src/routes')].filter(
			(path) => readFileSync(path, 'utf8').includes('backdrop-filter: var(--blur-thick)')
		);

		expect(blurred.length).toBeGreaterThanOrEqual(5);
		expect(blurred.length).toBeLessThanOrEqual(12);
		for (const path of blurred) expect(path).not.toMatch(/(CommitRows|DiffPane|FileList|NavRail)/);
	});

	it('removes motion when the platform requests it', () => {
		expect(css).toMatch(
			/@media \(prefers-reduced-motion: reduce\)[\s\S]*transition-duration:\s*0\.01ms !important/
		);
	});
});
