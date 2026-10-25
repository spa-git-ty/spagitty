<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Search } from './speedsearch.svelte';

	/**
	 * A row's text with what was typed into its list's search marked
	 * (FEAT-158). Plain text when nothing has been typed.
	 */

	interface Props {
		text: string;
		search: Search;
	}

	let { text, search }: Props = $props();

	const pieces = $derived.by(() => {
		const ranges = search.active ? search.ranges(text) : [];
		if (ranges.length === 0) return [{ text, hit: false }];
		const out: { text: string; hit: boolean }[] = [];
		let at = 0;
		for (const [start, end] of ranges) {
			if (start > at) out.push({ text: text.slice(at, start), hit: false });
			out.push({ text: text.slice(start, end), hit: true });
			at = end;
		}
		if (at < text.length) out.push({ text: text.slice(at), hit: false });
		return out;
	});
</script>

{#each pieces as piece, i (i)}{#if piece.hit}<mark class="hit">{piece.text}</mark>{:else}{piece.text}{/if}{/each}

<style>
	.hit {
		background: color-mix(in srgb, var(--accent) 24%, transparent);
		color: inherit;
		border-radius: 3px;
	}
</style>
