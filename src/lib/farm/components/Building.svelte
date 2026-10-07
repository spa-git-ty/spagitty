<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import TaskTrack from './TaskTrack.svelte';
	import {
		FARM_COPY as C,
		AUTONOMY_LEVELS,
		SEGMENTS,
		TASK_KIND_LABELS,
		activityLine,
		agentName,
		eventColour,
		failureLine,
		freeLine,
		isQuiet,
		listIds,
		liveRun,
		nowSentence,
		readyToLand,
		relativeTime,
		retryAgent,
		runLine,
		runningCheck,
		segment,
		statsLine,
		waitingOn,
		whereYouComeIn
	} from '../describe';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import type { Autonomy, Task, TaskStatus } from '../types';

	/**
	 * The farm's home while it runs (FEAT-109).
	 *
	 * Top to bottom it answers the handoff's four questions: what is happening
	 * (the sentence and the bar), what is waiting on you (Needs you), where each
	 * task is (the board), and what each agent is doing (the crew card).
	 *
	 * Stuck and ready-to-land tasks live in Needs you, not on the board, so
	 * nothing appears twice. Every count here is read from the snapshot; the
	 * only clock is the screen's.
	 */
	interface Props {
		now: number;
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		onopen: (id: string, tab?: string) => void;
		onadd: () => void;
		onrules: () => void;
		onactivity: () => void;
	}

	let { now, busy, act, onopen, onadd, onrules, onactivity }: Props = $props();

	let showCancelled = $state(false);
	let expanded = $state<string[]>([]);

	const COLUMNS: { name: string; caption: string; statuses: TaskStatus[]; colour: string }[] = [
		{
			name: C.upNext,
			caption: C.upNextDetail,
			statuses: ['ready', 'assigned', 'waiting', 'blocked'],
			colour: 'var(--warn)'
		},
		{ name: C.working, caption: C.workingDetail, statuses: ['running'], colour: 'var(--accent)' },
		{
			name: C.checking,
			caption: C.checkingDetail,
			statuses: ['verification', 'review'],
			colour: 'var(--lane-5)'
		},
		{ name: C.landed, caption: '', statuses: ['done'], colour: 'var(--ok)' }
	];

	const SLOTS = [1, 2, 3, 4, 6];

	const farm = $derived(farmStore.farm!);
	const tasks = $derived(farmStore.tasks);
	const needs = $derived(farmStore.needsYou);
	const needIds = $derived(new Set(needs.map((task) => task.id)));
	const target = $derived(tasks.find((t) => t.mergeTarget)?.mergeTarget ?? 'main');

	/** The tasks that count: a container is counted through its children. */
	const leaves = $derived(
		tasks.filter((t) => t.status !== 'cancelled' && !tasks.some((c) => c.parent === t.id))
	);
	const landed = $derived(leaves.filter((t) => t.status === 'done').length);
	const segments = $derived(
		leaves
			.map((task) => segment(task, farmStore.details[task.id], needIds.has(task.id)))
			.sort((a, b) => SEGMENTS.findIndex((s) => s.id === a) - SEGMENTS.findIndex((s) => s.id === b))
	);
	const legend = $derived(SEGMENTS.filter((s) => s.id !== 'waiting' && segments.includes(s.id)));

	const running = $derived(farmStore.runs.filter((r) => r.outcome.state === 'running'));
	const busyAgents = $derived(new Set(running.map((r) => r.agent)).size);

	function column(statuses: TaskStatus[]): Task[] {
		const list = tasks.filter(
			(t) => !t.parent && statuses.includes(t.status) && !needIds.has(t.id)
		);
		// Landed reads newest first; everything else keeps the plan's order.
		return statuses.includes('done') ? list.sort((a, b) => b.updatedMs - a.updatedMs) : list;
	}

	function agentFor(task: Task) {
		const id = liveRun(task.id, farmStore.runs)?.agent ?? task.implementedBy ?? task.assignedAgent;
		return farmStore.agents.find((a) => a.definition.id === id)?.definition;
	}

	/** The one line under a card's title. */
	function statusLine(task: Task): string {
		const run = liveRun(task.id, farmStore.runs);
		if (run?.phase === 'review') return `${agentName(run.agent, farmStore.agents)} is reviewing`;
		if (run) return runLine(run, now);
		if (task.status === 'verification')
			return runningCheck(task.id, farmStore.activity) ?? 'Checks are running';
		if (task.status === 'done') {
			const stats = statsLine(farmStore.details[task.id]?.stats);
			return `Landed ${relativeTime(task.updatedMs, now)} ago${stats ? ` · ${stats.split(',')[0]}` : ''}`;
		}
		const unmet = task.dependsOn.filter((id) => farmStore.byId.get(id)?.status !== 'done');
		if (unmet.length) return `Waits for ${listIds(unmet)}`;
		return (
			farmStore.waitingFor(task.id) ??
			waitingOn(task, farmStore.byId) ??
			task.note ??
			'Waits for a free agent'
		);
	}

	function toggle(id: string) {
		expanded = expanded.includes(id) ? expanded.filter((x) => x !== id) : [...expanded, id];
	}

	function setAutonomy(level: Autonomy) {
		// Merging by itself needs the merge permission too; see RulesEditor.
		return act('Could not change the autonomy', () =>
			api.configure({
				autonomy: level,
				permissions: { ...farm.permissions, merge: level === 'auto' || level === 'yolo' }
			})
		);
	}
