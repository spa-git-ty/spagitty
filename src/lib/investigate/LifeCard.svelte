<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { byOf, dayOf, factsOf, layout, targetLabel } from './chart';
	import { investigate } from './store.svelte';

	/**
	 * A life at a glance (FEAT-132): one line of facts, the size over time
	 * with a marker on each born, renamed, moved and rewrite, and a row of
	 * ticks for each author — agents in the agent's colour.
	 *
	 * The geometry is `chart.ts`'s. This draws it.
	 */
	const target = $derived(investigate.target);
	const kind = $derived(target?.kind ?? 'file');
	const entries = $derived(investigate.entries);
	const facts = $derived(
		factsOf(kind, entries, investigate.done, investigate.today, investigate.branch)
	);
	const drawn = $derived(layout(kind, entries));

	const plural = (count: number, one: string, many = `${one}s`) => (count === 1 ? one : many);
</script>

{#if target}
	<section class="card life" aria-label="Life of {targetLabel(target)}">
		<div class="facts">
			<span>
				<span class="note">Born</span>{' '}
				{#if facts.born}
					<b>{dayOf(facts.born.time)}</b>
					<span class="note">· {byOf(facts.born)} ·</span>
					<span class="mono">{facts.born.short}</span>
					{#if facts.born.note}<span class="note">· {facts.born.note}</span>{/if}
				{:else if investigate.reading}
					<Loader size="inline" label="Reading…" />
				{:else if investigate.done}
					<!-- Everything in the range is read, and its birth is not in it. -->
					<span class="note">before this range</span>
				{:else}
					<span class="note">before what has been read</span>
				{/if}
			</span>
			<span><b>{facts.commits}</b>{' '}<span class="note"> {plural(facts.commits, 'commit')}</span></span>
			<span><b>{facts.authors}</b>{' '}<span class="note"> {plural(facts.authors, 'author')}</span></span>
			{#if kind === 'file'}
				<span><b>{facts.renames}</b>{' '}<span class="note"> {plural(facts.renames, 'rename')}</span></span>
			{/if}
			{#if kind === 'folder' && facts.files !== null}
				<span><b>{facts.files}</b>{' '}<span class="note"> {plural(facts.files, 'file')} today</span></span>
			{/if}
			{#if facts.now !== null && kind !== 'branch'}
				<span>
					<b>{facts.now}</b>{' '}<span class="note">
						{plural(facts.now, 'line')} today{#if facts.peak !== null}&nbsp;· peak {facts.peak}{/if}</span
					>
				</span>
			{/if}
			{#if facts.against && facts.ahead !== null && facts.behind !== null}
				<span>
					<b>{facts.ahead}</b>{' '}<span class="note">ahead of {facts.against} ·</span>{' '}<b>{facts.behind}</b>{' '}<span class="note">behind</span>
				</span>
			{/if}
		</div>

		{#if entries.length > 0}
			<div class="grid">
				{#if drawn.line}
					<span class="label">{facts.unit === 'commits' ? 'Commits' : 'Lines'}</span>
					<div class="plot">
						<svg viewBox="0 0 1000 120" preserveAspectRatio="none" role="img" aria-label={drawn.said}>
							<path class="area" d={drawn.area} />
							<path class="line" d={drawn.line} vector-effect="non-scaling-stroke" />
						</svg>
						{#each drawn.marks as mark, i (i)}
							<div class="mark" style:left="{mark.x * 100}%" style:--ink={mark.ink}>
								<span class:left={mark.x > 0.8}>{mark.label}</span>
							</div>
						{/each}
					</div>
				{/if}

				{#each drawn.rows as row (row.name)}
					<span class="label who" style:color={row.ink}>
						<span class="dot" aria-hidden="true"></span>
						{#if row.agent}<Icon name="agent" size={11} weight={2.2} />{/if}
						<span class="name">{row.name}</span>
					</span>
					<div class="track" style:--ink={row.ink} aria-label="{row.name}: {row.ticks.length} commits">
						{#each row.ticks as tick, i (i)}
							<span style:left="{tick * 100}%"></span>
						{/each}
					</div>
				{/each}
				{#if drawn.others > 0}
					<span class="label note">+ {drawn.others} more</span>
					<span></span>
				{/if}

				<span></span>
				<div class="axis">
					{#each drawn.axis as label, i (i)}<span>{label}</span>{/each}
				</div>
			</div>
		{/if}
	</section>
{/if}

<style>
	.life {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px 18px 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	.facts {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 6px 18px;
	}

	/* A number and what it counts, a space apart whatever the markup wraps to. */
	.facts > span {
		display: inline-flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0 0.3em;
	}

	.facts b {
		font-weight: 600;
	}

	.grid {
		display: grid;
		grid-template-columns: 110px minmax(0, 1fr);
		column-gap: 12px;
		row-gap: 6px;
		align-items: center;
	}

	.label {
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.plot {
		position: relative;
		height: 120px;
	}

	.plot svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		display: block;
	}

	.area {
		fill: color-mix(in srgb, var(--accent) 16%, transparent);
	}

	.line {
		fill: none;
		stroke: var(--accent);
		stroke-width: 2;
	}

	.mark {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 0;
		border-left: 1px dashed var(--ink);
	}

	.mark span {
		position: absolute;
		top: 2px;
		left: 6px;
		padding: 1px 7px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-pill);
		background: var(--surface-2);
		color: var(--ink);
		font-size: calc(var(--fs-mono) * 0.96);
		white-space: nowrap;
	}

	/* Near the right edge the label goes to the other side of its line. */
	.mark span.left {
		left: auto;
		right: 6px;
	}

	.who {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}

	.who .name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.dot {
		flex: none;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: currentColor;
	}

	.track {
		position: relative;
		height: 14px;
		border-radius: var(--r-pill);
		background: var(--soft);
	}

	.track span {
		position: absolute;
		top: 3px;
		width: 8px;
		height: 8px;
		margin-left: -4px;
		border-radius: 50%;
		background: var(--ink);
	}

	.axis {
		display: flex;
		justify-content: space-between;
		font-size: calc(var(--fs-mono) * 0.96);
		color: var(--muted);
	}
</style>
