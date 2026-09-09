// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * One answer to "what is this platform", checked against the shapes the three
 * APIs actually produce (BUG-030).
 *
 * The application had three answers at once: the command palette read
 * `navigator.platform`, the Appearance section wrote `Ctrl` into its markup on
 * every platform including macOS, and the title bar once carried a `⌘K` chip —
 * a macOS key name, on Linux, for a shortcut that was `⌘F`.
 *
 * Detection is the part worth a table rather than an assumption about one
 * machine, because the answer depends on which of three APIs answers first and
 * the engine Spagitty ships on is the one that lacks the newest.
 */

import { describe, expect, it, beforeEach } from 'vitest';
import { alt, isMac, isMacLike, mod, reset, shortcut } from './platform';

describe('what counts as a Mac', () => {
	/**
	 * The current API wins where it exists. Chromium reports `macOS`, not
	 * `MacIntel`, so a check written for the old spelling answers `false` on a
	 * Mac the day the old one is frozen.
	 */
	it('prefers userAgentData, which spells it differently', () => {
		expect(isMacLike({ userAgentData: { platform: 'macOS' }, platform: 'Linux x86_64' })).toBe(true);
		expect(isMacLike({ userAgentData: { platform: 'Linux' }, platform: 'MacIntel' })).toBe(false);
	});

	/** Deprecated, and what WebKitGTK and Safari still answer. */
	it('falls back to navigator.platform', () => {
		for (const platform of ['MacIntel', 'MacPPC', 'iPhone', 'iPad']) {
			expect(isMacLike({ platform }), platform).toBe(true);
		}
		for (const platform of ['Linux x86_64', 'Win32', '']) {
			expect(isMacLike({ platform }), platform).toBe(false);
		}
	});

	/** Last resort, for an engine that has frozen or removed both. */
	it('reads the user agent string when neither API answers', () => {
		expect(
			isMacLike({
				userAgent:
					'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)'
			})
		).toBe(true);
		expect(isMacLike({ userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36' })).toBe(
			false
		);
	});

	/**
	 * Server-side rendering, and the tests that mount a component with no
	 * `navigator` at all. The answer has to be a boolean, not a throw.
	 */
	it('answers no when there is nothing to read', () => {
		expect(isMacLike(undefined)).toBe(false);
		expect(isMacLike({})).toBe(false);
	});
});

describe('how a shortcut is written', () => {
	beforeEach(() => reset({ platform: 'Linux x86_64' }));

	it('uses Ctrl with a separator off a Mac', () => {
		expect(isMac()).toBe(false);
		expect(mod()).toBe('Ctrl+');
		expect(shortcut('F')).toBe('Ctrl+F');
	});

	/**
	 * `⌘F`, not `⌘+F`. The separator belongs to the modifier rather than to the
	 * caller, so nothing that composes a shortcut has to know which platform it
	 * is on — which is the entire point of there being one function.
	 */
	it('uses the command glyph with no separator on a Mac', () => {
		reset({ platform: 'MacIntel' });

		expect(isMac()).toBe(true);
		expect(mod()).toBe('⌘');
		expect(shortcut('F')).toBe('⌘F');
		expect(shortcut('0')).toBe('⌘0');
	});

	/**
	 * The second modifier is a different physical key on each platform —
	 * Option against Alt — so spelling both from one function produces `⌘Alt`
	 * on a Mac. Two functions, deliberately.
	 */
	it('names the alternate modifier separately', () => {
		expect(alt()).toBe('Alt+');
		reset({ platform: 'MacIntel' });
		expect(alt()).toBe('⌥');
	});
});
