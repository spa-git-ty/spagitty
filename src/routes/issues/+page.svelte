<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { inTauri } from '$lib/api';
	import { repo } from '$lib/repo.svelte';
	import { issues } from '$lib/issues/store.svelte';
	import IssuesScreen from '$lib/issues/IssuesScreen.svelte';
	$effect(() => { const path = repo.info?.path ?? null; void repo.generation; untrack(() => { if (!path || inTauri()) void issues.load(path); }); });
	onDestroy(() => issues.detach());
</script>
<IssuesScreen />
