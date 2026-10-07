<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { repo } from '$lib/repo.svelte';
	import RequestRow from '$lib/requests/RequestRow.svelte';
	import PRWorkspace from '$lib/requests/PRWorkspace.svelte';
	import { requests } from '$lib/requests/store.svelte';
	import CreatePRModal from '$lib/requests/CreatePRModal.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import ScreenHead from '$lib/ui/ScreenHead.svelte';

	/**
	 * What is waiting on you, above what is waiting on everyone else.
	 *
	 * The one screen in Spagitty whose data comes off the network (FEAT-017).
	 * Read once when the repository changes and on demand — not on a timer:
	 * polling a host on a schedule nobody asked for spends somebody's rate
	 * limit while they are not looking at the screen.
	 *
	 * Every failure is the host's own sentence. "Could not load" is useless to
	 * somebody deciding whether to wait or to go and fix something, and offline,
	 * rate limited, refused and no-account are four different decisions.
	 */
	const needingYou = $derived(requests.needingYou);
	const waiting = $derived(requests.waitingOnOthers);

	/**
	 * No account for this host is a state, not an error (BUG-056): said as
	 * Review says it, whichever way the backend reported it.
	 */
	const noAccount = $derived(
		requests.error ? requests.error.includes('no account is connected') : !requests.connected
	);

	let generation: number | null = null;
	$effect(() => {
		const current = repo.generation;
		if (repo.info === null) return;
		if (generation === current) return;

		generation = current;
		untrack(() => {
			requests.clear();
			requests.load();
		});
	});
</script>

{#if requests.viewMode === 'workspace' && requests.open}
	<PRWorkspace />
{:else}
	<div class="screen">
		<ScreenHead
			title="Pull requests"
			repository={requests.repo ? `${requests.repo.owner}/${requests.repo.name}` : null}
		>
			{#snippet detail()}
				{#if requests.connected && requests.all.length > 0}
					<span class="note">{needingYou.length} waiting on you · {waiting.length} on others</span>
				{/if}
			{/snippet}
			{#snippet actions()}
				{#if requests.loading}<Loader size="inline" label="Reading…" />{/if}
				{#if requests.connected}
					<Btn primary onclick={() => requests.openCreateModal()}>+ Create PR</Btn>
				{/if}
				<Btn disabled={requests.loading} onclick={() => requests.load()}>Refresh</Btn>
			{/snippet}
		</ScreenHead>

		<div class="body">
			<div class="lists">
				{#if requests.loading && requests.all.length === 0}
					<Loader label="Reading pull requests…" />
				{:else if requests.repo === null && !requests.error}
					<EmptyState message="This repository is not on a service Spagitty can read." />
				{:else if noAccount}
					<EmptyState message="No account is connected.">
						{#snippet action()}
							<Btn primary onclick={() => goto('/settings#accounts')}>Settings → Accounts</Btn>
						{/snippet}
					</EmptyState>
				{:else if requests.error}
					<!--
						The host's own words. Offline, rate limited and refused are
						different decisions for the reader, and the backend already
						told them apart.
					-->
					<EmptyState message={requests.error} error>
						{#snippet action()}
							<Btn onclick={() => goto('/settings#accounts')}>Settings → Accounts</Btn>
						{/snippet}
					</EmptyState>
				{:else if requests.all.length === 0}
					<EmptyState message="Nothing open. Every pull request on this repository is closed." />
				{:else}
					{#if needingYou.length > 0}
						<section class="group">
							<h2 class="note heading">Needs you</h2>
							<ul class="list">
								{#each needingYou as request (request.id)}
									<RequestRow {request} />
								{/each}
							</ul>
						</section>
					{/if}

					{#if waiting.length > 0}
						<section class="group">
							<h2 class="note heading">Waiting on others</h2>
							<ul class="list">
								{#each waiting as request (request.id)}
									<RequestRow {request} waiting />
								{/each}
							</ul>
						</section>
					{/if}
				{/if}
			</div>
		</div>
	</div>

<CreatePRModal />
{/if}

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		min-width: 0;
	}

	.lists {
		flex: 1;
		min-width: 0;
		overflow: auto;
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.heading {
		margin: 0 0 6px;
		font-size: var(--fs-secondary);
		font-weight: inherit;
	}

	.list {
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
</style>
