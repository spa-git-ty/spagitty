<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Chip from '$lib/ui/Chip.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import { farmStore } from '../store.svelte';
	import {
		FARM_COPY as C,
		activityLine,
		agentColour,
		checksFailedAfter,
		clockTime,
		elapsed,
		eventAgent,
		eventColour,
		eventTask,
		filterEvents,
		type ActivityFilter
	} from '../describe';
	import type { AgentRun } from '../types';

	/**
	 * Who did what, and when (FEAT-112).
	 *
	 * The timeline draws the runs on record — one row per agent, from start to
	 * end or to now — with the checks and merges as markers on a last row. The
	 * log under it is the same record as sentences, newest first. Nothing is
	 * inferred that the record does not say.
	 */
	interface Props {
		now: number;
		onopen: (id: string) => void;
	}

	let { now, onopen }: Props = $props();

	const FILTERS: ActivityFilter[] = ['All', 'Needs you', 'Landed', 'Failures'];
	const TICKS = [0, 0.25, 0.5, 0.75, 1];

	let filter = $state<ActivityFilter>('All');

	const runs = $derived(farmStore.runs);
	const events = $derived(farmStore.activity.filter((e) => e.atMs > 0));

	/** The window: from the first thing on record to now, a minute at least. */
	const start = $derived(
		Math.min(now - 60_000, ...runs.map((r) => r.startedMs), ...events.map((e) => e.atMs))
	);
	const end = $derived(Math.max(now, ...runs.map((r) => r.endedMs ?? r.startedMs)));
	const span = $derived(Math.max(60_000, end - start));

	const rows = $derived(
		farmStore.agents.filter(
			(a) => a.definition.enabled || runs.some((r) => r.agent === a.definition.id)
		)
	);
	const markers = $derived(
		events.filter((e) => e.kind === 'verificationFinished' || e.kind === 'mergeCompleted')
	);
	const log = $derived(filterEvents(farmStore.activity, filter, farmStore.needsYou));

	/** A moment as a percentage across the timeline. */
	function at(ms: number): number {
		return Math.max(0, Math.min(100, ((ms - start) / span) * 100));
	}

	function label(run: AgentRun): string {
		if (run.task === 'planning') return 'plan';
		const title = farmStore.byId.get(run.task)?.title;
		const name = run.phase === 'review' ? `review ${run.task}` : run.task;
		return title && run.phase !== 'review' ? `${name} ${title}` : name;
	}

	function title(run: AgentRun): string {
		const until = run.endedMs ? clockTime(run.endedMs) : 'now';
		return `${run.phase} · ${run.task} · ${clockTime(run.startedMs)}–${until}`;
	}
</script>