</script>

{#snippet taskCard(task: Task)}
	{@const run = liveRun(task.id, farmStore.runs)}
	{@const children = tasks.filter((t) => t.parent === task.id)}
	{@const quiet = isQuiet(run, now)}
	<button
		class="card click-card task"
		class:omitted={task.status === 'cancelled'}
		aria-expanded={children.length ? expanded.includes(task.id) : undefined}
		onclick={() => (children.length ? toggle(task.id) : onopen(task.id))}
	>
		<div class="spread">
			<span class="mono muted">{task.id}</span>
			<Chip>{TASK_KIND_LABELS[task.kind]}</Chip>
		</div>
		<p class="task-title">{task.title}</p>
		<div class="task-meta">
			<AgentBadge agent={agentFor(task)} running={!!run} />
			<span class="muted line" class:warn={quiet}>
				{children.length
					? `${children.filter((t) => t.status === 'done').length}/${children.length} landed`
					: statusLine(task)}
			</span>
			{#if run || task.status === 'verification'}<Loader
					size="inline"
					label={statusLine(task)}
				/>{/if}
		</div>
		<TaskTrack {task} detail={farmStore.details[task.id]} {children} />
	</button>
	{#if children.length && expanded.includes(task.id)}
		<div class="children">
			{#each children as child (child.id)}{@render taskCard(child)}{/each}
			<Chip onclick={() => onopen(task.id)}>Open {task.id}</Chip>
		</div>
	{/if}
{/snippet}

<div class="farm-columns">
	<main class="farm-main">
		<section class="card now-card">
			<div>
				<p class="goal"><span class="muted">Goal</span> {farm.goal.title}</p>
				<p class="now-sentence">
					{#if running.length && farm.status !== 'paused'}<Loader
							size="inline"
							label="Working"
						/>{/if}
					{nowSentence(leaves, needs, farm.status === 'paused')}
				</p>
				<p class="muted">
					Started {relativeTime(farm.createdMs, now)} ago ·
					{AUTONOMY_LEVELS.find((a) => a.id === farm.autonomy)?.label}: you come in
					{whereYouComeIn(farm.autonomy)}
				</p>
			</div>
			<div>
				<div class="spread">
					<span
						><strong class="total">{landed}</strong>
						<span class="muted">of {leaves.length} landed</span></span
					>
					<span class="muted">{leaves.length - landed} to go</span>
				</div>
				<div class="segments" role="img" aria-label="{landed} of {leaves.length} landed">
					{#each segments as id, i (i)}
						<span
							class:glow={id === 'working'}
							style:background={SEGMENTS.find((s) => s.id === id)?.colour}
						></span>
					{/each}
				</div>
				<div class="legend muted">
					{#each legend as item (item.id)}
						<span><span class="dot" style:background={item.colour}></span>{item.label}</span>
					{/each}
				</div>
			</div>
		</section>

		{#if needs.length}
			<section class="section">
				<h3>
					{C.needsYou}
					<span class="muted">· {needs.length} · nothing else is waiting on a person</span>
				</h3>
				<div class="attention-grid">
					{#each needs as task (task.id)}
						{@const detail = farmStore.details[task.id]}
						{@const ready = readyToLand(detail)}
						{@const other = retryAgent(task, farmStore.agents, farmStore.runs)}
						{@const reviewer = detail?.runs.findLast((r) => r.phase === 'review')?.agent}
						<article class="card need" class:ok-card={ready} class:danger-card={!ready}>
							<span class="need-icon" class:ok={ready} class:danger={!ready}>
								<Icon name={ready ? 'merge' : 'warning'} />
							</span>
							<div class="need-body">
								<p class="need-title"><span class="mono muted">{task.id}</span> {task.title}</p>
								<p class="muted">
									{#if ready}
										Checks passed · {agentName(reviewer, farmStore.agents)} approved{statsLine(
											detail?.stats
										)
											? ` · ${statsLine(detail?.stats)}`
											: ''}
									{:else if task.status === 'review'}
										Checked and waiting for a review
									{:else}
										{failureLine(task, detail, farm.maxAttempts)}
									{/if}
								</p>
								<div class="actions">
									{#if ready}
										<Btn
											primary
											disabled={busy}
											onclick={() => act('Could not land the task', () => api.mergeTask(task.id))}
										>
											{C.landIt}
										</Btn>
										<Btn onclick={() => onopen(task.id, 'Review')}>{C.seeChanges}</Btn>
									{:else if task.status === 'review'}
										<Btn
											primary
											disabled={busy}
											onclick={() =>
												act('Could not ask for a review', () => api.reviewTask(task.id))}
										>
											Ask for a review
										</Btn>
										<Btn onclick={() => onopen(task.id, 'Checks')}>{C.openTask}</Btn>
									{:else}
										<Btn
											primary
											disabled={busy || !other}
											onclick={() =>
												act('Could not retry the task', () =>
													api.runTask(task.id, other?.definition.id ?? null)
												)}
										>
											Retry with {other?.definition.displayName ?? 'another agent'}
										</Btn>
										<Btn onclick={() => onopen(task.id, 'Checks')}>{C.openTask}</Btn>
									{/if}
								</div>
							</div>
						</article>
					{/each}
				</div>
			</section>
		{/if}

		<section class="board section">
			{#each COLUMNS as col (col.name)}
				{@const list = column(col.statuses)}
				<section>
					<h3>
						<span class="dot" style:background={col.colour}></span>
						{col.name} <span class="muted">{list.length}</span>
					</h3>
					<p class="muted board-caption">{col.caption || `Merged into ${target}`}</p>
					<div class="stack">
						{#each list as task (task.id)}{@render taskCard(task)}{/each}
					</div>
				</section>
			{/each}
		</section>
		{#if !tasks.length}<p class="empty">{C.noTasks}</p>{/if}

		{#if tasks.some((t) => t.status === 'cancelled')}
			<label class="inline section cancelled">
				<input type="checkbox" bind:checked={showCancelled} />{C.cancelled}
			</label>
			{#if showCancelled}
				<div class="board section">
					{#each tasks.filter((t) => t.status === 'cancelled') as task (task.id)}{@render taskCard(
							task
						)}{/each}
				</div>
			{/if}
		{/if}

		<footer class="farm-pill ornament">
			<button
				class="pill-button"
				disabled={busy}
				onclick={() =>
					act('Could not change the farm', farm.status === 'paused' ? api.start : api.pause)}
			>
				<Icon name={farm.status === 'paused' ? 'play' : 'pause'} />
				{farm.status === 'paused' ? C.resume : C.pause}
			</button>
			<span class="divider"></span>
			<label class="pill-select">
				<span class="muted">{C.autonomy}</span>
				<select
					value={farm.autonomy}
					disabled={busy}
					onchange={(e) => setAutonomy(e.currentTarget.value as Autonomy)}
				>
					{#each AUTONOMY_LEVELS as level (level.id)}<option value={level.id}>{level.label}</option
						>{/each}
				</select>
			</label>
			<span class="divider"></span>
			<label class="pill-select">
				<span class="slots" aria-hidden="true">
					{#each Array(farm.maxParallel) as _, i (i)}<span class:filled={i < busyAgents}
						></span>{/each}
				</span>
				<select
					aria-label="Agents at once"
					value={farm.maxParallel}
					disabled={busy}
					onchange={(e) =>
						act('Could not change how many run at once', () =>
							api.configure({ maxParallel: Number(e.currentTarget.value) })
						)}
				>
					{#each SLOTS.includes(farm.maxParallel) ? SLOTS : [...SLOTS, farm.maxParallel] as n (n)}
						<option value={n}>{n} at once</option>
					{/each}
				</select>
			</label>
			<span class="divider"></span>
			<button class="pill-button" onclick={onadd}><Icon name="plus" /> {C.addTask}</button>
			<button class="pill-button" onclick={onrules}><Icon name="settings" /> {C.rules}</button>
		</footer>
	</main>

	<aside class="card farm-aside">
		<div class="spread">
			<h3>Crew</h3>
			<span class="muted busy">
				{busyAgents} of {farm.maxParallel} busy
				<span class="slots" aria-hidden="true">
					{#each Array(farm.maxParallel) as _, i (i)}<span class:filled={i < busyAgents}
						></span>{/each}
				</span>
			</span>
		</div>
		<div class="crew">
			{#each farmStore.usable as agent (agent.definition.id)}
				{@const run = running.find((r) => r.agent === agent.definition.id)}
				{@const next = freeLine(agent.definition.id, tasks, true)}
				<div class="crew-row">
					<AgentBadge agent={agent.definition} running={!!run} />
					<div class="crew-text">
						<p>
							<strong>{agent.definition.displayName}</strong>
							{#if run && run.task !== 'planning'}
								<button class="mono task-id" onclick={() => onopen(run.task)}>{run.task}</button>
							{/if}
						</p>
						<p class="muted" class:warn={isQuiet(run, now)}>
							{run ? runLine(run, now) : next ? `${C.free} · ${next}` : C.free}
						</p>
					</div>
					{#if run}<Loader
							size="inline"
							label={`${agent.definition.displayName} is working`}
						/>{/if}
				</div>
			{/each}
		</div>
		<section class="rule">
			<div class="spread">
				<h3>{C.lately}</h3>
				<button class="link" onclick={onactivity}>{C.allActivity}</button>
			</div>
			<ul class="lately">
				{#each farmStore.activity
					.filter((e) => e.kind !== 'agentOutput')
					.slice(-5)
					.reverse() as event, i (i)}
					<li>
						<span class="dot" style:background={eventColour(event)}></span>
						<div>
							<p>{activityLine(event, farmStore.agents)}</p>
							{#if event.atMs}<small class="muted">{relativeTime(event.atMs, now)} ago</small>{/if}
						</div>
					</li>
				{/each}
			</ul>
		</section>
	</aside>
</div>

<style>
	.goal {
		margin: 0 0 6px;
	}

	.now-sentence {
		display: flex;
		align-items: center;
		gap: 10px;
		font-size: var(--fs-title);
		line-height: 1.35;
		margin: 0 0 8px;
	}

	.total {
		font-size: var(--fs-title);
	}

	.segments .glow {
		box-shadow: 0 0 6px color-mix(in srgb, var(--accent) 60%, transparent);
		animation: glow 2.4s ease-in-out infinite;
	}

	@keyframes glow {
		50% {
			opacity: 0.6;
		}
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 14px;
	}

	.legend > span {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.need {
		display: flex;
		gap: 14px;
		padding: 16px;
	}

	.need-icon {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		flex-shrink: 0;
		border-radius: 50%;
		background: color-mix(in srgb, currentColor 14%, transparent);
	}

	.need-body {
		flex: 1;
		min-width: 0;
	}

	.need-title {
		margin: 0 0 4px;
		font-weight: 550;
	}

	.task {
		display: flex;
		flex-direction: column;
	}

	.task .line {
		flex: 1;
		min-width: 0;
	}

	.children {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		margin-left: 12px;
		padding-left: 8px;
		border-left: 2px solid var(--soft);
	}

	.cancelled {
		color: var(--muted);
	}

	/* The pill's own controls: text buttons and selects that read as text. */
	.pill-button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 6px;
		border: none;
		border-radius: var(--r-button);
		background: none;
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}

	.pill-button:hover:not(:disabled) {
		background: var(--hover);
	}

	.pill-select {
		flex-direction: row;
		align-items: center;
		gap: 8px;
		font-size: var(--fs-ui);
	}

	.farm-pill select {
		border: none;
		background: none;
		padding: 4px 2px;
		color: var(--ink);
		cursor: pointer;
	}

	.divider {
		width: 1px;
		height: 20px;
		background: var(--soft);
	}

	.slots {
		display: inline-flex;
		gap: 3px;
	}

	.slots span {
		width: 7px;
		height: 14px;
		border-radius: 2px;
		background: var(--soft);
	}

	.slots span.filled {
		background: var(--accent);
	}

	.busy {
		display: inline-flex;
		align-items: center;
		gap: 8px;
	}

	.crew {
		display: flex;
		flex-direction: column;
		gap: 16px;
		margin-top: 16px;
	}

	.crew-row {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	.crew-text {
		flex: 1;
		min-width: 0;
	}

	.crew-text p {
		margin: 0;
	}

	.task-id {
		border: none;
		background: none;
		padding: 0 4px;
		color: var(--muted);
		cursor: pointer;
	}

	.task-id:hover {
		color: var(--accent);
	}

	.link {
		border: none;
		background: none;
		padding: 0;
		color: var(--accent);
		font: inherit;
		cursor: pointer;
	}

	.lately {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.lately li {
		display: flex;
		align-items: baseline;
		gap: 10px;
	}

	.lately p {
		margin: 0;
	}
</style>
