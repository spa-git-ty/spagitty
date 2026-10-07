<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import HooksView from '$lib/hooks/HooksView.svelte';
	import { repo } from '$lib/repo.svelte';

	/**
	 * The open repository's git hooks (FEAT-107): whether Spagitty runs them,
	 * and what each one runs. About the repository, like Remotes, so it names
	 * which one.
	 */
</script>

<section class="section" id="hooks">
	<header>
		<h2 class="heading">Hooks</h2>
		{#if repo.info}<span class="note">{repo.info.name}</span>{/if}
	</header>
	{#if repo.info === null}
		<p class="note">No repository is open.</p>
	{:else}
		<HooksView />
	{/if}
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 720px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	header {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}
</style>
