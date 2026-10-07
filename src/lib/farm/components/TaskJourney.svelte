<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import ContributedActions from '$lib/extensions/ContributedActions.svelte';
	import ExtensionPanels from '$lib/extensions/ExtensionPanels.svelte';
	import { extensions } from '$lib/extensions/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import { farmStore } from '../store.svelte';
	import { openInGraph } from '../graph';
	import * as api from '../api';
	import {
		FARM_COPY as C,
		STEPS,
		TASK_KIND_LABELS,
		TASK_TABS,
		agentName,
		askedBy,
		clockTime,
		duration,
		failureLine,
		isQuiet,
		journeyHeading,
		listIds,
		liveRun,
		readyToLand,
		relativeTime,
		retryAgent,
		runLine,
		track,
		transcriptKind,
		verificationLine
	} from '../describe';
	import type { Task, TaskDetail, TaskDraft } from '../types';

	/**
	 * One task's journey, at `/farm?task=<id>` (FEAT-110).
	 *
	 * The same six steps as the card's track, at full size and with a line of
	 * detail each; the actions for where the task is; and its evidence in tabs.
	 * The tab it opens on follows the state: a working task opens on its
	 * output, a ready one on its review, a stuck one on its checks.
	 */
	type Tab = (typeof TASK_TABS)[number];

	interface Props {
		task: Task;
		detail?: TaskDetail;
		now: number;
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		onback: () => void;
		onedit: () => void;
		ondelete: () => void;
		initialTab?: string | null;
	}

	let { task, detail, now, busy, act, onback, onedit, ondelete, initialTab }: Props = $props();

	/*
	 * The tab follows the task's state until the person picks one — the
	 * evidence that decides "ready" or "stuck" may arrive after the screen
	 * does — and from then on it is theirs and does not move under them.
	 */
	let picked = $state<Tab | null>(
		untrack(() =>
			initialTab && TASK_TABS.includes(initialTab as Tab) ? (initialTab as Tab) : null
		)
	);
	const tab = $derived.by<Tab>(() => {
		if (picked) return picked;
		if (readyToLand(detail)) return 'Review';
		if (task.status === 'failed' || task.status === 'blocked') return 'Checks';
		return 'Output';
	});
	let fullLog = $state<string | null>(null);
	let reassigning = $state(false);

	const farm = $derived(farmStore.farm);
	const run = $derived(liveRun(task.id, farmStore.runs));
	const agent = $derived(
		farmStore.agents.find(
			(a) => a.definition.id === (run?.agent ?? task.implementedBy ?? task.assignedAgent)
		)?.definition
	);
	const ready = $derived(readyToLand(detail));
	const stuck = $derived(task.status === 'failed' || task.status === 'blocked');
	const other = $derived(retryAgent(task, farmStore.agents, farmStore.runs));
	const target = $derived(task.mergeTarget ?? repo.info?.head.branch ?? 'the target branch');
	const reviewer = $derived(detail?.runs.findLast((r) => r.phase === 'review'));
	const isContainer = $derived(farmStore.tasks.some((t) => t.parent === task.id));
	const lines = $derived(fullLog?.split('\n') ?? farmStore.transcript(task.id));
	const checks = $derived(
		(task.verificationOverrides ? task.verification : farm?.verification) ?? []
	);

	/** Tasks that can start once this one lands. */
	const unlocks = $derived(
		farmStore.tasks
			.filter(
				(t) =>
					t.dependsOn.includes(task.id) &&
					t.dependsOn.every((id) => id === task.id || farmStore.byId.get(id)?.status === 'done')
			)
			.map((t) => t.id)
	);

	$effect(() => {
		extensions.setContext('farmTask', {
			taskId: task.id,
			taskHasCommit: Boolean(task.branch && task.worktree),
			workdir: task.worktree,
			base: task.mergeTarget ?? null
		});
		return () => extensions.setContext('farmTask', null);
	});

	/** One line under each of the six steps. */
	function stepDetail(i: number): string {
		switch (i) {
			case 0: {
				const waited = task.dependsOn.length ? ` · it waited for ${listIds(task.dependsOn)}` : '';
				return `${clockTime(task.createdMs)}${waited}`;
			}
			case 1:
				return task.branch ?? 'Not cut yet';
			case 2: {
				const who = agentName(agent?.id, farmStore.agents);
				return run && run.phase !== 'review' ? `${who} · ${relativeTime(run.startedMs, now)}` : who;
			}
			case 3:
				return detail?.verification
					? verificationLine(detail.verification)
					: checks.length
						? checks.join(', ')
						: C.noChecks;
			case 4:
				if (detail?.review) {
					const verdict = detail.review.decision === 'approve' ? 'approved' : 'asked for changes';
					return `${agentName(reviewer?.agent, farmStore.agents)} ${verdict}`;
				}
				return reviewer
					? `${agentName(reviewer.agent, farmStore.agents)} reads the diff`
					: 'Another agent reads the diff';
			default:
				if (task.status === 'done') return `Landed ${clockTime(task.updatedMs)}`;
				return farm?.autonomy === 'auto' || farm?.autonomy === 'yolo'
					? 'It merges itself'
					: 'You approve the merge';
		}
	}

	async function readFullLog() {
		const latest = detail?.runs.at(-1);
		if (!latest) return;
		await act('Could not read the log', async () => {
			fullLog = await api.transcript(latest.id, task.id);
		});
	}

	async function addProposal(title: string, description: string, dependsOn: string[]) {
		const draft: TaskDraft = {
			title,
			description,
			dependsOn,
			kind: 'general',
			priority: 'normal',
			allowedPaths: [],
			acceptanceCriteria: [],
			verification: [],
			verificationOverrides: false,
			assignedAgent: null,
			ready: false
		};
		await act('Could not add the task', () => api.addTask(draft));
	}
