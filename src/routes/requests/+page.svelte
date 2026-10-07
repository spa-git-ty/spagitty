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
		<header class="head">
			<div class="left">
				<span class="title">Pull requests</span>
				{#if requests.repo}
					<span class="note mono">{requests.repo.owner}/{requests.repo.name}</span>
				{/if}
				{#if requests.connected && requests.all.length > 0}
					<span class="note">
						{needingYou.length} waiting on you · {waiting.length} on others
					</span>
				{/if}
			</div>
			<div class="right">
				{#if requests.loading}<Loader size="inline" label="Reading…" />{/if}
				{#if requests.connected}
					<Btn primary onclick={() => requests.openCreateModal()}>+ Create PR</Btn>
				{/if}
				<Btn disabled={requests.loading} onclick={() => requests.load()}>Refresh</Btn>
			</div>
		</header>

		<div class="body">
			<div class="lists">
				{#if requests.loading && requests.all.length === 0}
					<Loader label="Reading pull requests…" />
				{:else if requests.error}
					<!--
						The host's own words. Offline, rate limited, refused and
						"no account for this host" are four different decisions for
						the reader, and the backend already told them apart.
					-->
					<div class="empty">
						<p class="note error">{requests.error}</p>
						<Btn onclick={() => goto('/settings#accounts')}>Settings → Accounts</Btn>
					</div>
				{:else if requests.repo === null}
					<div class="empty">
						<p class="note">This repository is not on a service Spagitty can read.</p>
					</div>
				{:else if !requests.connected}
					<div class="empty">
						<p class="note">No account is connected.</p>
						<Btn primary onclick={() => goto('/settings#accounts')}>
							Settings → Accounts
						</Btn>
					</div>
				{:else if requests.all.length === 0}
					<p class="note">Nothing open. Every pull request on this repository is closed.</p>
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

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
		flex: none;
	}

	.right {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.left {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.title {
		font-size: var(--fs-title);
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

	.empty {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		max-width: 520px;
	}
</style>
