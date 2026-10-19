<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { farmStore } from '$lib/farm/store.svelte';
	import { openInMerger } from '$lib/farm/merge';
	import { fetchAll, quickStash } from '$lib/graph/actions';
	import { follow } from '$lib/notifications/act';
	import { repo } from '$lib/repo.svelte';
	import { requests } from '$lib/requests/store.svelte';
	import { review } from '$lib/review/store.svelte';
	import { network } from '$lib/network/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import type { PullRequest } from '$lib/types';
	import ActivityCards from './ActivityCards.svelte';
	import BranchesCard from './BranchesCard.svelte';
	import CommitsCard from './CommitsCard.svelte';
	import FarmCard from './FarmCard.svelte';
	import NeedsCard from './NeedsCard.svelte';
	import RequestsCard from './RequestsCard.svelte';
	import WorkingCard from './WorkingCard.svelte';
	import { age, needs, type Need } from './needs';
	import { home } from './store.svelte';
	import { shownCards } from './layout';
	import { settings } from '$lib/settings/store.svelte';
	import './home.css';

	/**
	 * Home (2A, FEAT-140): what this repository needs from you, then its state at
	 * a glance. The top of the rail, and the screen a launch opens on.
	 */

	const info = $derived(repo.info);
	const overview = $derived(home.overview);
	const here = $derived(farmStore.path !== null && farmStore.path === info?.path);
	const farm = $derived(here ? farmStore.farm : null);
	const me = $derived(requests.currentUser);
	let stashing = $state(false);

	/** The clock the ages are read against: whenever something was read. */
	const now = $derived.by(() => {
		void overview;
		void requests.all;
		void home.defaultRuns;
		void farmStore.farm;
		return Date.now();
	});

	const rows = $derived(
		needs({
			farm: farm ? { tasks: farmStore.tasks, details: farmStore.details, runs: farmStore.runs } : null,
			pullRequests: requests.all,
			me,
			failedRuns: home.failedRuns,
			assigned: home.assigned
		})
	);
	const failing = $derived(new Set(home.failedRuns.map((r) => r.branch)));

	// A different repository is everything read again.
	$effect(() => {
		const path = info?.path ?? null;
		untrack(() => void home.open(path, repo.info?.head.branch ?? null));
	});

	// The same repository changed under the screen: the git half again.
	$effect(() => {
		void repo.info;
		void repo.counts;
		untrack(() => void home.refreshGit());
	});

	// The forge half rides the watcher's clock while Home is open.
	$effect(() => home.follow(() => repo.info?.head.branch ?? null));

	function startFarm() {
		void goto('/farm?start=1');
	}

	async function openPullRequest(pr: PullRequest) {
		if (await review.open(pr)) void goto('/review');
		else void goto('/requests');
	}

	async function act(need: Need) {
		const source = need.source;
		switch (source.kind) {
			case 'task':
				if (need.kind === 'mergeReady' && (await openInMerger(source.task))) return;
				void goto(`/farm?task=${encodeURIComponent(source.task.id)}`);
				return;
			case 'pr':
				await openPullRequest(source.pr);
				return;
			case 'run':
				if (source.run.url) follow({ kind: 'url', url: source.run.url });
				else void goto('/pipelines/live');
				return;
			case 'issue':
				if (source.issue.url) follow({ kind: 'url', url: source.issue.url });
				else void goto('/issues');
		}
	}

	async function stash() {
		stashing = true;
		try {
			if (await quickStash()) await home.refreshGit();
		} finally {
			stashing = false;
		}
	}

	const fetched = $derived(
		info?.lastFetched ? `fetched ${age(info.lastFetched * 1000, now)} ago` : 'never fetched'
	);
	/** The cards, in the order Settings › Home chose (FEAT-148). */
	const cards = $derived(shownCards(settings.home));

	const facts = $derived(
		[overview?.remote, overview?.licence, overview?.version].filter((part): part is string => !!part)
	);
</script>

