<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import type { PullRequest } from '$lib/types';
	import { age } from './needs';
	import { branchTag, driftBar } from './shape';
	import type { RecentBranch } from './types';

	/**
	 * Branches (FEAT-140): the five most recently touched, each with what it is
	 * and a two-sided bar against the default branch.
	 */
	interface Props {
		branches: RecentBranch[];
		count: number;
		defaultBranch: string | null;
		prs: PullRequest[];
		failing: Set<string>;
		now: number;
	}

	let { branches, count, defaultBranch, prs, failing, now }: Props = $props();

	const widest = $derived(Math.max(1, ...branches.map((b) => Math.max(b.ahead ?? 0, b.behind ?? 0))));
</script>

<section class="card home-card" aria-labelledby="home-branches">
	<div class="home-card-head">
		<h2 id="home-branches">Branches</h2>
		<button class="home-link" onclick={() => goto('/branches')}>All {count}</button>
	</div>
	{#each branches as branch (branch.name)}
		{@const tag = branchTag(branch, prs, failing, defaultBranch)}
		{@const bar = driftBar(branch, widest)}
		{@const against = branch.name === defaultBranch ? 'its upstream' : (defaultBranch ?? 'the default branch')}
		<div class="branch home-row">
			<div class="who">
				<span class="name mono home-ellipsis">{branch.name}</span>
				<span class="meta home-ellipsis">
					{#if tag}<span style:color={tag.colour}>{tag.text}</span> · {/if}{branch.authorName} · {age(branch.time * 1000, now)}
				</span>
			</div>
			<div
				class="drift"
				title={branch.ahead === null ? `Nothing to compare with` : `${branch.ahead} ahead, ${branch.behind} behind ${against}`}
			>
				<div class="bar">
					<div class="half left"><span style:width="{bar.behind}%"></span></div>
					<span class="mid"></span>
					<div class="half"><span class="ahead" style:width="{bar.ahead}%"></span></div>
				</div>
				<div class="numbers mono">
					<span>↓{branch.behind ?? '–'}</span><span class="up">↑{branch.ahead ?? '–'}</span>
				</div>
			</div>
		</div>
	{:else}
		<p class="home-quiet">No branches yet.</p>
	{/each}
</section>

<style>
	.branch {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 76px;
		align-items: center;
		gap: 12px;
		padding: 10px 0;
	}

	.who {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.name {
		font-size: var(--fs-secondary);
	}

	.meta {
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.drift {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.bar {
		display: flex;
		align-items: center;
		height: 6px;
	}

	.half {
		flex: 1;
		display: flex;
	}

	.half.left {
		justify-content: flex-end;
	}

	.half span {
		height: 6px;
		border-radius: 3px 0 0 3px;
		background: color-mix(in srgb, var(--ink) 40%, transparent);
	}

	.half .ahead {
		border-radius: 0 3px 3px 0;
		background: var(--ok);
	}

	.mid {
		width: 2px;
		height: 10px;
		background: color-mix(in srgb, var(--ink) 35%, transparent);
	}

	.numbers {
		display: flex;
		justify-content: space-between;
		color: var(--muted);
		font-size: calc(var(--fs-mono) * 0.92);
	}

	.up {
		color: var(--ok);
	}

	p {
		margin: 0;
	}
</style>
