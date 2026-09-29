<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import { ELBOW_RADIUS, LANE_PITCH, LANE_X0, NODE_R, ROW_PITCH } from '$lib/metrics';
	import { stash } from '$lib/stash/store.svelte';

	/**
	 * Stash entries, each drawn hanging off the commit it was made on.
	 *
	 * The lane is two rows per entry: the stash sits in lane 1 with the commit it
	 * came from in lane 0 below it, joined by an elbow — the same shape the graph
	 * draws for a branch, because that is exactly what a stash is.
	 *
	 * Drawn with the graph's metrics but not its canvas. The canvas exists to keep
	 * scrolling flat across a hundred thousand rows; a stash list is a dozen, and
	 * a handful of SVG paths is the smaller thing that reads the same.
	 */

	const entries = $derived(stash.entries);

	/**
	 * What you wrote, without the prefix git puts on every stash (TASK-046).
	 *
	 * git stores `On <branch>: <message>`, or `WIP on <branch>: <sha> <subject>`
	 * when there was no message. Shown whole, every row began with the same
	 * branch name and the part that tells entries apart was cut off.
	 */
	function note(message: string): string {
		const on = /^On [^:]+: (.*)$/s.exec(message);
		const written = on ? on[1] : message;
		// git's own "WIP on <branch>: …", and the same words when they were
		// given as the message — the default Spagitty's Stash button writes.
		return /^WIP on /.test(written) ? 'Work in progress' : written;
	}

	/** The lane column is two lanes wide plus the node's own radius. */
	const width = LANE_X0 + LANE_PITCH + NODE_R * 2;
	const height = ROW_PITCH * 2;

	const stashX = LANE_X0 + LANE_PITCH;
	const baseX = LANE_X0;
	const stashY = ROW_PITCH / 2;
	const baseY = ROW_PITCH + ROW_PITCH / 2;

	/**
	 * The graph's elbow, which is a rounded right angle rather than a curve
	 * (FEAT-053): down, a quarter turn, across, a quarter turn, down. Drawn with
	 * two arcs so this list and the graph draw a stash the same way.
	 */
	const corner = Math.min(ELBOW_RADIUS, Math.abs(baseX - stashX) / 2, ROW_PITCH / 2);
	const middle = (stashY + baseY) / 2;
	const elbow = [
		`M ${stashX} ${stashY}`,
		`L ${stashX} ${middle - corner}`,
		`A ${corner} ${corner} 0 0 0 ${stashX - corner} ${middle}`,
		`L ${baseX + corner} ${middle}`,
		`A ${corner} ${corner} 0 0 1 ${baseX} ${middle + corner}`,
		`L ${baseX} ${baseY}`
	].join(' ');
</script>

<nav class="list" aria-label="Stash entries">
	{#each entries as entry (entry.id)}
		<button
			class="entry"
			class:selected={stash.selected?.id === entry.id}
			onclick={() => stash.select(entry.id)}
		>
			<svg class="lane" {width} {height} aria-hidden="true" viewBox="0 0 {width} {height}">
				<path d={elbow} fill="none" stroke="var(--lane-2)" stroke-width="2" />
				<circle cx={stashX} cy={stashY} r={NODE_R} fill="var(--lane-2)" />
				<circle cx={baseX} cy={baseY} r={NODE_R - 2} fill="var(--lane-1)" />
			</svg>

			<!--
				Three quiet lines rather than one crowded one (TASK-046): what you
				wrote, then which entry and when, then the commit it hangs off.
			-->
			<div class="text">
				<span class="message" title={entry.message}>{note(entry.message)}</span>
				<span class="meta">
					<span class="mono">{entry.name}</span> · {relativeTime(entry.time)}
				</span>
				<span class="base" title={entry.parentSummary}>
					on <span class="mono">{entry.parentShort}</span>
					{entry.parentSummary}
				</span>
			</div>
		</button>
	{/each}

	{#if entries.length === 0}
		<div class="empty note">
			<p>Nothing is stashed.</p>
			<p>
				A stash puts your uncommitted work aside so you can do something else, and
				keeps it until you bring it back.
			</p>
		</div>
	{/if}
</nav>

<style>
	/* A fixed column since FEAT-034 put a diff pane beside it: two flexible
	   columns next to each other leave the divider between them with nothing
	   to mean. */
	.list {
		width: var(--stash-entries-w);
		flex: none;
		min-width: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px 6px;
	}

	/* A rounded row, not a band ruled off from the next (TASK-046). */
	.entry {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--r-button);
		text-align: left;
		width: 100%;
		transition: background var(--t-fast) var(--ease);
	}

	.entry:hover {
		background: var(--hover);
	}

	.entry.selected {
		background: var(--selection);
	}

	.lane {
		flex: none;
	}

	.text {
		display: flex;
		flex-direction: column;
		gap: 1px;
		min-width: 0;
	}

	.message,
	.meta,
	.base {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.message {
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--ink);
	}

	.meta,
	.base {
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.empty {
		padding: 14px 12px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 460px;
	}

	.empty p {
		margin: 0;
	}
</style>