<div class="home">
	{#if !info}
		<div class="empty">
			{#if repo.busy}
				<Loader label="Opening…" />
			{:else}
				<h1>Open a repository</h1>
				<p class="home-quiet">Home lists what it needs from you, and how it is doing.</p>
				<Btn primary onclick={() => repo.choose()}>Open repository…</Btn>
				{#if repo.error}<p class="home-error">{repo.error}</p>{/if}
			{/if}
		</div>
	{:else}
		<header class="head">
			<div class="who">
				<div class="name-row">
					<h1>{info.name}</h1>
					{#if info.head.branch}
						<span class="branch mono" title={overview?.upstream ? `Against ${overview.upstream.upstream}` : 'No upstream'}>
							{info.head.branch}
							{#if overview?.upstream}
								<span class="up">↑{overview.upstream.ahead}</span><span class="home-quiet">↓{overview.upstream.behind}</span>
							{/if}
						</span>
					{:else if info.head.short}
						<span class="branch mono">detached at {info.head.short}</span>
					{/if}
					<span class="home-quiet">{fetched}</span>
				</div>
				{#if facts.length}<div class="facts home-quiet">{facts.join(' · ')}</div>{/if}
			</div>
			<div class="actions">
				<Btn disabled={network.running} busy={network.running && network.operation === 'fetch'} onclick={() => fetchAll()}>
					Fetch all
				</Btn>
				<Btn primary onclick={startFarm}>New farm</Btn>
			</div>
		</header>

		{#if home.errors.overview}<p class="home-error">{home.errors.overview}</p>{/if}

		<!--
			One grid, packed densely, so any order still fills it: the wide card
			takes two columns and two rows where there are three, and the next
			two cards stack beside it as the design drew them.
		-->
		<div class="grid">
			{#each cards as card (card.id)}
				<div class="slot" class:wide={card.wide} data-card={card.id}>
					{#if card.id === 'needs'}
						<NeedsCard {rows} {now} onact={act} />
					{:else if card.id === 'farm'}
						<FarmCard {farm} onstart={startFarm} />
					{:else if card.id === 'working'}
						<WorkingCard
							changed={home.changed}
							stashes={repo.counts.stashes}
							busy={stashing}
							oncommit={() => goto('/changes')}
							onstash={stash}
						/>
					{:else if card.id === 'branches'}
						<BranchesCard
							branches={overview?.branches ?? []}
							count={overview?.branchCount ?? 0}
							defaultBranch={overview?.defaultBranch ?? null}
							prs={requests.all}
							{failing}
							{now}
						/>
					{:else if card.id === 'requests'}
						<RequestsCard
							prs={requests.all}
							{me}
							connected={requests.connected || requests.loading}
							error={requests.error}
							onopen={openPullRequest}
						/>
					{:else if card.id === 'commits'}
						<CommitsCard {now} />
					{:else}
						<ActivityCards
							card={card.id}
							{overview}
							runs={home.defaultRuns}
							latest={home.latest}
							pipelinesError={home.errors.pipelines}
							hasHost={home.host !== null}
							{now}
						/>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.home {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		padding: 26px 30px 30px;
		display: flex;
		flex-direction: column;
		gap: 22px;
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
	}

	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: space-between;
		gap: 16px;
	}

	.who {
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
	}

	.name-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}

	h1 {
		margin: 0;
		font-size: calc(var(--fs-title) * 1.55);
		font-weight: 600;
		letter-spacing: -0.01em;
	}

	.branch {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 4px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
	}

	.up {
		color: var(--ok);
	}

	.facts {
		font-size: var(--fs-secondary);
	}

	.actions {
		display: flex;
		gap: 10px;
	}

	/* Three across, wrapping on a narrow window. */
	.grid {
		container-type: inline-size;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
		grid-auto-flow: row dense;
		gap: 18px;
	}

	.slot {
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.slot > :global(.home-card) {
		flex: 1;
	}

	/* The whole width until there are three columns to take two of. */
	.slot.wide {
		grid-column: 1 / -1;
	}

	@container (min-width: 1056px) {
		.slot.wide {
			grid-column: span 2;
			grid-row: span 2;
		}
	}

	@media (max-width: 700px) {
		.home {
			padding: 18px 16px;
		}
		.grid {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