<div class="activity">
	<section class="card timeline">
		<div class="spread">
			<h3>The last {elapsed(span)}</h3>
			<div class="legend muted">
				<span><i class="swatch writing"></i>writing</span>
				<span><i class="swatch reviewing"></i>reviewing</span>
				<span><i class="swatch failed"></i>checks failed</span>
				<span><i class="dot landed"></i>landed</span>
			</div>
		</div>

		{#each rows as agent (agent.definition.id)}
			<div class="row">
				<span class="who"
					><AgentBadge agent={agent.definition} />{agent.definition.displayName}</span
				>
				<div class="lane">
					{#each runs.filter((r) => r.agent === agent.definition.id) as run (run.id)}
						<button
							class="bar"
							class:dashed={run.phase !== 'implementation'}
							class:failed={checksFailedAfter(run, runs, events)}
							style:left="{at(run.startedMs)}%"
							style:width="{Math.max(0.5, at(run.endedMs ?? end) - at(run.startedMs))}%"
							style:--agent-colour={agentColour(agent.definition)}
							title={title(run)}
							onclick={() => run.task !== 'planning' && onopen(run.task)}
						>
							{label(run)}
						</button>
					{/each}
				</div>
			</div>
		{/each}

		<div class="row">
			<span class="who muted">Checks · merges</span>
			<div class="lane">
				{#each markers as event, i (i)}
					{@const id = eventTask(event)}
					<button
						class="marker"
						style:left="{at(event.atMs)}%"
						style:background={eventColour(event)}
						title={activityLine(event, farmStore.agents)}
						aria-label={activityLine(event, farmStore.agents)}
						onclick={() => id && onopen(id)}
					></button>
				{/each}
			</div>
		</div>

		<div class="row">
			<span></span>
			<div class="times mono">
				{#each TICKS as fraction (fraction)}
					<span class:now={fraction === 1}
						>{fraction === 1 ? 'now' : clockTime(start + span * fraction)}</span
					>
				{/each}
			</div>
		</div>
	</section>

	<section class="section">
		<div class="spread log-head">
			<div class="inline">
				<h3>{C.whatHappened}</h3>
				{#each FILTERS as name (name)}
					<Chip active={filter === name} onclick={() => (filter = name)}>{name}</Chip>
				{/each}
			</div>
			<span class="muted">{C.newestFirst}</span>
		</div>
		<div class="card log">
			{#each log as event, i (i)}
				{@const id = eventTask(event)}
				{@const agent = farmStore.agents.find((a) => a.definition.id === eventAgent(event, runs))}
				<div class="event">
					<time class="mono muted">{clockTime(event.atMs)}</time>
					<AgentBadge agent={agent?.definition} />
					<span class="dot" style:background={eventColour(event)}></span>
					<span class="sentence">{activityLine(event, farmStore.agents)}</span>
					{#if id}<Chip onclick={() => onopen(id)}>{id}</Chip>{/if}
				</div>
			{:else}
				<p class="muted empty-log">{C.noActivity}</p>
			{/each}
		</div>
	</section>
</div>

<style>
	.activity {
		padding-bottom: 20px;
	}

	.timeline h3 {
		margin: 0;
	}

	.legend {
		display: flex;
		gap: 14px;
	}

	.legend span {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.swatch {
		width: 18px;
		height: 10px;
		border-radius: 3px;
	}

	.swatch.writing {
		background: color-mix(in srgb, var(--muted) 35%, transparent);
		border: 1px solid var(--muted);
	}

	.swatch.reviewing {
		border: 1px dashed var(--muted);
	}

	.swatch.failed {
		border: 1px solid var(--danger);
		background: repeating-linear-gradient(
			135deg,
			color-mix(in srgb, var(--danger) 35%, transparent) 0 3px,
			transparent 3px 6px
		);
	}

	.dot.landed {
		background: var(--ok);
	}

	.row {
		display: grid;
		grid-template-columns: 160px minmax(0, 1fr);
		align-items: center;
		gap: 12px;
		margin: 14px 0;
	}

	.who {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	/* Light rules mark the quarters; the accent edge is now. */
	.lane {
		position: relative;
		height: 34px;
		background: repeating-linear-gradient(
			90deg,
			transparent,
			transparent calc(25% - 1px),
			var(--soft) calc(25% - 1px),
			var(--soft) 25%
		);
		border-right: 2px solid var(--accent);
	}

	.bar {
		position: absolute;
		top: 4px;
		height: 26px;
		padding: 0 7px;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		text-align: left;
		color: var(--ink);
		font: var(--fs-mono) var(--font-mono);
		border: 1px solid var(--agent-colour);
		border-radius: var(--r-row);
		background: color-mix(in srgb, var(--agent-colour) 23%, transparent);
		cursor: pointer;
	}

	.bar.dashed {
		border-style: dashed;
		background: none;
	}

	.bar.failed {
		border-color: var(--danger);
		background: repeating-linear-gradient(
			135deg,
			color-mix(in srgb, var(--danger) 25%, transparent) 0 5px,
			transparent 5px 10px
		);
	}

	.marker {
		position: absolute;
		top: 10px;
		width: 14px;
		height: 14px;
		border: 2px solid var(--surface);
		border-radius: 50%;
		transform: translateX(-50%);
		cursor: pointer;
	}

	.times {
		display: flex;
		justify-content: space-between;
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.times .now {
		color: var(--accent);
	}

	.log-head h3 {
		margin: 0 8px 0 0;
	}

	.log {
		padding: 0;
		margin-top: 12px;
	}

	.event {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px;
		border-bottom: 1px solid var(--soft);
	}

	.event:last-child {
		border-bottom: 0;
	}

	.event time {
		width: 45px;
		flex-shrink: 0;
	}

	.sentence {
		flex: 1;
		min-width: 0;
	}

	.empty-log {
		padding: 16px;
	}

	@media (max-width: 750px) {
		.row {
			grid-template-columns: 95px minmax(0, 1fr);
		}

		.event {
			gap: 6px;
		}
	}
</style>
