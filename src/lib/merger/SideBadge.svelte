<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';

	/**
	 * A, B or ✎ in its side's colour (FEAT-100): which branch something is,
	 * or that it was typed by hand. The letter says it as well as the colour,
	 * so the two sides are told apart without telling colours apart.
	 */
	interface Props {
		/** `agent`: text an agent wrote, the fourth origin (2.0). */
		side: 'a' | 'b' | 'mine' | 'agent';
		small?: boolean;
	}

	let { side, small = false }: Props = $props();

	const LETTER = { a: 'A', b: 'B', mine: '✎' } as const;
</script>

<span class="badge {side}" class:small aria-hidden="true"
	>{#if side === 'agent'}<Icon name="agent" size="0.8em" weight={2.4} />{:else}{LETTER[side]}{/if}</span
>

<style>
	.badge {
		width: 18px;
		height: 18px;
		border-radius: 6px;
		display: inline-grid;
		place-items: center;
		flex: none;
		font-family: var(--font-ui);
		font-size: calc(var(--fs-mono) * 0.92);
		font-weight: 700;
		line-height: 1;
		color: var(--bg);
	}

	.small {
		width: 16px;
		height: 16px;
		font-size: calc(var(--fs-mono) * 0.84);
		border-radius: 5px;
	}

	.a {
		background: var(--side-a);
	}

	.b {
		background: var(--side-b);
	}

	.mine {
		background: var(--side-mine);
	}

	.agent {
		background: var(--agent);
	}
</style>
