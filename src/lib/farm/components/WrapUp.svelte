<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import AgentBadge from './AgentBadge.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { repo } from '$lib/repo.svelte';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import { openInGraph } from '../graph';
	import { FARM_COPY as C, agentColour, elapsed } from '../describe';
	import type { Task, TaskDraft } from '../types';

	/**
	 * Everything landed (FEAT-113).
	 *
	 * The seal, what landed and by whom, what the agents raised in their
	 * hand-offs and nobody has acted on, and the clean-up. Every number here is
	 * read from the record — the tasks, their runs, their branches' stats — so
	 * the summary cannot claim more than happened. "Checks passed" counts task
	 * branches, not the target, because the target is never checked as a whole.
	 */
	interface Props {
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		onopen: (id: string) => void;
		onnew: (title?: string, description?: string) => void;
	}

	let { busy, act, onopen, onnew }: Props = $props();

	type Raised = {
		task: Task;
		title: string;
		description: string;
		kind: 'question' | 'risk' | 'proposal';
	};

	const KIND_LABELS: Record<Raised['kind'], string> = {
		question: 'Question',
		risk: 'Risk',
		proposal: 'Proposed task'
	};

	let swept = $state<string | null>(null);

	const farm = $derived(farmStore.farm!);

	/** What landed, in the order it landed. */
	const landed = $derived(
		farm.tasks
			.filter((t) => t.status === 'done' && !farm.tasks.some((c) => c.parent === t.id))
			.sort((a, b) => a.updatedMs - b.updatedMs)
	);

	const target = $derived(
		landed.find((t) => t.mergeTarget)?.mergeTarget ?? repo.info?.head.branch ?? 'main'
	);

	/** Who did what: tasks built and reviews done per agent, busiest first. */
	const contributions = $derived(
		farmStore.agents
			.map((agent) => {
				const id = agent.definition.id;
				const completed = (phase: string) =>
					farmStore.runs.filter(
						(r) => r.agent === id && r.phase === phase && r.outcome.state === 'completed'
					).length;
				return {
					agent,
					planned: completed('planning') > 0,
					built: landed.filter((t) => t.implementedBy === id).length,
					reviewed: completed('review')
				};
			})
			.filter((c) => c.planned || c.built || c.reviewed)
			.sort((a, b) => b.built + b.reviewed - (a.built + a.reviewed))
	);
	const most = $derived(Math.max(1, ...contributions.map((c) => c.built + c.reviewed)));

	/** The open items from every hand-off. */
	const raised = $derived<Raised[]>(
		landed.flatMap((task) => {
			const handoff = farmStore.details[task.id]?.handoff;
			if (!handoff) return [];
			return [
				...handoff.questions.map((title) => ({
					task,
					title,
					description: '',
					kind: 'question' as const
				})),
				...handoff.risks.map((title) => ({ task, title, description: '', kind: 'risk' as const })),
				...handoff.proposedTasks.map((p) => ({
					task,
					title: p.title,
					description: p.description,
					kind: 'proposal' as const
				}))
			];
		})
	);

	const commits = $derived(
		landed.reduce((sum, t) => sum + (farmStore.details[t.id]?.stats?.commits ?? 0), 0)
	);
	const checked = $derived(
		landed.filter((t) => {
			const verification = farmStore.details[t.id]?.verification;
			return verification?.passed && !verification.unverified;
		}).length
	);

	function agentOf(task: Task) {
		return farmStore.agents.find((a) => a.definition.id === task.implementedBy)?.definition;
	}

	/** An answer is kept with the task that asked, under the question. */
	function answered(item: Raised): string | null {
		const marker = `${item.title}\nAnswer: `;
		const at = item.task.description.indexOf(marker);
		return at < 0 ? null : item.task.description.slice(at + marker.length).split('\n')[0];
	}

	function draftOf(task: Task): TaskDraft {
		return {
			title: task.title,
			description: task.description,
			kind: task.kind,
			priority: task.priority,
			dependsOn: task.dependsOn,
			allowedPaths: task.allowedPaths,
			acceptanceCriteria: task.acceptanceCriteria,
			verification: task.verification,
			verificationOverrides: task.verificationOverrides,
			assignedAgent: task.assignedAgent,
			ready: false
		};
	}

	async function answer(item: Raised) {
		const reply = await dialog.prompt({
			title: C.answer,
			body: item.title,
			label: 'Your answer',
			confirmLabel: C.save
		});
		if (reply === null) return;
		await act('Could not save the answer', () =>
			api.editTask(item.task.id, {
				...draftOf(item.task),
				description: `${item.task.description}\n\n${item.title}\nAnswer: ${reply}`
			})
		);
	}

	async function makeTask(item: Raised) {
		await act('Could not add the task', () =>
			api.addTask({
				title: item.title,
				description: item.description,
				kind: 'general',
				priority: 'normal',
				dependsOn: [],
				allowedPaths: [],
				acceptanceCriteria: [],
				verification: [],
				verificationOverrides: false,
				assignedAgent: null,
				ready: false
			})
		);
	}

	async function sweep() {
		await act('Could not tidy up', async () => {
			const removed = await api.sweep();
			await farmStore.leftovers();
			const worktrees = `${removed.length} ${removed.length === 1 ? 'worktree' : 'worktrees'}`;
			swept = `${worktrees} removed. The commits stay on ${target}.`;
		});
	}
</script>

