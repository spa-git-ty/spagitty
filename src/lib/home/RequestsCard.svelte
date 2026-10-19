<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import type { PullRequest } from '$lib/types';
	import { pullRequestOrder, pullRequestState } from './shape';

	/**
	 * Pull requests (FEAT-140): the open ones, the person's own and those
	 * waiting on them first, with their state in colour and a checks dot.
	 */
	interface Props {
		prs: PullRequest[];
		me: string | null;
		connected: boolean;
		error: string | null;
		onopen: (pr: PullRequest) => void;
	}

	let { prs, me, connected, error, onopen }: Props = $props();

	const shown = $derived(pullRequestOrder(prs, me).slice(0, 5));

	const CHECKS = {
		passing: { colour: 'var(--ok)', title: 'Checks passed' },
		failing: { colour: 'var(--danger)', title: 'Checks failed' },
		running: { colour: 'var(--warn)', title: 'Checks running' }
	} as const;
</script>

<section class="card home-card" aria-labelledby="home-prs">
	<div class="home-card-head">
		<h2 id="home-prs">Pull requests</h2>
		<button class="home-link" onclick={() => goto('/requests')}>{prs.length} open</button>
	</div>
	{#if error}
		<p class="home-error">{error}</p>
	{:else if !connected}
		<p class="home-quiet">No account for this repository's host. Connect one in Settings › Accounts.</p>
	{/if}
	{#each shown as pr (pr.id)}
		{@const state = pullRequestState(pr, me)}
		{@const checks = pr.checks ? CHECKS[pr.checks] : null}
		<button class="pr home-row" onclick={() => onopen(pr)}>
			<span class="who">
				<span class="title home-ellipsis"><span class="mono number">#{pr.number}</span> {pr.title}</span>
				<span class="meta">
					<span style:color={state.colour}>{state.text}</span> · {pr.authorName === me ? 'you' : pr.authorName} ·
					<span class="mono"><span class="added">+{pr.added}</span> <span class="removed">−{pr.removed}</span></span>
				</span>
			</span>
			{#if checks}<span class="checks" title={checks.title} style:--dot={checks.colour}></span>{/if}
		</button>
	{:else}
		{#if connected && !error}<p class="home-quiet">No open pull requests.</p>{/if}
	{/each}
</section>

<style>
	.pr {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		width: 100%;
		padding: 10px 0;
		border-left: 0;
		border-right: 0;
		border-bottom: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.pr:hover .title {
		color: var(--accent);
	}

	.who {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.title {
		display: block;
	}

	.number,
	.meta {
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.added {
		color: var(--ok);
	}

	.removed {
		color: var(--danger);
	}

	.checks {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: var(--dot);
		box-shadow: 0 0 0 4px color-mix(in srgb, var(--dot) 16%, transparent);
	}

	p {
		margin: 0;
	}
</style>
