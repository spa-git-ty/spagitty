<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { STEPS, track, type TrackState } from '../describe';
	import type { Task, TaskDetail } from '../types';

	/**
	 * The six short bars under every task card (FEAT-108): the same six steps
	 * as the Task screen's stepper, so a card and its screen never disagree.
	 *
	 * A container (FEAT-076) has no steps of its own. Its track is one bar per
	 * child instead, filled as they land.
	 */
	interface Props {
		task: Task;
		detail?: TaskDetail;
		children?: Task[];
	}

	let { task, detail, children = [] }: Props = $props();

	const WORDS: Record<TrackState, string> = {
		done: 'done',
		current: 'now',
		failed: 'failed',
		waiting: 'waiting',
		soft: 'not yet',
		yourTurn: 'your turn'
	};

	const bars = $derived<TrackState[]>(
		children.length
			? children.map((child) => (child.status === 'done' ? 'done' : 'soft'))
			: track(task, detail)
	);

	const title = $derived(
		children.length
			? `${children.filter((c) => c.status === 'done').length} of ${children.length} landed`
			: STEPS.map((step, i) => `${step}: ${WORDS[bars[i]]}`).join(' · ')
	);
</script>

<div class="task-track" role="img" aria-label={title} {title}>
	{#each bars as state, i (i)}<span class={state}></span>{/each}
</div>

<style>
	.task-track {
		display: flex;
		gap: 3px;
		width: 100%;
		margin-top: 10px;
	}

	.task-track span {
		flex: 1;
		height: 4px;
		border-radius: var(--r-pill);
		background: var(--soft);
	}

	.task-track .done,
	.task-track .yourTurn {
		background: var(--ok);
	}

	.task-track .current {
		background: var(--accent);
		box-shadow: 0 0 6px color-mix(in srgb, var(--accent) 60%, transparent);
		animation: glow 2.4s ease-in-out infinite;
	}

	.task-track .failed {
		background: var(--danger);
	}

	.task-track .waiting {
		background: color-mix(in srgb, var(--warn) 70%, transparent);
	}

	@keyframes glow {
		50% {
			opacity: 0.55;
		}
	}
</style>
