<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Markdown from './Markdown.svelte';
	import type { ObservedState, ReviewStatusData } from './types';

	/**
	 * The `reviewStatus` renderer (FEAT-096, used by FEAT-099): what a service
	 * said about a change, as typed data. "Not observed" is its own state and
	 * never drawn as a pass — silence is not approval.
	 */
	interface Props {
		data: ReviewStatusData;
	}

	let { data }: Props = $props();

	const LABELS: Record<ObservedState, string> = {
		notObserved: 'No review seen',
		requested: 'Review requested',
		running: 'Reviewing',
		completed: 'Reviewed',
		stale: 'Reviewed an older revision',
		unavailable: 'Unavailable'
	};
	const state = $derived(LABELS[data.state] ? data.state : 'unavailable');
</script>

<div class="status" aria-label="Review status">
	<div class="line">
		<span class="state state-{state}">{LABELS[state]}</span>
		{#if data.revision}<span class="mono muted" title={data.revision}>{data.revision.slice(0, 8)}</span>{/if}
		{#if data.complete === false}<span class="muted">Not everything could be read</span>{/if}
	</div>
	{#if data.headline}<p class="headline">{data.headline}</p>{/if}
	{#if data.summary}<Markdown source={data.summary} />{/if}

	{#if data.items?.length}
		<ul class="items">
			{#each data.items as item, index (index)}
				<li class="item">
					<div class="line">
						<span class="kind">{item.kind}</span>
						{#if item.title}<span class="title">{item.title}</span>{/if}
						{#if item.state}<span class="muted">{item.state}</span>{/if}
					</div>
					<div class="line muted">
						{#if item.author}<span>{item.author}{item.authorType === 'bot' ? ' (bot)' : ''}</span>{/if}
						{#if item.path}<span class="mono">{item.path}{item.line ? `:${item.line}` : ''}</span>{/if}
					</div>
					{#if item.body}
						<details>
							<summary>Details</summary>
							<Markdown source={item.body} />
						</details>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}

	{#if data.links?.length}
		<Markdown source={data.links.map((l) => `[${l.title}](${l.url})`).join(' · ')} compact />
	{/if}
</div>

<style>
	.status,
	.items {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.items {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.item {
		padding: 6px 8px;
		border-radius: var(--r-field);
		background: var(--surface);
		border: 1px solid var(--line);
	}

	.line {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.state {
		font-weight: 600;
	}

	.state-completed {
		color: var(--ok);
	}

	.state-stale,
	.state-unavailable {
		color: var(--warn);
	}

	.kind {
		font-size: var(--fs-secondary);
		text-transform: capitalize;
		color: var(--muted);
	}

	.title {
		font-weight: 550;
		overflow-wrap: anywhere;
	}

	.headline {
		margin: 0;
	}

	.muted {
		color: var(--muted);
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	details summary {
		cursor: pointer;
		color: var(--muted);
	}
</style>
