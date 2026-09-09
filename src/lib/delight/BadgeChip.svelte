<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Badge, Rarity } from '$lib/delight/badges';

	/**
	 * One badge, drawn once and used everywhere (FEAT-072).
	 *
	 * The badge screen, the reward moment and the actor rows all draw the same
	 * component at different sizes, because a badge somebody recognises in a
	 * grid has to be the same object they saw when they earned it. Rarity is
	 * carried by the ring rather than by a label: a legendary badge should be
	 * identifiable across a room, and a word underneath it is not.
	 *
	 * A locked badge is drawn as its own shape with the glyph removed, not as an
	 * empty box — the outline is what makes a badge screen read as a collection
	 * with holes in it rather than as a list that happens to be short.
	 */

	interface Props {
		found: Badge | null;
		/** Not earned yet. Drawn as an outline, and a secret one says nothing. */
		locked?: boolean;
		size?: 'small' | 'medium' | 'large';
		/** Equipped as this actor's title. */
		equipped?: boolean;
		onclick?: () => void;
	}

	let { found, locked = false, size = 'medium', equipped = false, onclick }: Props = $props();

	const secret = $derived(locked && (found === null || found.secret === true));
	const rarity = $derived<Rarity>(found?.rarity ?? 'common');
	const label = $derived(
		secret ? 'Undiscovered badge' : `${found?.name ?? 'Unknown badge'} — ${found?.line ?? ''}`
	);
</script>

<svelte:element
	this={onclick ? 'button' : 'div'}
	class="badge {rarity} {size}"
	class:locked
	class:secret
	class:shame={found?.shame}
	class:equipped
	title={label}
	aria-label={label}
	role={onclick ? 'button' : 'img'}
	{onclick}
>
	<span class="glyph" aria-hidden="true">{secret ? '?' : (found?.emoji ?? '·')}</span>
	{#if size !== 'small'}
		<span class="name">{secret ? '???' : (found?.name ?? 'Unknown')}</span>
	{/if}
	{#if equipped}<span class="pin" aria-hidden="true">title</span>{/if}
</svelte:element>

<style>
	.badge {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 4px;
		padding: 10px 8px;
		border-radius: var(--r-panel);
		border: 1px solid var(--soft);
		background-color: var(--surface-veil);
		position: relative;
		min-width: 92px;
		text-align: center;
		transition:
			transform var(--t-fast) var(--spring),
			border-color var(--t-fast) var(--ease),
			background var(--t-fast) var(--ease);
	}

	button.badge:hover {
		transform: translateY(-2px);
	}

	/*
	 * The badge's own mark. Sized from the type scale rather than in pixels
	 * (TASK-042), so raising the text size grows the badges with everything
	 * else — they were 22px and 54px whatever anybody had chosen.
	 */
	.glyph {
		font-size: calc(var(--fs-title) * 1.15);
		line-height: 1.1;
	}

	.small {
		min-width: 0;
		padding: 3px 6px;
		flex-direction: row;
	}

	.small .glyph {
		font-size: var(--fs-ui);
	}

	.large {
		min-width: 160px;
		padding: 18px 16px;
	}

	.large .glyph {
		font-size: calc(var(--fs-title) * 2.8);
	}

	.name {
		font-size: var(--fs-mono);
		color: var(--muted);
		max-width: 100%;
	}

	.large .name {
		font-size: var(--fs-title);
		color: var(--ink);
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	/*
	 * Rarity is a ring, and it gets stronger rather than louder.
	 *
	 * Common is the surface's own hairline — a common badge is still a badge and
	 * should not look broken — and each step up borrows more of the accent until
	 * legendary, which is the only one that glows. Reserving the glow for one
	 * tier is what keeps it meaning something.
	 */
	.uncommon {
		border-color: color-mix(in srgb, var(--ok) 40%, var(--soft));
	}

	.rare {
		border-color: color-mix(in srgb, var(--accent) 55%, transparent);
		background-color: color-mix(in srgb, var(--accent) 8%, var(--surface));
	}

	.epic {
		border-color: color-mix(in srgb, var(--accent) 80%, transparent);
		background-color: color-mix(in srgb, var(--accent) 14%, var(--surface));
	}

	.legendary {
		border-color: var(--warn);
		background-color: color-mix(in srgb, var(--warn) 16%, var(--surface));
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--warn) 35%, transparent);
	}

	/* An anti-badge is never gold. It is the surface, gone slightly wrong. */
	.shame {
		border-color: color-mix(in srgb, var(--danger) 35%, var(--soft));
		background-color: color-mix(in srgb, var(--danger) 7%, var(--surface));
		box-shadow: none;
	}

	.locked {
		background: transparent;
		border-style: dashed;
		border-color: var(--soft);
		box-shadow: none;
	}

	.locked .glyph,
	.locked .name {
		opacity: 0.45;
		filter: grayscale(1);
	}

	.secret .glyph {
		color: var(--muted);
	}

	.equipped {
		border-color: var(--accent);
	}

	.pin {
		position: absolute;
		top: -7px;
		right: -6px;
		font-size: var(--fs-mono);
		letter-spacing: 0.06em;
		text-transform: uppercase;
		padding: 1px 5px;
		border-radius: var(--r-pill);
		background: var(--accent);
		color: var(--on-accent);
	}
</style>
