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
 * | source    | means |
 * |-----------|-------|
 * | `manual`  | the user picked light or dark, and it stays |
 * | `system`  | follow the desktop's light/dark setting, live |
 * | `omarchy` | follow the desktop's whole **palette**, live (FEAT-080) |
 *
 * `mode` remains what is **resolved** and on screen, which is what everything
 * else in the application wants to read. Picking a family does not change the
 * source; picking a mode does, because picking a mode is the act of taking
 * over.
 *
 * # The revision, and why `id` is not enough
 *
 * `id` is `family-mode`, and it is what `LaneCanvas` invalidates its colour and
 * portrait caches from. Every followed desktop palette has the same one —
 * `omarchy-dark` — so switching between two different dark desktop themes is
 * invisible to anything keyed off it, and the graph would keep painting the
 * previous theme's lane colours (FEAT-080). `revision` is a counter bumped
 * whenever the *values* change, which is the question a cache actually has.
 *
 * # The cached palette
 *
 * Every commit also writes the *resolved* custom properties to storage, under
 * `spagitty.theme.palette`. Nothing in this module reads it back: it exists for
 * `static/theme-boot.js`, which runs before SvelteKit has loaded and paints the
 * right colours on the first frame instead of the fourth. See that file for why
 * it is a file rather than an inline script.
 */

import { fingerprint, paletteFrom, modeOf, type DesktopTheme } from './omarchy';
import {
	DEFAULT_FAMILY,
	isFamily,
	paletteOf,
	properties,
	variantOf,
	type FamilyId,
	type Mode,
	type Palette,
	type Variant
} from './themes';

/** Kept from before the family existed, so an upgrade keeps its light/dark. */
const MODE_KEY = 'spagitty.theme';
const FAMILY_KEY = 'spagitty.theme.family';
/** Where the mode comes from, as opposed to what it currently is. */
const SOURCE_KEY = 'spagitty.theme.source';
/** The resolved palette, for the boot script. Never read by this module. */
export const PALETTE_KEY = 'spagitty.theme.palette';

/** Where the light/dark answer, and possibly the whole palette, comes from. */
export type Source = 'manual' | 'system' | 'omarchy';

const SOURCES: Source[] = ['manual', 'system', 'omarchy'];

let mode = $state<Mode>('light');
let family = $state<FamilyId>(DEFAULT_FAMILY);
let source = $state<Source>('manual');
let revision = $state(0);

/**
 * The desktop's palette, while one is being followed.
 *
 * Held rather than re-derived on every read, and **kept through a failed
 * read**: `omarchy-theme-set` replaces a directory, so there is a moment when
 * the old palette is gone and the new one is not there yet. Dropping to the
 * built-in family for that moment would be a visible flash in the middle of a
 * theme change, which is the opposite of the feature.
 */
let desktop = $state<Palette | null>(null);
let desktopName = $state<string | null>(null);
/** Light or dark, as the desktop's own palette declares it. */
let desktopMode = $state<Mode | null>(null);
/** Why the desktop palette is not in use, for Appearance's quiet status line. */
let desktopReason = $state<string | null>(null);

/** The fingerprint of what is on screen, so `revision` moves only on a change. */
let painted = '';

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

/**
 * The palette that should be on screen.
 *
 * The desktop's, while one is being followed and one has been read; the chosen
 * family's otherwise. The fallback is not a failure path — it is what a machine
 * with no Omarchy gets, and what a followed machine gets before the first read
 * lands.
 */
function current(): Palette {
	if (source === 'omarchy' && desktop) return desktop;
	return paletteOf(family, mode);
}

