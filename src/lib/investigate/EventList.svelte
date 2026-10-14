<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { fullDate } from '$lib/format';
	import type { LifeEntry } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { byOf, dayOf, inkOf, KIND_INK } from './chart';
	import { investigate, LIFE_MAX } from './store.svelte';

	/**
	 * A life's events, newest first (FEAT-132): each with its kind, who, the
	 * commit's summary and what it added and removed.
	 *
	 * A virtual list, because a file five thousand commits have touched is
	 * five thousand rows and the screen shows nine of them.
	 */
	const entries = $derived(investigate.listed);

	/** Two events of one commit — a branch's reset and its commit — are two rows. */
	const keyOf = (entry: LifeEntry) => `${entry.commit}:${entry.kind}:${entry.time}`;

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
		const at = entries.indexOf(investigate.selected as LifeEntry);
		const next = entries[at + (event.key === 'ArrowDown' ? 1 : -1)];
		if (!next) return;
		event.preventDefault();
		investigate.select(next);
		list?.scrollToIndex(entries.indexOf(next), 'nearest');
	}

	let list = $state<ReturnType<typeof VirtualRows<LifeEntry>> | null>(null);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section class="card events" aria-label="Events" {onkeydown}>
	{#if investigate.error}
		<p class="note said error">{investigate.error}</p>
	{:else if entries.length === 0 && investigate.reading}
		<Loader />
	{:else if entries.length === 0}
		<p class="note said">Nothing in this range</p>
	{:else}
		<VirtualRows bind:this={list} items={entries} key={keyOf} estimate={() => 58} label="Events, newest first">
			{#snippet row(entry: LifeEntry)}
				<button
					class="event"
					class:on={entry === investigate.selected}
					aria-pressed={entry === investigate.selected}
					title={fullDate(entry.time)}
					onclick={() => investigate.select(entry)}
				>
					<span class="date">{dayOf(entry.time)}</span>
					<span class="what">
						<span class="top">
							<span class="kind" style:color={KIND_INK[entry.kind]}>{entry.kind}</span>
							<span class="who" style:color={inkOf(entry)}>
								{#if entry.agent}<Icon name="agent" size={11} weight={2.2} />{/if}
								{byOf(entry)}
							</span>
						</span>
						<!-- A rename is read by its two names; anything else by what the commit said. -->
						<span class="summary"
							>{(entry.kind === 'renamed' || entry.kind === 'moved') && entry.note
								? entry.note
								: entry.summary}</span
						>
					</span>
					<span class="lines mono">
						<span class="plus">+{entry.added}</span> <span class="minus">−{entry.removed}</span>
					</span>
				</button>
			{/snippet}
		</VirtualRows>
		{#if investigate.reading}
			<div class="foot"><Loader size="inline" label="Reading the rest…" /></div>
		{:else if investigate.more}
			<div class="foot">
				<span class="note">The newest {LIFE_MAX.toLocaleString()} are read</span>
				<Btn onclick={() => investigate.loadMore()}>Load more</Btn>
			</div>
		{/if}
	{/if}
</section>

<style>
	.events {
		flex: 1 1 340px;
		max-width: 420px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.event {
		width: 100%;
		display: flex;
		align-items: flex-start;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--r-button);
		text-align: left;
	}

	.event:hover {
		background: var(--hover);
	}

	.event.on {
		background: var(--surface-2);
		box-shadow: var(--shadow-1);
	}

	.date {
		flex: none;
		width: 66px;
		padding-top: 2px;
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.what {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.top {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.kind {
		flex: none;
		padding: 0 7px;
		border: 1px solid currentColor;
		border-radius: var(--r-pill);
		font-size: calc(var(--fs-mono) * 0.96);
	}

	.who {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-mono);
	}

	.summary {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
	}

	.lines {
		flex: none;
		padding-top: 2px;
		font-size: calc(var(--fs-mono) * 0.96);
		white-space: nowrap;
	}

	.plus {
		color: var(--ok);
	}

	.minus {
		color: var(--danger);
	}

	.foot {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 6px 8px 2px;
	}

	.said {
		margin: auto;
		padding: 18px;
		text-align: center;
	}

	.error {
		color: var(--danger);
	}
</style>
