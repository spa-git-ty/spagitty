<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onDestroy, untrack, type Snippet } from 'svelte';
	import { inTauri } from '$lib/api';
	import { repo } from '$lib/repo.svelte';
	import { builder } from '$lib/hooks/builder.svelte';

	/** One lifetime for both tabs: changing tabs never discards a draft. */
	let { children }: { children: Snippet } = $props();
	$effect(() => {
		const path = repo.info?.path ?? null;
		void repo.generation;
		untrack(() => { if (!path || inTauri()) void builder.load(path); });
	});
	onDestroy(() => builder.detach());
</script>

{@render children()}