/** Put the current palette on the document. */
function apply(): void {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;

	// Still set, because `app.css` keys its two boot blocks off it and because
	// it is what any future stylesheet rule would need.
	root.setAttribute('data-theme', mode);

	const palette = current();
	const tokens = properties(palette);
	for (const [name, value] of Object.entries(tokens)) {
		root.style.setProperty(name, value);
	}

	// The revision, bumped on a change of *values* rather than of identity.
	// Two different desktop dark themes share `omarchy-dark`, so anything that
	// caches by `id` — the lane canvas does — would keep the old colours
	// through a switch between them.
	const next = fingerprint(palette);
	if (next !== painted) {
		painted = next;
		revision += 1;
	}

	// What the next cold start paints before any of this has loaded. Written
	// here rather than in `commit`, so a palette that arrives any other way —
	// a system change, a desktop's own palette — is cached too.
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
 * Take a reading of the desktop's palette.
 *
 * **A failed read is not applied.** `omarchy-theme-set` builds a `next-theme`
 * directory and replaces `current/theme`, so between those two moments there is
 * no readable palette at all — and falling back to the built-in family for that
 * moment would be a flash of Catppuccin in the middle of a theme change. The
 * last valid palette stays until a new valid one arrives, which is also what
 * happens if somebody uninstalls Omarchy while the window is open.
 */
export function receiveDesktop(theme: DesktopTheme | null): void {
	if (!theme) return;

	desktopReason = theme.reason;

	if (!theme.available || !theme.palette) return;

	desktop = paletteFrom(theme.palette);
	desktopName = theme.name;
	desktopMode = modeOf(theme.palette);

	if (source !== 'omarchy') return;

	// The desktop decides light or dark too: a followed palette carries its own
	// mode, and `data-theme` has to agree with it or `app.css`'s boot values
	// fight the inline properties on any token a palette does not set.
	mode = desktopMode;
	remember(MODE_KEY, mode);
	apply();
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
	 * How many times the palette's *values* have changed.
	 *
	 * What a cache keyed on colours should read, in place of `id`. Two
	 * different desktop dark themes share `omarchy-dark`, so `id` cannot see a
	 * switch between them — which is exactly the case FEAT-080 introduces and
	 * `LaneCanvas` would otherwise get wrong.
	 */
	get revision(): number {
		return revision;
	},

	/** True once a desktop palette has been read. Appearance offers the option. */
	get desktopAvailable(): boolean {
		return desktop !== null;
	},

	/** The desktop's own name for what is on — `sushi-dark-palette`. */
	get desktopName(): string | null {
		return desktopName;
	},

	/** Why there is no desktop palette, when there is none. */
	get desktopReason(): string | null {
		return desktopReason;
	},

	/** Hand the store a reading from the backend. */
	receiveDesktop,

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

	/**
	 * Follow the desktop's whole palette (FEAT-080).
	 *
	 * Selectable whether or not a palette has been read yet — the shell reads
	 * one at startup, but a restored preference has to survive being restored
	 * *before* that read lands, and refusing the source until the answer
	 * arrives would silently demote somebody's choice on every launch.
	 */
	followDesktop(): void {
		setSource('omarchy');
		// The palette carries its own mode, and taking it here rather than
		// waiting for the next reading is what stops the family chips and
		// `data-theme` disagreeing with what is painted for one event.
		if (desktopMode) {
			mode = desktopMode;
			remember(MODE_KEY, mode);
		}
		apply();
	},

	/**
	 * The family is orthogonal to where the mode comes from — but choosing one
	 * is an opt-out of following the desktop's palette, because a family and a
	 * desktop palette are two answers to the same question. It is not an
	 * opt-out of `system`, which answers a different one.
	 */
	setFamily(next: FamilyId): void {
		if (source === 'omarchy') setSource('manual');
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
		source = SOURCES.includes(storedSource as Source)
			? (storedSource as Source)
			: validMode
				? 'manual'
				: 'system';

		remember(SOURCE_KEY, source);
		commit(family, source === 'system' ? systemMode() : (validMode ?? systemMode()));
		watchSystem();
	},

	/**
	 * The variant's name, for a screen that wants to say what is on.
	 *
	 * A followed desktop has no family and no variant, so it answers with the
	 * desktop's own name where there is one — which is what somebody looking at
	 * Appearance wants to read, rather than "Mocha" for a palette that is not
	 * Catppuccin.
	 */
	get label(): string {
		if (source === 'omarchy' && desktop) return desktopName ?? 'the desktop';
		return variantOf(family, mode).name;
	},

	/** Stop following the desktop. For the shell's teardown. */
	dispose(): void {
		watching?.();
		watching = null;
	}
};
