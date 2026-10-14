<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { inTauri } from '$lib/api';
	import { repo } from '$lib/repo.svelte';
	import { releases } from '$lib/releases/store.svelte';
	import ReleasesScreen from '$lib/releases/ReleasesScreen.svelte';
	$effect(() => { const path = repo.info?.path ?? null; void repo.generation; untrack(() => { if (!path || inTauri()) void releases.load(path); }); });
	onDestroy(() => releases.detach());
</script>
<ReleasesScreen />
