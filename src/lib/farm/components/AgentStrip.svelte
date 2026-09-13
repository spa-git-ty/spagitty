<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { duration, quietLine } from '../describe';
	import type { AgentRun, Task } from '../types';

	/**
	 * Who is working, on what, and for how long (FEAT-077).
	 *
	 * The question a person actually has while a farm runs is not "what is the
	 * state of task seven", it is "what are they all doing" — and answering it
	 * meant reading a list of a dozen rows and matching agent names against
	 * statuses. One chip per working agent answers it in a glance, and the
	 * strip disappears when nothing is running rather than leaving an empty
	 * shelf.
	 *
	 * A chip that has gone quiet says so. That is the whole of the stall
	 * handling: nothing is stopped, because a model may think for a long time
	 * and killing it throws the work away — but a run that has said nothing for
	 * six minutes and a run that died four minutes ago look identical, and only
	 * one of them is worth interrupting.
	 */
	interface Props {
		/** The runs in flight. */
		runs: AgentRun[];
		byId: Map<string, Task>;
		/** The clock, passed in so the whole screen ticks together. */
		now: number;
		onselect: (task: string) => void;
	}

	let { runs, byId, now, onselect }: Props = $props();

	const working = $derived(runs.filter((run) => run.outcome.state === 'running'));
</script>

{#if working.length > 0}
	<div class="strip">
		{#each working as run (run.id)}
			{@const quiet = quietLine(run, now)}
			<button
				class="agent"
				class:quiet={quiet !== null}
				onclick={() => onselect(run.task)}
				title={quiet ?? `${run.agent} has been on ${run.task} for ${duration(now - run.startedMs)}`}
			>
				<span class="dot" class:still={quiet !== null}></span>
				<span class="who">{run.agent}</span>
				<span class="what mono">{byId.get(run.task)?.title ?? run.task}</span>
				<span class="how-long note">{duration(now - run.startedMs)}</span>
			</button>
		{/each}
	</div>
{/if}

<style>
	.strip {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border-bottom: 1px solid color-mix(in srgb, var(--line) 45%, transparent);
	}

	.agent {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		max-width: 320px;
		padding: 3px 10px;
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
		border-radius: var(--r-pill);
		background-color: var(--surface-veil);
		font-size: var(--fs-secondary);
		/* It arrives when its agent starts, rather than appearing. */
		animation: arrive var(--t-slow) var(--ease);
	}

	.agent:hover {
		border-color: color-mix(in srgb, var(--accent) 60%, transparent);
	}

	/* Six minutes without a word. Not an error — a question. */
	.quiet {
		border-color: color-mix(in srgb, var(--warn) 45%, transparent);
	}

	.dot {
		flex: none;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background-color: var(--accent);
		animation: breathe 2.4s var(--ease) infinite;
	}

	/*
	 * A quiet run's dot stops breathing and holds.
	 *
	 * The animation *is* the message: a farm you glance at says "still going"
	 * by moving, so the one that has stopped moving is the one to look at.
	 */
	.still {
		background-color: var(--warn);
		animation: none;
	}

	.who {
		font-weight: 600;
		white-space: nowrap;
	}

	.what {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.how-long {
		white-space: nowrap;
	}

	@keyframes breathe {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}

	@keyframes arrive {
		from {
			opacity: 0;
			transform: translateY(-2px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.agent,
		.dot {
			animation: none;
		}
	}
</style>
