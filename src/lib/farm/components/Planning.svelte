<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import {
		FARM_COPY as C,
		agentName,
		elapsed,
		plannedCount,
		planningKind,
		waves
	} from '../describe';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import type { AgentRun } from '../types';

	/**
	 * The planner at work (FEAT-111).
	 *
	 * The weave orb, the planner's lines as they arrive, and what it was given.
	 * The lines are the planning transcript the store already keeps; only the
	 * last four are at full strength, so the newest reads first.
	 *
	 * When the run ends with drafts the orb stops, the title says so, and the
	 * screen waits for `Review the plan` — the route moves on by itself only
	 * when the window has focus, so nobody comes back to a screen that changed
	 * under them.
	 */
	interface Props {
		/** The planning run in flight, or nothing once it has finished. */
		run: AgentRun | null;
		now: number;
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		onreview: () => void;
	}

	let { run, now, busy, act, onreview }: Props = $props();

	/** How many lines stay at full strength. */
	const FRESH = 4;

	const lines = $derived(farmStore.planning);
	const drafts = $derived(farmStore.drafts);
	const planner = $derived(
		farmStore.agents.find((agent) => agent.definition.id === run?.agent)?.definition
	);
	const name = $derived(agentName(run?.agent, farmStore.agents));
	const title = $derived(run ? `${name} is planning` : C.planReady);
	const subtitle = $derived(
		run
			? `${elapsed(now - run.startedMs)} so far · ${C.plannerReading}`
			: `${drafts.length} ${drafts.length === 1 ? 'task' : 'tasks'} in ${waves(drafts).length} ${
					waves(drafts).length === 1 ? 'wave' : 'waves'
				}`
	);
</script>

<div class="farm-columns">
	<main class="farm-main planning">
		<div class="orb" class:finished={!run}>
			<Loader label="" />
		</div>
		<h2>{title}</h2>
		<p class="subtitle">{subtitle}</p>
		{#if !run}
			<div class="review"><Btn primary onclick={onreview}>{C.reviewPlan}</Btn></div>
		{/if}

		<div class="spread stream-head">
			<span class="muted">{C.planningDoing}</span>
			<span class="mono muted">{plannedCount(drafts.length, !!run)}</span>
		</div>
		<ol class="stream" aria-live="polite">
			{#each lines as line, i (i)}
				{@const kind = planningKind(line)}
				<li class:older={i < lines.length - FRESH}>
					<span class="kind {kind}">{kind}</span>
					<span>{line}</span>
				</li>
			{:else}
				<li class="older"><span class="kind thinking">reading</span><span>…</span></li>
			{/each}
		</ol>
	</main>

	<aside class="card farm-aside given">
		<div class="who">
			<AgentBadge agent={planner} running={!!run} />
			<div>
				<strong>{name} plans</strong>
				<p class="muted">{C.plannerReadOnly}</p>
			</div>
		</div>
		<section>
			<h3 class="muted">{C.plannerGiven}</h3>
			<p>{C.plannerGivenDetail}</p>
		</section>
		<section>
			<h3 class="muted">{C.plannerTaskSays}</h3>
			<ul>
				{#each C.plannerTaskParts as part}<li>{part}</li>{/each}
			</ul>
		</section>
		<section>
			<h3 class="muted">{C.plannerThen}</h3>
			<p>{C.plannerThenDetail}</p>
		</section>
		{#if run}
			<div class="stop">
				<Btn disabled={busy} onclick={() => act('Could not stop planning', api.cancelPlan)}>
					{C.stopPlanning}
				</Btn>
			</div>
		{/if}
	</aside>
</div>

<style>
	.planning {
		max-width: 760px;
		width: 100%;
		margin: 0 auto;
		padding-top: 24px;
	}

	/* The weave orb at 132 px: the Loader's own, scaled rather than redrawn. */
	.orb {
		height: 150px;
		display: grid;
		place-items: center;
	}

	.orb :global(.pane) {
		flex: none;
		min-height: 0;
		padding: 0;
	}

	.orb :global(.orb) {
		transform: scale(1.435);
	}

	.finished :global(*) {
		animation: none !important;
	}

	h2 {
		text-align: center;
		margin: 18px 0 6px;
	}

	.subtitle {
		text-align: center;
		color: var(--muted);
	}

	.review {
		display: flex;
		justify-content: center;
		margin-top: 12px;
	}

	.stream-head {
		margin: 28px 4px 8px;
	}

	.stream {
		list-style: none;
		margin: 0;
		padding: 14px 18px;
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
		background: var(--sunken);
		font: var(--fs-mono) / 1.75 var(--font-mono);
		max-height: 420px;
		overflow: auto;
	}

	.stream li {
		display: flex;
		gap: 16px;
		animation: rise var(--t-enter) var(--ease);
	}

	.stream .older {
		opacity: 0.45;
	}

	.kind {
		min-width: 64px;
		flex-shrink: 0;
	}

	.kind.reading {
		color: var(--lane-5);
	}

	.kind.thinking {
		color: var(--lane-4);
	}

	.kind.task {
		color: var(--ok);
	}

	.given {
		display: flex;
		flex-direction: column;
		gap: 18px;
	}

	.given h3 {
		font-weight: 400;
		margin: 0 0 6px;
	}

	.given ul {
		margin: 0;
		padding-left: 18px;
		line-height: 1.8;
	}

	.who {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	.who p {
		margin: 2px 0 0;
	}

	.stop {
		margin-top: auto;
		display: grid;
	}
</style>
