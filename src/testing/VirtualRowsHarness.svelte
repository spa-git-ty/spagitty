<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { createRawSnippet } from 'svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';

	/**
	 * A list whose owner asks for its top row from an effect, as the review
	 * room does when a file opens (BUG-047).
	 */
	let { items, onrun }: { items: string[]; onrun: () => void } = $props();

	let list = $state<VirtualRows<string> | null>(null);
	const row = createRawSnippet((item: () => string) => ({ render: () => `<span>${item()}</span>` }));

	$effect(() => {
		onrun();
		list?.scrollToIndex(0, 'start');
	});
</script>

<VirtualRows bind:this={list} {items} key={(item: string) => item} estimate={() => 20} {row} overscan={600} />
