<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { PIPELINE_CARD_HEIGHT as H, PIPELINE_CARD_WIDTH as W } from '$lib/metrics';
	import { layout } from './layout';
	import { clock, mark, tone } from './states';
	import type { Job } from './types';

	interface Props {
		jobs: Job[];
		stages?: string[];
		selected?: string;
		/** Owned by the screen, which draws the Stages / Needs switch in its own bar. */
		mode?: 'needs' | 'stages';
		onselect: (id: string) => void;
		/** Under the cards, inside the scrolling area: the legend. */
		footer?: Snippet;
	}

	let { jobs, stages = [], selected = '', mode = 'needs', onselect, footer }: Props = $props();

	let expanded = $state(new Set<string>()), collapsed = $state(false);
	let left = $state(0), top = $state(0), width = $state(1000), height = $state(600);
	let previous = '';
	$effect(() => {
		const key = jobs.map((j) => j.id).join('|');
		if (key !== previous) { previous = key; expanded = new Set(); collapsed = jobs.length > 200; }
	});

	const graph = $derived(layout(jobs, stages, mode, expanded, collapsed));
	// Cards and edges are placed once and only the visible ones are drawn.
	const cards = $derived(graph.cards.filter((c) => c.x + W >= left - 100 && c.x <= left + (width || 1000) + 100 && c.y + H >= top - 100 && c.y <= top + (height || 600) + 100));
	const edges = $derived(graph.edges.filter((e) => Math.max(e.x1, e.x2) >= left && Math.min(e.x1, e.x2) <= left + (width || 1000) && Math.max(e.y1, e.y2) >= top && Math.min(e.y1, e.y2) <= top + (height || 600)));

	function scroll(event: Event) {
		const node = event.currentTarget as HTMLElement;
		left = node.scrollLeft;
		top = node.scrollTop;
	}

	function toggle(id: string) {
		const next = new Set(expanded);
		if (next.has(id)) next.delete(id); else next.add(id);
		expanded = next;
	}

	function select(id: string) {
		if (!collapsed) return onselect(id);
		const group = graph.cards.find((c) => c.job.id === id)?.job;
		collapsed = false;
		const first = jobs.find((j) => (j.stage || `Depth ${j.depth + 1}`) === group?.name);
		if (first) onselect(first.id);
	}
</script>

<div class="viewport" onscroll={scroll} bind:clientWidth={width} bind:clientHeight={height} role="region" aria-label="Pipeline graph">
	{#if jobs.length > 200}
		<button class="collapse" onclick={() => { collapsed = !collapsed; }}>{collapsed ? 'Expand stages' : 'Collapse to stages'}</button>
	{/if}
	<div class="canvas" style:width={`${graph.width}px`} style:height={`${graph.height}px`}>
		{#each graph.labels as label (label.column)}
			<span class="column" style:left={`${label.x}px`} style:width={`${W}px`}>{label.text}</span>
		{/each}
		<svg width={graph.width} height={graph.height} aria-hidden="true">
			{#each edges as edge (edge.key)}<path d={edge.path} class:skipped={edge.skipped} />{/each}
		</svg>
		{#each cards as card (card.key)}
			<div
				class="job"
				data-tone={tone(card.job.state)}
				class:selected={selected === card.job.id}
				style:left={`${card.x}px`}
				style:top={`${card.y}px`}
				style:width={`${W}px`}
				style:height={`${H}px`}
				title={card.job.reason}
			>
				<button class="pick" onclick={() => select(card.job.id)} aria-label={`Select ${card.job.name}`}>
					<span class="name">
						<span class="mark" aria-hidden="true">{mark(card.job.state)}</span>
						<span class="label">{card.job.name}</span>
						{#if card.job.duration != null}<span class="time">{clock(card.job.duration)}</span>{/if}
					</span>
					<span class="subtitle">{card.subtitle}</span>
				</button>
				{#if card.count > 1}
					<button class="matrix" aria-label={`${expanded.has(card.job.id) ? 'Collapse' : 'Expand'} ${card.job.name} matrix`} onclick={() => toggle(card.job.id)}>×{card.count}</button>
				{/if}
			</div>
		{/each}
	</div>
	{@render footer?.()}
</div>

<style>
	.viewport {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 18px 16px;
	}

	.collapse {
		position: absolute;
		top: 10px;
		right: 14px;
		z-index: 1;
		padding: 4px 12px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-pill);
		background: var(--surface-veil);
		font-size: var(--fs-secondary);
	}

	.canvas {
		position: relative;
	}

	.column {
		position: absolute;
		top: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	svg {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}

	path {
		fill: none;
		stroke: var(--line);
		stroke-width: 1.6;
	}

	path.skipped {
		stroke-dasharray: 4 4;
	}

	.job {
		position: absolute;
		box-sizing: border-box;
		display: flex;
		align-items: center;
		background: var(--surface-2);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		box-shadow: var(--shadow-1);
	}

	.job.selected {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	/* Skipped is faded and dashed, so the state does not rest on colour alone. */
	.job[data-tone='skipped'] {
		opacity: 0.55;
		border-style: dashed;
		box-shadow: none;
	}

	.job[data-tone='running'] {
		border-color: var(--lane-5);
		background: color-mix(in srgb, var(--lane-5) 8%, var(--surface-2));
	}

	.job[data-tone='failed'] {
		border-color: color-mix(in srgb, var(--danger) 55%, transparent);
	}

	.pick {
		flex: 1;
		min-width: 0;
		height: 100%;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 2px;
		padding: 7px 10px;
		text-align: left;
	}

	.name {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	.label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.mark {
		flex: none;
		width: 12px;
		text-align: center;
		color: var(--muted);
	}

	.job[data-tone='ok'] .mark { color: var(--ok); }
	.job[data-tone='waits'] .mark { color: var(--warn); }
	.job[data-tone='running'] .mark { color: var(--lane-5); }
	.job[data-tone='failed'] .mark { color: var(--danger); }

	.time {
		flex: none;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		font-weight: 400;
		color: var(--muted);
	}

	.subtitle {
		display: block;
		padding-left: 18px;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-family: var(--font-mono);
		font-size: calc(var(--fs-mono) * 0.92);
		color: var(--muted);
	}

	.matrix {
		flex: none;
		align-self: flex-start;
		margin: 6px 6px 0 0;
		padding: 1px 6px;
		border-radius: var(--r-pill);
		background: var(--sunken);
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		color: var(--muted);
	}
</style>
