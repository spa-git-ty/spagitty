<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { repo } from '$lib/repo.svelte';
	import { farmStore } from '$lib/farm/store.svelte';
	import * as api from '$lib/farm/api';
	import { FARM_COPY as C, PANES, phaseFor } from '$lib/farm/describe';
	import * as farmDelight from '$lib/farm/delight';
	import type { FarmSettings, Task, TaskDraft, TaskStatus } from '$lib/farm/types';
	import FarmPhases from '$lib/farm/components/FarmPhases.svelte';
	import Setup from '$lib/farm/components/Setup.svelte';
	import Planning from '$lib/farm/components/Planning.svelte';
	import PlanReview from '$lib/farm/components/PlanReview.svelte';
	import Building from '$lib/farm/components/Building.svelte';
	import TaskJourney from '$lib/farm/components/TaskJourney.svelte';
	import CrewPanel from '$lib/farm/components/CrewPanel.svelte';
	import ActivityPanel from '$lib/farm/components/ActivityPanel.svelte';
	import WrapUp from '$lib/farm/components/WrapUp.svelte';
	import RulesEditor from '$lib/farm/components/RulesEditor.svelte';
	import TaskEditor from '$lib/farm/components/TaskEditor.svelte';
	import FarmSheet from '$lib/farm/components/FarmSheet.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import '$lib/farm/farm.css';

	/*
	 * The Farm (1Q), as a journey: Crew → Goal → Plan → Build → Wrap up
	 * (FEAT-108 to FEAT-113).
	 *
	 * The screen follows the farm's state rather than a tab: no farm is Setup,
	 * a planner running is Planning, drafts alone are Plan review, anything
	 * beyond a draft is Building — with Crew and Activity as its views, kept in
	 * `?pane=` — and a completed farm is Wrap up. `?task=` is a task's own
	 * screen, which the board's cards lead to.
	 */

	let busy = $state(false);
	let editing = $state<Task | 'new' | null>(null);
	let rules = $state(false);
	let settings = $state<FarmSettings>({});
	let newGoal = $state(false);
	let newTitle = $state('');
	let newDescription = $state('');
	/** A finished plan waits on Planning until it is asked for, if the window was away. */
	let holdPlanning = $state(false);
	let wasPlanning = false;
	let now = $state(Date.now());

	const farm = $derived(farmStore.farm);
	const taskId = $derived(page.url.searchParams.get('task'));
	const selected = $derived(farmStore.tasks.find((t) => t.id === taskId));
	const pane = $derived.by(() => {
		const asked = page.url.searchParams.get('pane');
		if (asked === 'crew' || asked === 'agents') return 'Crew';
		if (asked === 'activity') return 'Activity';
		return 'Board';
	});
	const planning = $derived(farmStore.planningRun);
	/** A plan proposed and nothing accepted yet: the screen is Plan review. */
	const onlyDrafts = $derived(
		farmStore.drafts.length > 0 &&
			!farmStore.tasks.some((t) => t.status !== 'draft' && t.status !== 'cancelled')
	);
	const phase = $derived(phaseFor(newGoal ? null : farm, planning, farmStore.usable.length));

	/** Building and its views: the only screens that offer Board · Crew · Activity. */
	const building = $derived(!!farm && !newGoal && !planning && !holdPlanning && !onlyDrafts);

	/** The line under each phase: what it holds, or what it is doing. */
	const phaseSubtitles = $derived.by(() => {
		const crew = `${farmStore.usable.length} ${farmStore.usable.length === 1 ? 'agent' : 'agents'}`;
		if (!farm || newGoal) return [crew, newTitle];
		const tasks = farmStore.tasks.filter((t) => t.status !== 'cancelled');
		const left = tasks.filter((t) => t.status !== 'done').length;
		const plan = planning
			? 'thinking'
			: onlyDrafts
				? `${farmStore.drafts.length} proposed`
				: `${tasks.length} ${tasks.length === 1 ? 'task' : 'tasks'}`;
		const done = farm.status === 'completed';
		const build = done ? `${tasks.length} landed` : phase === 3 ? `${left} to go` : '';
		return [crew, farm.goal.title, plan, build, done ? 'tidy up' : ''];
	});

	// The one clock (FEAT-077): five seconds, and only while something runs.
	$effect(() => {
		if (!farmStore.runs.some((r) => r.outcome.state === 'running')) return;
		const clock = setInterval(() => (now = Date.now()), 5000);
		return () => clearInterval(clock);
	});

	// Visiting the screen opens this repository's farm, whatever it holds.
	$effect(() => {
		const path = repo.info?.path;
		if (path) void farmStore.open(path);
		else farmStore.reset();
		newGoal = false;
	});

	// Move on to Plan review by itself only when the window has focus.
	$effect(() => {
		if (planning) {
			wasPlanning = true;
			holdPlanning = false;
		} else if (wasPlanning) {
			wasPlanning = false;
			holdPlanning =
				typeof document !== 'undefined' && !document.hasFocus() && farmStore.drafts.length > 0;
		}
	});

	// The old Settings pane lives on as the Rules sheet.
	$effect(() => {
		if (page.url.searchParams.get('pane') === 'settings' && !rules) {
			openRules();
			void goto('/farm', { replaceState: true });
		}
	});

	/*
	 * Hand a task to the delight layer when it is seen becoming done (FEAT-077).
	 *
	 * Only the change counts. A task that was already done when the farm was
	 * read is history, and awarding it again on every visit would score the
	 * same work once a session.
	 */
	const lastStatus = new Map<string, TaskStatus>();
	$effect(() => {
		for (const detail of Object.values(farmStore.details)) {
			const key = `${farm?.id}:${detail.task.id}`;
			const before = lastStatus.get(key);
			lastStatus.set(key, detail.task.status);
			if (detail.task.status !== 'done' || !before || before === 'done') continue;
			farmDelight.taskCompleted(detail.task, detail.verification, detail.review);
			if (detail.review && detail.task.implementedBy) {
				farmDelight.reviewCompleted(detail.task.implementedBy, detail.review);
			}
		}
	});

	/** Run one action, refresh, and say so if it failed. One at a time. */
	async function act(message: string, run: () => Promise<unknown>): Promise<boolean> {
		if (busy) return false;
		busy = true;
		farmStore.clearError();
		try {
			await run();
			await farmStore.refresh();
			return true;
		} catch (cause) {
			notice.failed(message, farmStore.fail(cause));
			return false;
		} finally {
			busy = false;
		}
	}

	function openTask(id: string, tab?: string) {
		const query = tab ? `&tab=${encodeURIComponent(tab)}` : '';
		void goto(`/farm?task=${encodeURIComponent(id)}${query}`);
	}

	function openRules() {
		settings = farm
			? {
					autonomy: farm.autonomy,
					verification: [...farm.verification],
					maxParallel: farm.maxParallel,
					maxAttempts: farm.maxAttempts,
					permissions: { ...farm.permissions },
					supplemental: farm.supplemental
				}
			: {};
		rules = true;
	}

	async function saveRules() {
		if (await act('Could not save the rules', () => api.configure(settings))) rules = false;
	}

	async function saveTask(draft: TaskDraft) {
		const target = editing;
		const saved = await act('Could not save the task', () =>
			target && target !== 'new' ? api.editTask(target.id, draft) : api.addTask(draft)
		);
		if (saved) editing = null;
	}

	async function deleteTask() {
		if (!selected) return;
		const id = selected.id;
		const sure = await dialog.confirm({
			title: `Delete ${id}`,
			body: 'The task and its worktree are removed. Commits on an unmerged branch are kept.',
			confirmLabel: 'Delete',
			danger: true
		});
		if (!sure) return;
		if (await act('Could not delete the task', () => api.deleteTask(id))) void goto('/farm');
	}

	/** Setup again, for a new goal — empty, or carried over from Wrap up. */
	function startNew(title = '', description = '') {
		newTitle = title;
		newDescription = description;
		newGoal = true;
		void goto('/farm');
	}
