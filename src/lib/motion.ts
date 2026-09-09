// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Whether anything is allowed to move, asked where CSS cannot answer.
 *
 * `app.css` has a `prefers-reduced-motion` block that flattens every animation
 * and every transition in the application to nothing. It is thorough and it is
 * not sufficient, because it can only reach things the *browser* is animating.
 *
 * A Svelte transition is not one of those. `fly`, `fade`, `slide` and the rest
 * are driven from JavaScript: the runtime writes a new `transform` or `opacity`
 * into the element's inline style on every frame. There is no CSS transition to
 * shorten, so `transition-duration: 0.01ms !important` applies to nothing and
 * the element moves exactly as far and as long as it was told to.
 *
 * That is what `+layout.svelte` was doing. Every navigation ran an
 * unconditional `in:fly={{ y: 6, duration: 140 }}`, with a comment saying
 * `prefers-reduced-motion` turned it off in `app.css`. It did not. Somebody who
 * has asked their machine to stop moving things got a screen that slid on every
 * single navigation, and the evidence that it did not was a media query that
 * could not see it.
 *
 * So the question is asked here instead, and asked at the moment the transition
 * starts rather than once at module load — the preference can change while the
 * application is running, and a value captured at startup would be wrong for
 * the rest of the session.
 */

import { fly, type FlyParams, type TransitionConfig } from 'svelte/transition';

/** True when the desktop has asked for less movement. */
export function prefersReducedMotion(): boolean {
	if (typeof matchMedia === 'undefined') return false;
	return matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/**
 * A `fly` that is not a fly when movement is not wanted.
 *
 * Returns a config with no duration and no movement rather than declining to
 * transition at all: Svelte still needs *a* transition object for the element,
 * and a zero-duration one is the difference between "arrives instantly" and
 * "throws in the transition runtime".
 *
 * Used for the screen change in `+layout.svelte`. Any other JavaScript-driven
 * transition added later belongs here too, for the same reason.
 */
export function gentleFly(node: Element, params: FlyParams = {}): TransitionConfig {
	if (prefersReducedMotion()) return { duration: 0 };
	return fly(node, params);
}