<div class="farm-columns">
	<main class="farm-main">
		<section class="seal-row">
			<span class="seal"><Icon name="check" size="2.4em" /></span>
			<div>
				<h2>{farm.goal.title} has landed</h2>
				<p class="muted">
					{landed.length}
					{landed.length === 1 ? 'task' : 'tasks'} in {elapsed(
						farm.updatedMs - farm.createdMs
					)}{commits ? ` · ${commits} ${commits === 1 ? 'commit' : 'commits'} on ${target}` : ''} · checks
					passed on {checked} of {landed.length} task branches
				</p>
			</div>
		</section>

		<section class="section">
			<h3>{C.whatLanded}</h3>
			<div class="card rows">
				{#each landed as task (task.id)}
					{@const detail = farmStore.details[task.id]}
					<button class="row" onclick={() => onopen(task.id)}>
						<span class="mono muted">{task.id}</span>
						<span class="title">{task.title}</span>
						<AgentBadge agent={agentOf(task)} />
						<span class="mono muted sha">{detail?.mergeSha?.slice(0, 7) ?? '—'}</span>
						<span class="muted tries">{task.attempts} {task.attempts === 1 ? 'try' : 'tries'}</span>
					</button>
				{/each}
			</div>
		</section>

		<section class="section">
			<h3>{C.raised} <span class="muted">· {C.raisedDetail}</span></h3>
			{#if raised.length}
				<div class="card rows">
					{#each raised as item, i (i)}
						{@const reply = answered(item)}
						<div class="row raised">
							<AgentBadge agent={agentOf(item.task)} />
							<div class="title">
								<p>{item.title}</p>
								<span class="muted">{KIND_LABELS[item.kind]} · in {item.task.id}</span>
								{#if reply}<p class="muted">Answered: {reply}</p>{/if}
							</div>
							{#if item.kind === 'question'}
								<Btn disabled={busy} onclick={() => answer(item)}>{C.answer}</Btn>
							{:else if item.kind === 'risk'}
								<Btn disabled={busy} onclick={() => makeTask(item)}>{C.makeTask}</Btn>
							{:else}
								<Btn onclick={() => onnew(item.title, item.description)}>{C.startWith}</Btn>
							{/if}
						</div>
					{/each}
				</div>
			{:else}
				<p class="muted">{C.noRaised}</p>
			{/if}
			{#each Object.entries(farmStore.detailErrors) as [id, message] (id)}
				<p class="warn">Could not read {id}'s hand-off: {message}</p>
			{/each}
		</section>
	</main>

	<aside class="card farm-aside close">
		<section>
			<h3>{C.whoDidWhat}</h3>
			<div class="who-list">
				{#each contributions as c (c.agent.definition.id)}
					<div>
						<div class="spread">
							<span class="inline">
								<AgentBadge agent={c.agent.definition} />{c.agent.definition.displayName}
							</span>
							<small class="muted">
								{c.planned ? 'planned · ' : ''}{c.built} built · {c.reviewed} reviewed
							</small>
						</div>
						<div class="track">
							<div
								class="bar"
								style:width="{Math.max(4, ((c.built + c.reviewed) / most) * 100)}%"
								style:background={agentColour(c.agent.definition)}
							></div>
						</div>
					</div>
				{/each}
			</div>
		</section>

		<section class="rule">
			<h3>{C.tidyUp}</h3>
			<p class="muted">
				Merged farm branches and clean worktrees are removed. Uncommitted or unmerged work stays.
			</p>
			<Btn disabled={busy} onclick={sweep}>{C.removeWorktrees}</Btn>
			{#if swept}<p class="muted">{swept}</p>{/if}
		</section>

		<div class="finish">
			<Btn onclick={() => act(`Could not open ${target}`, () => openInGraph(target))}>
				Open {target} in Graph
			</Btn>
			<Btn primary onclick={() => onnew()}>{C.newGoal}</Btn>
		</div>
	</aside>
</div>

<style>
	.seal-row {
		display: flex;
		align-items: center;
		gap: 24px;
		padding: 20px 0 8px;
	}

	.seal-row h2 {
		font-size: calc(var(--fs-title) * 1.6);
		margin: 0 0 6px;
	}

	/* Pops in once, then stays. */
	.seal {
		display: grid;
		place-items: center;
		width: 84px;
		height: 84px;
		flex-shrink: 0;
		border-radius: 50%;
		color: var(--ok);
		border: 1px solid color-mix(in srgb, var(--ok) 60%, transparent);
		background: color-mix(in srgb, var(--ok) 12%, var(--surface-veil));
		box-shadow: var(--glass-rim), var(--shadow-1);
		animation: pop-in var(--t-enter-liquid) var(--spring-liquid);
	}

	.rows {
		padding: 0;
		overflow: hidden;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 16px;
		width: 100%;
		padding: 14px 18px;
		border: 0;
		border-bottom: 1px solid var(--soft);
		background: none;
		color: var(--ink);
		font: inherit;
		text-align: left;
	}

	.row:last-child {
		border-bottom: 0;
	}

	button.row {
		cursor: pointer;
	}

	button.row:hover {
		background: var(--hover);
	}

	.title {
		flex: 1;
		min-width: 0;
	}

	.raised p {
		margin: 0;
	}

	.sha {
		width: 64px;
	}

	.tries {
		width: 48px;
		text-align: right;
	}

	.close {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.who-list {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.who-list .inline {
		flex-wrap: nowrap;
		white-space: nowrap;
	}

	.who-list small {
		text-align: right;
	}

	.track {
		margin: 8px 0 0 34px;
	}

	.bar {
		height: 6px;
		border-radius: var(--r-pill);
	}

	.finish {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding-top: 20px;
	}
</style>