</script>

<div class="farm-journey">
	{#if !taskId}
		<header class="farm-head">
			<div class="farm-head-left">
				<h1>Farm</h1>
				{#if building}
					{#each PANES as name (name)}
						<Chip
							active={pane === name}
							onclick={() => goto(name === 'Board' ? '/farm' : `/farm?pane=${name.toLowerCase()}`)}
						>
							{name}{name === 'Crew' ? ` ${farmStore.usable.length}` : ''}
						</Chip>
					{/each}
				{:else if repo.info && farmStore.loaded}
					<span class="farm-context">
						{#if !farm || newGoal}
							{repo.info.name} · no farm yet
						{:else if onlyDrafts && !planning && !holdPlanning}
							The plan for <strong>{farm.goal.title}</strong>
						{:else}
							{farm.goal.title}
						{/if}
					</span>
				{/if}
			</div>
			{#if repo.info}
				<FarmPhases current={phase} subtitles={phaseSubtitles} />
			{/if}
		</header>
	{/if}

	<div class="farm-body">
		{#if !repo.info}
			<div class="empty">
				<h2>A farm lives in a repository</h2>
				<p>Open one, and put a crew of agents on its work.</p>
				<Btn primary onclick={() => repo.choose()}>Open repository…</Btn>
			</div>
		{:else}
			{#if farmStore.error}
				<div class="card danger-card spread" role="alert">
					<span>{farmStore.error}</span>
					<Btn onclick={() => farmStore.clearError()}>{C.close}</Btn>
				</div>
			{/if}

			{#if farmStore.loading && !farmStore.loaded}
				<Loader label="Reading the farm" />
			{:else if taskId}
				{#if selected}
					{#key selected.id}
						<TaskJourney
							task={selected}
							detail={farmStore.details[selected.id]}
							{now}
							{busy}
							{act}
							initialTab={page.url.searchParams.get('tab')}
							onback={() => goto('/farm')}
							onedit={() => (editing = selected)}
							ondelete={deleteTask}
						/>
					{/key}
				{:else}
					<div class="empty">
						<p>Task {taskId} is not in this farm.</p>
						<Btn onclick={() => goto('/farm')}>Board</Btn>
					</div>
				{/if}
			{:else if newGoal || !farm}
				<Setup
					{busy}
					{act}
					title={newTitle}
					description={newDescription}
					oncreated={() => (newGoal = false)}
				/>
			{:else if planning || holdPlanning}
				<Planning run={planning} {now} {busy} {act} onreview={() => (holdPlanning = false)} />
			{:else if onlyDrafts}
				<PlanReview {busy} {act} onrules={openRules} onedit={(task) => (editing = task)} />
			{:else if pane === 'Crew'}
				<CrewPanel {now} {busy} {act} />
			{:else if pane === 'Activity'}
				<ActivityPanel {now} onopen={openTask} />
			{:else if farm.status === 'completed'}
				<WrapUp {busy} {act} onopen={openTask} onnew={startNew} />
			{:else}
				<Building
					{now}
					{busy}
					{act}
					onopen={openTask}
					onrules={openRules}
					onadd={() => (editing = 'new')}
					onactivity={() => goto('/farm?pane=activity')}
				/>
			{/if}
		{/if}
	</div>

	{#if editing}
		<FarmSheet title={editing === 'new' ? 'New task' : C.edit} onclose={() => (editing = null)}>
			{#key editing === 'new' ? 'new' : editing.id}
				<TaskEditor
					task={editing === 'new' ? null : editing}
					candidates={farmStore.tasks.filter((t) => editing === 'new' || t.id !== editing?.id)}
					{busy}
					onsave={saveTask}
					oncancel={() => (editing = null)}
				/>
			{/key}
		</FarmSheet>
	{/if}

	{#if rules}
		<FarmSheet title={C.rules} onclose={() => (rules = false)}>
			<RulesEditor bind:settings {busy} {act} />
			<div class="actions">
				<Btn onclick={() => (rules = false)}>{C.close}</Btn>
				<Btn primary disabled={busy || !farm} onclick={saveRules}>{C.saveRules}</Btn>
			</div>
		</FarmSheet>
	{/if}
</div>
