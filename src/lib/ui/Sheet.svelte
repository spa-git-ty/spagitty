<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * A window over the app, in the dialog's glass (FEAT-107): the backdrop,
	 * the panel, its edge and its entrance. For windows wider than a question
	 * — a hook's log, the hooks themselves — so each does not carry its own
	 * copy of the material.
	 */
	interface Props {
		/** The id of the element that names the window. */
		labelledby: string;
		width?: string;
		/** Escape, and a click outside — when the window may be dismissed. */
		ondismiss?: () => void;
		onkeydown?: (event: KeyboardEvent) => void;
		children: Snippet;
	}

	let { labelledby, width = '720px', ondismiss, onkeydown, children }: Props = $props();

	function keydown(event: KeyboardEvent) {
		onkeydown?.(event);
		if (!event.defaultPrevented && event.key === 'Escape' && ondismiss) {
			event.preventDefault();
			ondismiss();
		}
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) ondismiss?.();
	}}
>
	<div
		class="panel"
		style="width: min({width}, 94vw)"
		role="dialog"
		aria-modal="true"
		aria-labelledby={labelledby}
		tabindex="-1"
		onkeydown={keydown}
	>
		{@render children()}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: color-mix(in srgb, var(--umbra) 40%, transparent);
		z-index: 61;
	}

	.panel {
		max-height: 84vh;
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 16px;
		background-color: var(--glass-thick);
		backdrop-filter: var(--blur-thick);
		-webkit-backdrop-filter: var(--blur-thick);
		border: var(--glass-edge-line);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		box-shadow: var(--shadow-3);
		animation: rise-in var(--t-enter-liquid) var(--spring-liquid);
	}
</style>
