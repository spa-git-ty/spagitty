<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { inTauri } from '$lib/api';
	import { repo } from '$lib/repo.svelte';
	import { pipelines } from '$lib/pipelines/store.svelte';
	import PipelinesScreen from '$lib/pipelines/PipelinesScreen.svelte';
	$effect(() => { const path = repo.info?.path ?? null; void repo.generation; untrack(() => { void pipelines.setView('preview'); if (!path || inTauri()) void pipelines.load(path); }); });
	onDestroy(() => pipelines.detach());
</script>
<PipelinesScreen />
