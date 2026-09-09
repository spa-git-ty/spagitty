// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Which palette is on: where the choice comes from, which family, and light or
 * dark within it.
 *
 * Switching is custom properties only — no component re-render, no stylesheet
 * swap. The chosen palette is written onto `<html>` as inline custom
 * properties, which beat anything in the stylesheet, so `src/app.css` can carry
 * the default family as its boot values and the first paint is already right.
 *
 * The palettes themselves are data in `./themes`. Nothing here knows a colour.
 *
 * This is stored in `localStorage` rather than with the other settings, and
 * deliberately: the theme has to be applied before anything has been read from
 * disk, and the boot path cannot wait on a Tauri command to find out what
 * colour the window is.
 *
 * # The source is not the mode (BUG-031)
 *
 * There used to be two facts here — family and mode — and `init()` collapsed a
 * third into them. On a first run it read the system's light/dark preference
 * **once** and wrote the answer down as an explicit mode. From that moment the
 * application had an explicit choice the user had never made, so a desktop
 * switching to dark in the evening moved everything except Spagitty. There was
 * no way back to following the system short of clearing storage, and no way to
 * tell a stored mode that had been chosen from one that had been sampled.
 *
 * So *where the preference comes from* is now its own value:
 *
 * | source   | means |
 * |----------|-------|
 * | `manual` | the user picked light or dark, and it stays |
 * | `system` | follow the desktop, live |
 *
 * `mode` remains what is **resolved** and on screen, which is what everything
 * else in the application wants to read. Picking a family does not change the
 * source; picking a mode does, because picking a mode is the act of taking
 * over. A third source arrives with FEAT-080.
 *
 * # The cached palette
 *
 * Every commit also writes the *resolved* custom properties to storage, under
 * `spagitty.theme.palette`. Nothing in this module reads it back: it exists for
 * `static/theme-boot.js`, which runs before SvelteKit has loaded and paints the
 * right colours on the first frame instead of the fourth. See that file for why
 * it is a file rather than an inline script.
 */

import {
	DEFAULT_FAMILY,
	isFamily,
	paletteOf,
	properties,
	variantOf,
	type FamilyId,
	type Mode,
	type Variant
} from './themes';

/** Kept from before the family existed, so an upgrade keeps its light/dark. */
const MODE_KEY = 'spagitty.theme';
const FAMILY_KEY = 'spagitty.theme.family';
/** Where the mode comes from, as opposed to what it currently is. */
const SOURCE_KEY = 'spagitty.theme.source';
/** The resolved palette, for the boot script. Never read by this module. */
export const PALETTE_KEY = 'spagitty.theme.palette';

/** Where the light/dark answer comes from. */
export type Source = 'manual' | 'system';

let mode = $state<Mode>('light');
let family = $state<FamilyId>(DEFAULT_FAMILY);
let source = $state<Source>('manual');

/** Torn down when the source stops being `system`, and on unmount. */
let watching: (() => void) | null = null;

/** Read a key, treating an unreadable store as an unset one. */
function stored(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		// Private mode or a locked-down webview.
		return null;
	}
}

function remember(key: string, value: string): void {
	try {
		localStorage.setItem(key, value);
	} catch {
		// The theme just won't persist. It is not worth failing a paint over.
	}
}

