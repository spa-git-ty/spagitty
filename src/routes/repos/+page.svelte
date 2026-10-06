<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { onMount } from 'svelte';
	import { clone } from '$lib/clone/store.svelte';
	import RepoCard from '$lib/repos/RepoCard.svelte';
	import { repos } from '$lib/repos/store.svelte';
	import BrandMark from '$lib/ui/BrandMark.svelte';
	import Wordmark from '$lib/ui/Wordmark.svelte';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * Every repository you work in, and which ones need attention.
	 *
	 * Unlike the other screens this one does not need a repository open: it is
	 * where you go when none is, so it loads on mount rather than on the open
	 * repository changing.
	 */

	onMount(() => {
		repos.load();
	});

	const needing = $derived(repos.needingAttention);
	const idle = $derived(repos.idle);
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Your repositories</span>
			{#if repos.loaded && repos.cards.length > 0}
				<span class="note">
					{repos.cards.length === 1 ? '1 repository' : `${repos.cards.length} repositories`}
				</span>
			{/if}
		</div>
		<div class="right">
			{#if repos.loading}<Loader size="inline" label="Reading…" />{/if}
			<Btn disabled={repos.busy} onclick={() => repos.load()}>Refresh</Btn>
			<Btn disabled={repos.busy} onclick={() => clone.show()}>Clone…</Btn>
			<Btn primary disabled={repos.busy} onclick={() => repos.choose()}>
				Open repository…
			</Btn>
		</div>
	</header>

	<div class="body">
		{#if repos.error}
			<p class="note error">{repos.error}</p>
		{:else if !repos.loaded}
			<Loader label="Reading…" />
		{:else if repos.cards.length === 0}
			<div class="empty">
				<div class="brand-hero">
					<BrandMark size={48} />
					<Wordmark size={30} />
				</div>
				<p class="note">No repositories yet.</p>
				<div class="row">
					<Btn primary disabled={repos.busy} onclick={() => repos.choose()}>
						Open repository…
					</Btn>
					<Btn disabled={repos.busy} onclick={() => clone.show()}>Clone…</Btn>
				</div>
			</div>
		{:else}
			{#if needing.length > 0}
				<section class="group">
					<h2 class="note heading">Needs you</h2>
					<div class="grid">
						{#each needing as card (card.path)}
							<RepoCard {card} />
						{/each}
					</div>
				</section>
			{/if}

			{#if idle.length > 0}
				<section class="group">
					<h2 class="note heading">Nothing in progress</h2>
					<div class="grid">
						{#each idle as card (card.path)}
							<RepoCard {card} idle />
						{/each}
					</div>
				</section>
			{/if}
		{/if}
	</div>

	<footer class="foot">
		{#if repos.writeError}
			<span class="note error">{repos.writeError}</span>
		{/if}
	</footer>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head,
	.foot {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.head {
		justify-content: space-between;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.foot {
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-top: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.title {
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 12px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.heading {
		margin: 0;
		font-weight: inherit;
	}

	.grid {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
		align-items: stretch;
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 12px;
		max-width: 480px;
		padding: 16px 0;
	}

	.brand-hero {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 4px;
	}
	.empty p {
		margin: 0;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.error {
		color: var(--danger);
	}
</style>
