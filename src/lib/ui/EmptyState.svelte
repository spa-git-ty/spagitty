<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * A screen with nothing to show, said in the middle of the window
	 * (BUG-056) — the window, not the pane or column it happens to sit in, so it
	 * lands in the same place on every screen and does not move as panes are
	 * resized. Only the message takes the pointer; everything around it is
	 * clicked through.
	 */
	interface Props {
		message: string;
		error?: boolean;
		action?: Snippet;
	}

	let { message, error = false, action }: Props = $props();
</script>

<div class="center">
	<div class="state" role="status">
		<p class="note" class:error>{message}</p>
		{@render action?.()}
	</div>
</div>

<style>
	.center {
		position: fixed;
		inset: 0;
		display: grid;
		place-items: center;
		pointer-events: none;
		z-index: 2;
	}

	.state {
		pointer-events: auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		max-width: min(520px, 80vw);
		text-align: center;
	}

	p {
		margin: 0;
	}

	.error {
		color: var(--danger);
	}
</style>