/** What the desktop is asking for right now, or light where it cannot say. */
function systemMode(): Mode {
	if (typeof matchMedia === 'undefined') return 'light';
	return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/** Put the current palette on the document. */
function apply(): void {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;

	// Still set, because `app.css` keys its two boot blocks off it and because
	// it is what any future stylesheet rule would need.
	root.setAttribute('data-theme', mode);

	const tokens = properties(paletteOf(family, mode));
	for (const [name, value] of Object.entries(tokens)) {
		root.style.setProperty(name, value);
	}

	// What the next cold start paints before any of this has loaded. Written
	// here rather than in `commit`, so a palette that arrives any other way —
	// a system change, and later a desktop's own palette — is cached too.
	remember(PALETTE_KEY, JSON.stringify({ mode, tokens }));
}

function commit(nextFamily: FamilyId, nextMode: Mode): void {
	family = nextFamily;
	mode = nextMode;
	apply();
	remember(FAMILY_KEY, nextFamily);
	remember(MODE_KEY, nextMode);
}

/**
 * Start or stop following the desktop.
 *
 * The listener is added only while the source is `system`, rather than added
 * once and ignored when it does not apply: a media query listener that fires
 * into a branch that discards it is a thing that looks like it works.
 */
function watchSystem(): void {
	watching?.();
	watching = null;

	if (source !== 'system' || typeof matchMedia === 'undefined') return;

	const query = matchMedia('(prefers-color-scheme: dark)');
	const onChange = () => {
		// Guarded, because the teardown and the event can race on a fast
		// preference change and applying a mode the source no longer wants
		// would be worse than missing one.
		if (source !== 'system') return;
		commit(family, systemMode());
	};

	// `addEventListener` on a MediaQueryList is the current API and the one
	// WebKit has had since 14. `addListener` is the deprecated fallback, kept
	// because the AppImage is deliberately built against an old glibc and can
	// meet an old WebKitGTK.
	if (typeof query.addEventListener === 'function') {
		query.addEventListener('change', onChange);
		watching = () => query.removeEventListener('change', onChange);
	} else if (typeof query.addListener === 'function') {
		query.addListener(onChange);
		watching = () => query.removeListener(onChange);
	}
}

function setSource(next: Source): void {
	source = next;
	remember(SOURCE_KEY, next);
	watchSystem();
}

export const theme = {
	get mode(): Mode {
		return mode;
	},
	get family(): FamilyId {
		return family;
	},
	/** Where the light/dark answer comes from, as opposed to what it is. */
	get source(): Source {
		return source;
	},
	get isDark(): boolean {
		return mode === 'dark';
	},

	/**
	 * What is on, as one string: `"catppuccin-dark"`.
	 *
	 * Read by the lane canvas to know when to repaint. A boolean could not say
	 * that the family changed while the mode did not, which is a repaint too.
	 */
	get id(): string {
		return `${family}-${mode}`;
	},

	/** The current variant, so a screen can say "Mocha" rather than "dark". */
	get variant(): Variant {
		return variantOf(family, mode);
	},

	/**
	 * Choose light or dark explicitly.
	 *
	 * This is the act of taking over from the desktop, so it sets the source as
	 * well. A control that changed the mode and left the source alone would be
	 * overruled by the next system change, which is a control that does not
	 * work.
	 */
	setMode(next: Mode): void {
		setSource('manual');
		commit(family, next);
	},

	/** Follow the desktop's light/dark preference, now and as it changes. */
	followSystem(): void {
		setSource('system');
		commit(family, systemMode());
	},

	/** The family is orthogonal to where the mode comes from. */
	setFamily(next: FamilyId): void {
		commit(next, mode);
	},

	toggle(): void {
		this.setMode(mode === 'dark' ? 'light' : 'dark');
	},

	/**
	 * Restore what was chosen last.
	 *
	 * Three cases, and the third is the migration:
	 *
	 * - a stored source is obeyed;
	 * - nothing stored **and** nothing else stored either is a fresh install,
	 *   which follows the system — that is what the old code did on the first
	 *   run, minus writing the answer down as a decision;
	 * - nothing stored but a mode present is an upgrade from before the source
	 *   existed. That mode is treated as `manual`, because it might have been
	 *   chosen, and demoting somebody's explicit dark theme to "whatever the
	 *   desktop says" would lose a real preference. The reverse mistake — a
	 *   sampled mode kept as manual — costs one visit to Appearance.
	 */
	init(): void {
		const storedFamily = stored(FAMILY_KEY);
		family = storedFamily && isFamily(storedFamily) ? storedFamily : DEFAULT_FAMILY;

		const storedMode = stored(MODE_KEY);
		const validMode = storedMode === 'light' || storedMode === 'dark' ? storedMode : null;

		const storedSource = stored(SOURCE_KEY);
		if (storedSource === 'system' || storedSource === 'manual') {
			source = storedSource;
		} else {
			source = validMode ? 'manual' : 'system';
		}

		remember(SOURCE_KEY, source);
		commit(family, source === 'system' ? systemMode() : (validMode ?? systemMode()));
		watchSystem();
	},

	/** Stop following the desktop. For the shell's teardown. */
	dispose(): void {
		watching?.();
		watching = null;
	}
};
