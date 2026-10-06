<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import * as api from '$lib/api';
	import { byName, timing, type Summary } from '$lib/timing.svelte';
	import type { CommandTiming } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * Timings (TASK-052): what the application spends its time on, readable
	 * without devtools.
	 *
	 * - **Waiting for the repository**: each command's wait for the open
	 *   repository and how long it then held it, from the backend. A wait is
	 *   time spent behind another command.
	 * - **Calls**: each call from the screen to the backend, round trip.
	 * - **Screens and diffs**: navigating until the screen is painted, and a
	 *   diff from its file arriving until it is painted.
	 *
	 * Kept since the panel opened or was last cleared, worst first.
	 */

	/** How often the backend's half is read while the panel is open. */
	const EVERY_MS = 2000;
	/** Rows per table: the worst, which is what is being looked for. */
	const ROWS = 12;

	let holds = $state.raw<CommandTiming[]>([]);
	let last = 0;
	let error = $state<string | null>(null);

	async function read() {
		if (!api.inTauri()) return;
		try {
			const fresh = await api.commandTimings(last);
			if (fresh.length === 0) return;
			last = fresh[fresh.length - 1].seq;
			holds = [...holds, ...fresh].slice(-2000);
			error = null;
		} catch (e) {
			error = String(e);
		}
	}

	$effect(() => {
		void read();
		const timer = setInterval(() => void read(), EVERY_MS);
		return () => clearInterval(timer);
	});

	const held = $derived(
		byName(
			holds,
			(hold) => hold.command,
			(hold) => hold.heldUs / 1000
		).slice(0, ROWS)
	);
	const waited = $derived(
		byName(
			holds.filter((hold) => hold.waitUs >= 1000),
			(hold) => hold.command,
			(hold) => hold.waitUs / 1000
		).slice(0, ROWS)
	);
	const trips = $derived.by(() => {
		void timing.version;
		return byName(
			timing.trips(),
			(trip) => trip.name,
			(trip) => trip.ms
		).slice(0, ROWS);
	});
	const measures = $derived.by(() => {
		void timing.version;
		return timing.measures().slice(-ROWS).reverse();
	});

	function ms(value: number): string {
		if (value < 1) return `${value.toFixed(2)} ms`;
		if (value < 10) return `${value.toFixed(1)} ms`;
		return `${Math.round(value).toLocaleString()} ms`;
	}

	function clear() {
		holds = [];
		timing.clear();
	}
</script>

{#snippet table(rows: Summary[], empty: string)}
	{#if rows.length === 0}
		<p class="note">{empty}</p>
	{:else}
		<table>
			<thead>
				<tr><th></th><th>count</th><th>median</th><th>95%</th><th>worst</th></tr>
			</thead>
			<tbody>
				{#each rows as row (row.name)}
					<tr>
						<td class="mono name">{row.name}</td>
						<td>{row.count}</td>
						<td>{ms(row.p50)}</td>
						<td>{ms(row.p95)}</td>
						<td class:slow={row.max >= 100}>{ms(row.max)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
{/snippet}

<div class="timings">
	<div class="row">
		<Btn onclick={clear}>Clear</Btn>
		{#if error}<span class="note error" title={error}>The backend's timings could not be read.</span>{/if}
	</div>

	<h4 class="label">Holding the repository</h4>
	{@render table(held, 'Nothing has held the repository yet.')}

	<h4 class="label">Waiting for it</h4>
	{@render table(waited, 'Nothing has waited a millisecond or more.')}

	<h4 class="label">Calls, round trip</h4>
	{@render table(trips, 'No calls yet.')}

	<h4 class="label">Screens and diffs, until painted</h4>
	{#if measures.length === 0}
		<p class="note">Open a screen or a diff.</p>
	{:else}
		<table>
			<tbody>
				{#each measures as measure (measure.at)}
					<tr>
						<td>{measure.kind === 'navigate' ? 'screen' : 'diff'}</td>
						<td class="mono name">{measure.label}</td>
						<td>{measure.size === undefined ? '' : `${measure.size.toLocaleString()} lines`}</td>
						<td class:slow={measure.ms >= 100}>{ms(measure.ms)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

<style>
	.timings {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.label {
		margin: 6px 0 0;
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	table {
		border-collapse: collapse;
		font-size: var(--fs-secondary);
		font-variant-numeric: tabular-nums;
	}

	th {
		text-align: right;
		font-weight: 400;
		color: var(--muted);
		padding: 2px 10px;
	}

	td {
		text-align: right;
		padding: 2px 10px;
		white-space: nowrap;
	}

	td.name {
		text-align: left;
		max-width: 260px;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.slow {
		color: var(--warn);
	}

	.error {
		color: var(--danger);
	}
</style>
