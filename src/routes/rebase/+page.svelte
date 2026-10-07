<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { branches } from '$lib/branches/store.svelte';
	import RebaseScreen from '$lib/rebase/RebaseScreen.svelte';
	import { rebase } from '$lib/rebase/store.svelte';
	import { repo } from '$lib/repo.svelte';

	/**
	 * Rebase (1E). The screen is `RebaseScreen` (FEAT-106); this is where it
	 * reads what it starts from.
	 */
	onMount(() => {
		if (!api.inTauri() || !repo.info) return;

		// A rebase left unfinished — by a previous session, or the command
		// line — is the screen's state on arrival.
		void rebase.refreshProgress();

		// The upstream the branch already tracks is the usual thing to replay
		// onto, so it is chosen and planned straight away. Anything else would
		// be a guess.
		void branches.load().then(() => {
			const current = branches.rows.find((row) => row.current);
			if (rebase.upstream === '' && current?.upstream) pick(current.upstream);
		});

		return () => rebase.clear();
	});

	function pick(name: string) {
		rebase.upstream = name;
		void rebase.load();
	}
</script>

<div class="screen">
	{#if repo.info}
		<RebaseScreen branch={repo.info.head.branch} headShort={repo.info.head.short} onpick={pick} />
	{:else}
		<p class="note empty">Open a repository to rebase its branches.</p>
	{/if}
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.empty {
		padding: 20px;
	}
</style>
