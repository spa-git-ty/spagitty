<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { PIPELINE_CARD_HEIGHT as H, PIPELINE_CARD_WIDTH as W } from '$lib/metrics';
	import { layout } from './layout';
	import type { Job } from './types';
	let { jobs, stages = [], selected = '', onselect }: { jobs: Job[]; stages?: string[]; selected?: string; onselect: (id: string) => void } = $props();
	let mode = $state<'needs' | 'stages'>('needs'), expanded = $state(new Set<string>()), collapsed = $state(false);
	let left = $state(0), top = $state(0), width = $state(1000), height = $state(600);
	let previous = '';
	$effect(() => { const key = jobs.map((j) => j.id).join('|'); if (key !== previous) { previous = key; expanded = new Set(); collapsed = jobs.length > 200; } });
	const graph = $derived(layout(jobs, stages, mode, expanded, collapsed));
	const cards = $derived(graph.cards.filter((c) => c.x + W >= left - 100 && c.x <= left + (width || 1000) + 100 && c.y + H >= top - 100 && c.y <= top + (height || 600) + 100));
	const edges = $derived(graph.edges.filter((e) => Math.max(e.x1, e.x2) >= left && Math.min(e.x1, e.x2) <= left + (width || 1000) && Math.max(e.y1, e.y2) >= top && Math.min(e.y1, e.y2) <= top + (height || 600)));
	function scroll(event: Event) { const node = event.currentTarget as HTMLElement; left = node.scrollLeft; top = node.scrollTop; }
	function toggle(id: string) { const next = new Set(expanded); if (next.has(id)) next.delete(id); else next.add(id); expanded = next; }
	function select(id: string) { if (collapsed) { const group = graph.cards.find((c) => c.job.id === id)?.job; collapsed = false; const first = jobs.find((j) => (j.stage || `Depth ${j.depth + 1}`) === group?.name); if (first) onselect(first.id); } else onselect(id); }
</script>
<div class="graph-tools"><span class="note">{jobs.length} jobs</span><span class="grow"></span>{#if jobs.length > 200}<button onclick={() => { collapsed = !collapsed; }}>{collapsed ? 'Expand stages' : 'Collapse to stages'}</button>{/if}<div class="segments" aria-label="Graph layout"><button class:on={mode === 'stages'} onclick={() => { mode = 'stages'; }}>Stages</button><button class:on={mode === 'needs'} onclick={() => { mode = 'needs'; }}>Needs</button></div></div>
<div class="viewport" onscroll={scroll} bind:clientWidth={width} bind:clientHeight={height} role="region" aria-label="Pipeline graph">
	<div class="canvas" style:width={`${graph.width}px`} style:height={`${graph.height}px`}>
		{#each graph.labels as label (label.column)}<span class="column" style:left={`${label.x}px`}>{label.text}</span>{/each}
		<svg width={graph.width} height={graph.height} aria-hidden="true">{#each edges as edge (edge.key)}<path d={edge.path} class:skipped={edge.skipped} />{/each}</svg>
		{#each cards as card (card.key)}
			<div class="card" class:selected={selected === card.job.id} class:skipped={card.job.state === 'skipped'} style:left={`${card.x}px`} style:top={`${card.y}px`} style:width={`${W}px`} style:height={`${H}px`}>
				<button class="pick" onclick={() => select(card.job.id)} aria-label={`Select ${card.job.name}`}><strong><span class="dot" data-state={card.job.state} aria-hidden="true">{card.job.state === 'skipped' ? '–' : card.job.state === 'approval' ? '‖' : card.job.state === 'success' ? '✓' : '●'}</span>{card.job.name}</strong><span class="mono subtitle">{card.subtitle}</span></button>
				{#if card.count > 1}<button class="matrix" aria-label={`${expanded.has(card.job.id) ? 'Collapse' : 'Expand'} ${card.job.name} matrix`} onclick={() => toggle(card.job.id)}>×{card.count}</button>{/if}
			</div>
		{/each}
	</div>
</div>
<style>
	.graph-tools { display: flex; align-items: center; gap: 10px; padding: 8px 14px; border-bottom: 1px solid var(--soft); flex: none; }
	.grow { flex: 1; } .segments { display: flex; border: 1px solid var(--soft); border-radius: var(--r-pill); background: var(--sunken); padding: 3px; }
	.segments button { padding: 4px 12px; border-radius: var(--r-pill); font-size: var(--fs-secondary); } .on { background: var(--surface-2); }
	.viewport { overflow: auto; min-height: 180px; flex: 1; padding: 16px; } .canvas { position: relative; }
	.column { position: absolute; top: 0; text-transform: uppercase; color: var(--muted); font-size: var(--fs-secondary); }
	svg { position: absolute; inset: 0; pointer-events: none; } path { fill: none; stroke: var(--line); stroke-width: 1.6; } path.skipped { stroke-dasharray: 4 4; }
	.card { position: absolute; background: var(--surface-2); border: 1px solid var(--pane-edge); border-radius: var(--r-field); display: flex; align-items: center; }
	.card.selected { outline: 2px solid var(--accent); outline-offset: 2px; } .card.skipped { opacity: .55; border-style: dashed; }
	.pick { display: flex; flex-direction: column; gap: 2px; min-width: 0; width: 100%; padding: 7px 10px; text-align: left; }
	strong { display: flex; align-items: center; gap: 6px; font-size: var(--fs-secondary); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
	.subtitle { display: block; width: 100%; font-family: var(--font-mono); font-size: var(--fs-mono); color: var(--muted); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
	.dot { color: var(--muted); } .dot[data-state="runs"], .dot[data-state="success"] { color: var(--ok); } .dot[data-state="approval"] { color: var(--warn); } .dot[data-state="running"] { color: var(--lane-5); } .dot[data-state="failure"], .dot[data-state="failed"] { color: var(--danger); }
	.matrix { flex: none; padding: 3px; font-family: var(--font-mono); font-size: var(--fs-mono); }
</style>