</script>

<header class="task-head">
	<div class="inline">
		<Chip onclick={onback}><Icon name="chevron-left" /> Board</Chip>
		<span class="mono muted">{task.id}</span>
		<h1>{task.title}</h1>
		<Chip>{TASK_KIND_LABELS[task.kind]}</Chip>
	</div>
	<div class="inline">
		{#if run}
			<Btn
				danger
				disabled={busy}
				onclick={() => act('Could not stop the task', () => api.cancelTask(task.id))}
			>
				{C.stop}
			</Btn>
			{#if task.worktree}
				<Btn onclick={() => repo.open(task.worktree!)}>{C.openWorktree}</Btn>
			{/if}
		{:else if ready}
			<Btn
				disabled={busy}
				onclick={() => act('Could not send the task back', () => api.retryTask(task.id))}
			>
				{C.sendBack}
			</Btn>
			<Btn
				primary
				disabled={busy}
				onclick={() => act('Could not land the task', () => api.mergeTask(task.id))}
			>
				Land it into {target}
			</Btn>
		{:else if stuck}
			<Btn
				danger
				disabled={busy}
				onclick={() => act('Could not give up on the task', () => api.cancelTask(task.id))}
			>
				{C.giveUp}
			</Btn>
			<Btn onclick={onedit}>{C.edit}</Btn>
			<Btn
				primary
				disabled={busy || !other}
				onclick={() =>
					act('Could not retry the task', () => api.runTask(task.id, other?.definition.id ?? null))}
			>
				Retry with {other?.definition.displayName ?? 'another agent'}
			</Btn>
		{:else if task.status !== 'done' && task.status !== 'cancelled'}
			<Btn
				primary
				disabled={busy || isContainer}
				onclick={() =>
					act('Could not run the task', () => api.runTask(task.id, task.assignedAgent))}
			>
				Run the task
			</Btn>
		{/if}
	</div>
</header>

<div class="farm-columns">
	<main class="farm-main stack">
		<section class="card stepper">
			<div class="spread">
				<h2>{journeyHeading(task, detail, run, farmStore.agents)}</h2>
				<span class="muted">Attempt {Math.max(1, task.attempts)} of {farm?.maxAttempts}</span>
			</div>
			<ol>
				{#each track(task, detail) as state, i (i)}
					<li class={state}>
						<span class="step-dot">
							{#if state === 'current'}
								<Loader size="inline" label={STEPS[i]} />
							{:else if state === 'done'}
								<Icon name="check" />
							{:else if state === 'yourTurn'}
								<Icon name="merge" />
							{:else if state === 'failed'}
								<Icon name="close" />
							{:else}
								{i + 1}
							{/if}
						</span>
						<strong>{STEPS[i]}</strong>
						<small class="muted">{stepDetail(i)}</small>
					</li>
				{/each}
			</ol>
		</section>

		{#if ready}
			<p class="card ok-card callout">
				Landing merges {task.branch} into {target}{unlocks.length
					? ` and frees ${listIds(unlocks)} to start`
					: ''}.
			</p>
		{:else if stuck}
			<p class="card danger-card callout">
				{failureLine(task, detail, farm?.maxAttempts ?? task.attempts)}.
			</p>
		{/if}

		<nav class="inline" aria-label="Task evidence">
			{#each TASK_TABS as name (name)}
				<Chip active={tab === name} onclick={() => (picked = name)}>
					{name}{name === 'Changes' && detail?.stats ? ` ${detail.stats.files.length}` : ''}
				</Chip>
			{/each}
		</nav>

		<section class="task-tab">
			{#if tab === 'Output'}
				<div class="output">
					{#each lines as line, i (i)}
						<div class="line {transcriptKind(line)}">{line}</div>
					{:else}
						<p class="muted">{C.noOutput}</p>
					{/each}
					{#if run}<span class="caret" aria-hidden="true">▍</span>{/if}
				</div>
				<div class="spread tab-foot">
					<span class="muted inline" class:warn={isQuiet(run, now)}>
						{#if run}
							<Loader size="inline" label="Working" />
							{agentName(run.agent, farmStore.agents)} is working · {runLine(run, now)}
						{/if}
					</span>
					<Btn disabled={busy || !detail?.runs.length} onclick={readFullLog}>{C.fullLog}</Btn>
				</div>
			{:else if tab === 'Changes'}
				<div class="card">
					{#if detail?.stats?.files.length}
						{#each detail.stats.files as file (file.path)}
							<div class="spread file-row">
								<span class="mono">{file.path}</span>
								{#if file.binary}
									<Chip>Binary</Chip>
								{:else}
									<span class="mono"
										><span class="ok">+{file.added}</span>
										<span class="danger">−{file.removed}</span></span
									>
								{/if}
							</div>
						{/each}
					{:else if detail?.handoff?.filesChanged.length}
						{#each detail.handoff.filesChanged as file (file)}<p class="mono">{file}</p>{/each}
					{:else}
						<p class="muted">{C.noChanges}</p>
					{/if}
					{#if task.branch}
						<div class="actions">
							<Btn
								onclick={() => act('Could not open the branch', () => openInGraph(task.branch!))}
							>
								{C.openBranch}
							</Btn>
						</div>
					{/if}
				</div>
			{:else if tab === 'Checks'}
				<div class="stack">
					{#if detail?.verification}
						{#each detail.verification.results as check (check.command)}
							<details class="card check" open={!check.passed}>
								<summary class="spread">
									<span class="inline">
										<span
											class="dot"
											style:background={check.passed ? 'var(--ok)' : 'var(--danger)'}
										></span>
										<span class="mono">{check.command}</span>
									</span>
									<span class="muted"
										>{check.passed ? 'passed' : 'failed'} · {duration(check.durationMs)}</span
									>
								</summary>
								<pre>{check.output}</pre>
							</details>
						{/each}
					{:else}
						<p class="muted">{C.noChecks}</p>
					{/if}
					<div class="actions">
						<Btn
							disabled={busy || !!run || !task.worktree}
							onclick={() => act('Could not run the checks', () => api.verifyTask(task.id))}
						>
							Run the checks
						</Btn>
					</div>
				</div>
			{:else if tab === 'Review'}
				<div class="card stack">
					{#if detail?.review}
						<div class="task-meta">
							<AgentBadge
								agent={farmStore.agents.find((a) => a.definition.id === reviewer?.agent)
									?.definition}
							/>
							<strong>{agentName(reviewer?.agent, farmStore.agents)}</strong>
							<Chip active={detail.review.decision === 'approve'}>
								{detail.review.decision === 'approve'
									? 'Approved'
									: detail.review.decision === 'blocked'
										? 'Blocked'
										: 'Changes asked'}
							</Chip>
						</div>
						<p>{detail.review.summary}</p>
						{#each detail.review.issues as issue, i (i)}
							<div class="issue">
								<Chip>{issue.severity}</Chip>
								<span class="mono muted">{issue.file}</span>
								<span>{issue.message}</span>
							</div>
						{/each}
					{:else}
						<p class="muted">{C.noReview}</p>
					{/if}
					<div class="actions">
						<Btn
							disabled={busy || !!run || !task.worktree}
							onclick={() => act('Could not ask for a review', () => api.reviewTask(task.id))}
						>
							Ask for a review
						</Btn>
						{#if farm?.supplemental && farm.supplemental.mode !== 'off'}
							<Btn
								disabled={busy}
								onclick={() =>
									act('Could not run CodeRabbit', () => api.reviewSupplemental(task.id))}
							>
								CodeRabbit
							</Btn>
						{/if}
					</div>
				</div>
			{:else}
				<div class="card stack">
					{#if detail?.handoff}
						<p>{detail.handoff.summary}</p>
						{#each detail.handoff.risks as risk, i (i)}<p class="warn">Risk: {risk}</p>{/each}
						{#each detail.handoff.questions as question, i (i)}<p>Question: {question}</p>{/each}
						{#each detail.handoff.proposedTasks as proposal, i (i)}
							<div class="spread proposal">
								<span>{proposal.title}</span>
								<Btn
									disabled={busy}
									onclick={() =>
										addProposal(proposal.title, proposal.description, proposal.dependsOn)}
								>
									Add to plan
								</Btn>
							</div>
						{/each}
					{:else}
						<p class="muted">{C.noHandoff}</p>
					{/if}
				</div>
			{/if}
		</section>

		<ContributedActions context="farmTask" />
		<ExtensionPanels
			location="farmTask"
			revision={task.id}
			onsend={async (extension, record, findings) => {
				await extensions.send(extension, record, findings);
				await farmStore.refresh();
			}}
		/>

		{#if !run}
			<div class="actions">
				<Btn
					disabled={busy || task.status === 'done'}
					onclick={() => act('Could not break the task down', () => api.decompose(task.id, null))}
				>
					Break it down
				</Btn>
				<Btn danger disabled={busy} onclick={ondelete}>Delete</Btn>
			</div>
		{/if}
	</main>

	<aside class="card farm-aside brief">
		<div class="who">
			<AgentBadge {agent} running={!!run} large />
			<div>
				<strong>{agentName(agent?.id, farmStore.agents)}</strong>
				<p class="muted" class:warn={isQuiet(run, now)}>
					{run
						? (run.phase === 'review' ? 'Reviewing' : 'Working') + ` · ${runLine(run, now)}`
						: task.status === 'done'
							? 'Landed it'
							: task.implementedBy
								? 'Wrote it'
								: C.free}
				</p>
			</div>
			<Btn disabled={busy || !!run} onclick={() => (reassigning = !reassigning)}>{C.reassign}</Btn>
		</div>
		{#if reassigning}
			<label>
				{C.reassign}
				<select
					value={task.assignedAgent ?? ''}
					onchange={(e) =>
						act('Could not reassign the task', () =>
							api.assignTask(task.id, e.currentTarget.value)
						)}
				>
					<option disabled value="">{C.firstFree}</option>
					{#each farmStore.usable as a (a.definition.id)}
						<option value={a.definition.id}>{a.definition.displayName}</option>
					{/each}
				</select>
			</label>
		{/if}

		<section>
			<h3 class="muted">{C.brief}</h3>
			<p>{task.description}</p>
		</section>

		{#if task.acceptanceCriteria.length}
			<section>
				<h3 class="muted">{C.doneWhen}</h3>
				<ul class="criteria">
					{#each task.acceptanceCriteria as criterion, i (i)}
						<li class:ok={task.status === 'done'}>
							<Icon name={task.status === 'done' ? 'circle-check' : 'circle'} />
							<span>{criterion}</span>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		<dl>
			<dt>{C.mayTouch}</dt>
			<dd>
				{#each task.allowedPaths as path (path)}<span class="mono path">{path}</span>{:else}Whole
					repository{/each}
			</dd>
			{#if task.dependsOn.length}
				<dt>{C.needs}</dt>
				<dd class="inline">
					{#each task.dependsOn as id (id)}
						<Chip onclick={() => goto(`/farm?task=${encodeURIComponent(id)}`)}>
							{id} · {farmStore.byId.get(id)?.status === 'done'
								? 'landed'
								: (farmStore.byId.get(id)?.status ?? 'missing')}
						</Chip>
					{/each}
				</dd>
			{/if}
			<dt>{C.askedBy}</dt>
			<dd>{askedBy(task.origin, farmStore.agents)}</dd>
			<dt>{C.branch}</dt>
			<dd class="mono">{task.branch ?? '—'}</dd>
			<dt>{C.worktree}</dt>
			<dd class="mono">{task.worktree ?? '—'}</dd>
		</dl>

		<div class="edit"><Btn onclick={onedit}>{C.edit}</Btn></div>
	</aside>
</div>

<style>
	.task-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 16px 0 14px;
	}

	.task-head h1 {
		font-size: var(--fs-title);
		font-weight: 550;
		margin: 0;
	}

	.stepper h2 {
		font-size: var(--fs-ui);
		margin: 0;
	}

	.stepper ol {
		display: grid;
		grid-template-columns: repeat(6, minmax(0, 1fr));
		gap: 10px;
		list-style: none;
		margin: 14px 0 0;
		padding: 0;
	}

	.stepper li {
		position: relative;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	/* The bar from the step before to this one. */
	.stepper li + li::before {
		content: '';
		position: absolute;
		top: 17px;
		left: calc(-100% + 36px);
		right: calc(100% + 10px);
		height: 2px;
		background: var(--soft);
	}

	.stepper li.done + li::before,
	.stepper li.done + li.yourTurn::before {
		background: color-mix(in srgb, var(--ok) 60%, transparent);
	}

	.step-dot {
		position: relative;
		z-index: 1;
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		border: 1px solid var(--soft);
		border-radius: 50%;
		background: var(--surface);
		color: var(--muted);
	}

	.done .step-dot {
		color: var(--ok);
		border-color: var(--ok);
	}

	.yourTurn .step-dot {
		color: var(--bg);
		border-color: var(--ok);
		background: var(--ok);
	}

	.failed .step-dot {
		color: var(--danger);
		border-color: var(--danger);
	}

	.current .step-dot {
		border-color: var(--accent);
		background: var(--accent-soft);
	}

	.stepper li.soft strong {
		color: var(--muted);
		font-weight: 400;
	}

	.stepper small {
		overflow-wrap: anywhere;
	}

	.callout {
		margin: 0;
	}

	.output {
		min-height: 280px;
		max-height: 460px;
		overflow: auto;
		padding: 14px 18px;
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
		background: var(--sunken);
		font: var(--fs-mono) / 1.75 var(--font-mono);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.line.think {
		color: var(--muted);
	}

	.line.edit {
		color: var(--ok);
	}

	.line.command {
		color: var(--lane-5);
	}

	.line.error {
		color: var(--danger);
	}

	.caret {
		color: var(--accent);
		animation: blink 1s steps(1) infinite;
	}

	@keyframes blink {
		50% {
			opacity: 0;
		}
	}

	.tab-foot {
		margin-top: 10px;
	}

	.file-row {
		padding: 10px 0;
		border-bottom: 1px solid var(--soft);
	}

	.check summary {
		cursor: pointer;
	}

	.check pre {
		margin-top: 12px;
	}

	.issue {
		display: flex;
		align-items: baseline;
		gap: 10px;
		padding: 10px 0;
		border-top: 1px solid var(--soft);
	}

	.proposal {
		padding-top: 10px;
		border-top: 1px solid var(--soft);
	}

	.brief {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.brief h3 {
		font-weight: 400;
		margin: 0 0 6px;
	}

	.who {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	.who > div {
		flex: 1;
		min-width: 0;
	}

	.who p {
		margin: 2px 0 0;
	}

	.criteria {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.criteria li {
		display: flex;
		gap: 8px;
		align-items: flex-start;
	}

	dl {
		display: grid;
		grid-template-columns: 90px minmax(0, 1fr);
		gap: 10px;
		margin: 0;
	}

	dt {
		color: var(--muted);
	}

	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}

	.path {
		display: inline-block;
		margin: 0 4px 4px 0;
		padding: 1px 8px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
	}

	.edit {
		margin-top: auto;
		display: flex;
		justify-content: center;
	}

	@media (max-width: 850px) {
		.stepper ol {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}

		.stepper li::before {
			display: none;
		}
	}
</style>
