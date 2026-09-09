<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		primary?: boolean;
		/**
		 * An action that cannot be taken back — a delete, a hard reset, a force
		 * push. Tinted rather than filled: a solid red button reads as an error
		 * the screen is already in, where a tinted one reads as one it is
		 * offering.
		 */
		danger?: boolean;
		disabled?: boolean;
		title?: string;
		/**
		 * Turn the travelling glow off on a primary button.
		 *
		 * The glow marks *the* thing to do next, so a screen showing several
		 * primary buttons at once has more than one of them and none is the
		 * next thing. Set this on the ones that are merely filled.
		 */
		quiet?: boolean;
		onclick?: (event: MouseEvent) => void;
		children: Snippet;
	}

	let {
		primary = false,
		danger = false,
		disabled = false,
		quiet = false,
		title,
		onclick,
		children
	}: Props = $props();
</script>

<button
	class="btn"
	class:primary
	class:danger
	class:glow={primary && !quiet}
	{disabled}
	{title}
	{onclick}
>
	{@render children()}
</button>

<style>
	.btn {
		border-radius: var(--r-button);
		padding: 4px 12px;
		font-size: var(--fs-secondary);
		font-weight: 550;
		display: inline-flex;
		align-items: center;
		gap: 5px;
		white-space: nowrap;
		/* Deliberately no fill named here: a rule matching every button, the
		   glow included, may not touch one. BUG-002, and `btn.test.ts` reads
		   this file to keep it that way. The fills below animate themselves. */
		transition:
			border-color var(--t-fast) var(--ease),
			box-shadow var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease),
			transform var(--t-fast) var(--ease);
	}

	/*
	 * The background is set on everything *except* the glow, and that exclusion
	 * is the whole point rather than tidiness.
	 *
	 * `.glow` lives in `app.css` and paints the accent fill through
	 * `padding-box` with the travelling ring in `border-box`. Svelte scopes this
	 * rule to `.btn.s-hash`, which is specificity (0,2,0) against the global
	 * `.glow`'s (0,1,0) — so a plain `.btn { background: transparent }` wins and
	 * the primary button loses its fill entirely, leaving `--on-accent` text on
	 * the page background. That is BUG-002, and it is invisible in a screenshot
	 * of a light theme until someone looks for the label.
	 *
	 * A secondary button is a *surface* now rather than an outline: the raised
	 * shade, a catch of light along its top edge and a resting shadow, so it
	 * reads as something to press instead of a rectangle drawn around a word.
	 */
	.btn:not(.glow) {
		border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
		background-color: var(--surface-veil);
		box-shadow: var(--glass-rim), var(--shadow-1);
		transition-property: border-color, box-shadow, color, transform, background;
	}

	/*
	 * `:not(.glow)` here for the same reason as the fill above: the glow's
	 * border *is* its travelling ring, painted through `border-box`, so an
	 * opaque border colour on hover paints over the effect.
	 *
	 * Hover changes the border and the label. It no longer lifts (TASK-042).
	 *
	 * A control that rises a pixel as the pointer nears it is a target moving
	 * while it is being aimed at, and in a dialog with a row of buttons the
	 * pointer crosses two of them to reach the third. The colour change says
	 * "this one" at least as clearly and says it without moving the thing being
	 * pointed at.
	 *
	 * The press below keeps its motion: that is a key going down in response to
	 * something somebody did, which is exactly what motion is for.
	 */
	.btn:not(.glow):hover:not(:disabled) {
		border-color: color-mix(in srgb, var(--accent) 55%, var(--line));
		color: var(--accent);
		box-shadow: none;
	}

	/* Pressed, the key goes down. */
	.btn:not(.glow):active:not(:disabled) {
		background-color: var(--press);
		box-shadow: none;
		transform: translateY(1px) scale(0.985);
	}

	/*
	 * `:not(.glow)` on the fill, not on the colour.
	 *
	 * The text colour is shared, because it is the same either way.
	 */
	.btn.primary {
		color: var(--on-accent);
		box-shadow: none;
	}

	.btn.primary:not(.glow) {
		background: var(--accent);
		border-color: color-mix(in srgb, var(--accent-deep) 80%, transparent);
	}

	/* The glow's ring is 2px where the plain border is 1px; take the extra
	   pixel back out of the padding so both buttons are the same size. */
	.btn.glow {
		padding: 3px 11px;
	}

	.btn.primary:hover:not(:disabled) {
		color: var(--on-accent);
		filter: brightness(1.07) saturate(1.05);
	}

	.btn.primary:active:not(:disabled) {
		filter: brightness(0.96);
		transform: translateY(1px) scale(0.985);
		box-shadow: none;
	}

	/*
	 * Destructive, said the way GitKraken says it: the palette's own red as the
	 * border and the label, over a tint of it rather than a solid block. A row
	 * of solid red buttons reads as an error state; a tinted one reads as an
	 * action you should look at twice.
	 */
	.btn.danger:not(.glow) {
		color: var(--danger);
		border-color: color-mix(in srgb, var(--danger) 55%, transparent);
		background-color: color-mix(in srgb, var(--danger) 13%, var(--surface));
	}

	.btn.danger:not(.glow):hover:not(:disabled) {
		color: var(--danger);
		border-color: var(--danger);
		background-color: color-mix(in srgb, var(--danger) 22%, var(--surface));
		box-shadow: none;
	}

	.btn:disabled {
		opacity: 0.45;
		cursor: default;
		box-shadow: none;
	}

	/* Nothing lifts, drops or brightens for anyone who has asked the machine to
	   stop moving things. */
	@media (prefers-reduced-motion: reduce) {
		.btn {
			transition: none;
		}

		.btn:active:not(:disabled) {
			transform: none;
		}
	}
</style>
