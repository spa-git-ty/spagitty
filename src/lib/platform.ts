// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What this platform is, for the handful of places the answer is visible.
 *
 * Two things need it and they need it for different reasons:
 *
 * - **Key names.** The same combination is written `⌘F` on a Mac and `Ctrl+F`
 *   everywhere else. Spagitty had three answers to that at once: the command
 *   palette formatted the modifier per platform, the Appearance section wrote
 *   `Ctrl` into its markup on every platform including macOS, and the title bar
 *   once carried a `⌘K` chip — a macOS key name, on Linux, for a shortcut that
 *   was `⌘F`. All three are the same sentence, so it is written once here.
 * - **Window chrome.** The window is undecorated on every platform and draws
 *   its own controls, which on macOS are in the wrong place, in the wrong
 *   order, and the wrong shape. Deciding that needs the same fact.
 *
 * # Why not `navigator.platform`
 *
 * It is deprecated, and every engine has been discussing freezing its value for
 * years. `navigator.userAgentData.platform` is the replacement and is not in
 * WebKit, which is the engine Spagitty actually ships on Linux and macOS — so
 * neither one alone answers. Both are consulted, newest first, and the user
 * agent string is the last resort. That is more machinery than a one-line check
 * and it is the difference between "correct today" and "correct after the next
 * WebKit release".
 *
 * # Why it is not a store
 *
 * The platform does not change while the application is running. A `$state`
 * would invite a component to react to it, and there is nothing to react to;
 * it is read once, at module load, and the result is a constant. `reset()`
 * exists for the tests, which need to pretend otherwise.
 */

/** What `navigator` looks like to the parts of this file that read it. */
interface PlatformSource {
	platform?: string;
	userAgent?: string;
	userAgentData?: { platform?: string };
}

/**
 * Whether `source` describes a Mac.
 *
 * Pure and exported so the table of cases can be a test rather than an
 * assumption about one machine. `iPhone` and `iPad` are matched too: Spagitty
 * does not run there, but the key-name answer is the same and a check that
 * quietly excluded them would be a check with an unstated boundary.
 */
export function isMacLike(source: PlatformSource | undefined): boolean {
	if (!source) return false;

	// The current API, where it exists. It reports `macOS` rather than `MacIntel`.
	const modern = source.userAgentData?.platform;
	if (modern) return /^mac/i.test(modern);

	// The deprecated one, which is what WebKit still answers.
	if (source.platform) return /Mac|iPhone|iPad/.test(source.platform);

	// Last resort. `Macintosh` appears in every Mac user agent string, and
	// `Mac OS X` in the WebKit ones Spagitty actually runs inside.
	return /Mac(intosh| OS X)|iPhone|iPad/.test(source.userAgent ?? '');
}

let mac = isMacLike(typeof navigator === 'undefined' ? undefined : navigator);

/** True on macOS. Read once at load; the platform does not change. */
export function isMac(): boolean {
	return mac;
}

/**
 * Re-read the platform. For tests, which cannot reload the module.
 *
 * Deliberately takes the source rather than reading `navigator` itself, so a
 * test states the case it is testing instead of mutating a global that the
 * next test then inherits.
 */
export function reset(source: PlatformSource | undefined): void {
	mac = isMacLike(source);
}

/**
 * The primary modifier, as this platform writes it.
 *
 * `⌘` carries no separator because that is how macOS writes a combination —
 * `⌘F`, not `⌘+F`. `Ctrl+` carries its own, so both compose the same way and a
 * caller never has to know which one it got.
 */
export function mod(): string {
	return mac ? '⌘' : 'Ctrl+';
}

/**
 * A shortcut, written for this platform: `shortcut('F')` is `⌘F` or `Ctrl+F`.
 *
 * The key is passed as it should be *displayed*, not as `KeyboardEvent.key`
 * reports it — `+`, `−` and `0` are what the zoom controls show, and two of
 * those are not the characters the event carries.
 */
export function shortcut(key: string): string {
	return `${mod()}${key}`;
}

/**
 * The alternate modifier, used where a chord needs a second one.
 *
 * Separate from [`mod`] because the two do not always agree about which
 * physical key they mean: on macOS the primary is Command and the alternate is
 * Option, and on everything else the primary is Control and the alternate is
 * Alt. Spelling both from one function would produce `⌘Alt` on a Mac.
 */
export function alt(): string {
	return mac ? '⌥' : 'Alt+';
}
