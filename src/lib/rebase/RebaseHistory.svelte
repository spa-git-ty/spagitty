<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { PreviewRow } from '$lib/types';

	/**
	 * History after (FEAT-106): the branch as the plan leaves it, drawn from
	 * the preview rather than sketched — the branch it lands on at the foot,
	 * each new commit above it in the order it is made, a folded commit with
	 * how many it took in, a reworded one ringed in its colour, and the ones
	 * that may stop on a conflict marked. What the plan drops is listed below.
	 */
	interface Props {
		rows: PreviewRow[];
		dropped: { short: string; summary: string }[];
		onto: string;
		ontoShort: string;
		branch: string;
	}

	let { rows, dropped, onto, ontoShort, branch }: Props = $props();

	/** Past this many, the oldest are summed up rather than drawn. */
	const SHOWN = 8;
	const PITCH = 34;
	const X = 30;
	const WIDTH = 320;

	const hidden = $derived(Math.max(0, rows.length - SHOWN));
	const drawn = $derived(rows.slice(hidden));
	/** One slot for the summed-up oldest, when there are any. */
	const slots = $derived(drawn.length + (hidden > 0 ? 1 : 0));
	const height = $derived(slots * PITCH + 64);
	const base = $derived(height - 26);
	const yOf = (slot: number) => base - (slot + 1) * PITCH;
	const first = $derived(hidden > 0 ? 1 : 0);
	const top = $derived(drawn.length > 0 ? yOf(first + drawn.length - 1) : base);

	const clip = (text: string, length = 30) =>
		text.length > length ? `${text.slice(0, length - 1)}…` : text;
</script>

<div class="history">
	<svg
		viewBox="0 0 {WIDTH} {height}"
		width="100%"
		style="max-width: {WIDTH}px"
		role="img"
		aria-label="History of {branch} after the rebase: {rows.length} commits on {onto}"
	>
		<path d="M{X} {base + 22}V{base}" stroke="var(--side-a)" stroke-width="2.5" fill="none"></path>
		{#if drawn.length > 0}
			<path d="M{X} {base}V{top}" stroke="var(--ok)" stroke-width="2.5" fill="none"></path>
		{/if}

		<circle cx={X} cy={base} r="7" fill="var(--side-a)"></circle>
		<text x={X + 18} y={base + 4} class="label onto">onto {clip(onto, 22)} · {ontoShort}</text>

		{#if hidden > 0}
			<text x={X + 18} y={yOf(0) + 4} class="label muted">+ {hidden} earlier</text>
			<circle cx={X} cy={yOf(0)} r="3" fill="var(--ok)"></circle>
		{/if}

		{#each drawn as row, index (row.id)}
			{@const y = yOf(first + index)}
			{@const folded = row.absorbed.length > 0}
			{#if row.mayConflict}
				<circle cx={X} cy={y} r={folded ? 12.5 : 10.5} class="risk"></circle>
			{/if}
			<circle
				cx={X}
				cy={y}
				r={folded ? 8.5 : 6.5}
				fill="var(--bg)"
				stroke={row.reworded ? 'var(--lane-4)' : folded ? 'var(--side-b)' : 'var(--ok)'}
				stroke-width="2.6"
			></circle>
			{#if folded}
				<text x={X} y={y + 3.5} class="count" text-anchor="middle">{row.absorbed.length + 1}</text>
			{/if}
			<text x={X + 18} y={y + 4} class="label">
				<tspan class="sha">{row.short}</tspan>
				<tspan dx="6">{clip(row.summary)}</tspan>
			</text>
		{/each}

		{#if drawn.length > 0}
			<text x={X + 18} y={top - 18} class="label branch">{clip(branch, 34)}</text>
		{/if}
	</svg>

	{#if dropped.length > 0}
		<p class="dropped note">
			<span class="mark" aria-hidden="true"></span>
			{dropped.length} dropped:
			{#each dropped as commit, index (commit.short)}<span class="mono" title={commit.summary}>{commit.short}</span>{index < dropped.length - 1 ? ', ' : ''}{/each}
		</p>
	{/if}
</div>

<style>
	.history {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	svg {
		display: block;
		overflow: visible;
	}

	.label {
		font-size: 11px;
		fill: var(--ink);
	}

	.sha {
		font-family: var(--font-mono);
		fill: var(--muted);
	}

	.onto {
		fill: var(--side-a);
		font-family: var(--font-mono);
	}

	.branch {
		fill: var(--ok);
		font-family: var(--font-mono);
		font-weight: 650;
	}

	.muted {
		fill: var(--muted);
	}

	.count {
		font-size: 9.5px;
		font-weight: 700;
		fill: var(--side-b);
	}

	.risk {
		fill: none;
		stroke: var(--danger);
		stroke-width: 1.5;
		stroke-dasharray: 3 3;
	}

	.dropped {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
		color: var(--danger);
	}

	.mark {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		border: 1.5px dashed var(--danger);
	}
</style>
