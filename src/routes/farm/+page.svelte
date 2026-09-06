<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import * as api from '$lib/farm/api';
	import AgentCard from '$lib/farm/components/AgentCard.svelte';
	import TaskDetailPanel from '$lib/farm/components/TaskDetail.svelte';
	import TaskEditor from '$lib/farm/components/TaskEditor.svelte';
	import TaskRow from '$lib/farm/components/TaskRow.svelte';
	import ActivityDrawer from '$lib/farm/components/ActivityDrawer.svelte';
	import AgentStrip from '$lib/farm/components/AgentStrip.svelte';
	import ProgressRing from '$lib/farm/components/ProgressRing.svelte';
	import PlanningCard from '$lib/farm/components/PlanningCard.svelte';
	import Starter from '$lib/farm/components/Starter.svelte';
	import * as farmDelight from '$lib/farm/delight';
	import { AUTONOMY_LEVELS, FARM_STATUS_LABELS, PROVIDER_LABELS, quietLine } from '$lib/farm/describe';
	import { lines, text } from '$lib/farm/options';
	import { farmStore } from '$lib/farm/store.svelte';
	import type { Task, TaskDetail, TaskDraft } from '$lib/farm/types';
	import { repo } from '$lib/repo.svelte';
	import { panels } from '$lib/panels.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * Farm (1Q) — supervising a small engineering team inside the Git client
	 * (FEAT-073).
	 *
	 * Three columns, and the split is the plan's product principle rather than
	 * a layout preference. The left column answers *what is the plan*, the
	 * middle answers *what is happening to this task*, and the drawer along the
	 * bottom answers *what has happened* and *what is the agent saying*. A
	 * person who can see all of it at once can supervise; one who has to
	 * navigate between them is reading a log.
	 *
	 * The screen has no timers. Everything arrives on the farm's event channel,
	 * which the store subscribes to — see its header for why polling is the
	 * wrong shape for something that moves at a model's pace. The one exception
	 * is the elapsed clock inside `PlanningCard`, which counts something no
	 * event will ever report.
	 */

	type Pane = 'tasks' | 'agents' | 'settings';

	let pane = $state<Pane>((page.url.searchParams.get('pane') as Pane) || 'tasks');

	$effect(() => {
		const requested = page.url.searchParams.get('pane');
		if (requested === 'tasks' || requested === 'agents' || requested === 'settings') {
			pane = requested;
		}
	});
	let selected = $state<string | null>(null);
	let detail = $state<TaskDetail | null>(null);
	let editing = $state<Task | null | 'new'>(null);
	let busy = $state(false);

	// The settings panel's working copy. Applied on save rather than on every
	// keystroke: a verification command is not valid halfway through being
	// typed, and saving each character would run a write per keystroke.
	let verificationText = $state('');
	let goalTitle = $state('');
	let goalDescription = $state('');

	const farm = $derived(farmStore.farm);
	const tasks = $derived(farmStore.tasks);
	const usable = $derived(farmStore.usable);
	/** The planning run in flight, if there is one. */
	const planning = $derived(farmStore.planningRun);
	/** Tasks a planner proposed that nobody has accepted or discarded yet. */
	const drafts = $derived(farmStore.drafts);
	const running = $derived(farmStore.runs.filter((run) => run.outcome.state === 'running'));

	/**
	 * One clock for the screen (FEAT-077).
	 *
	 * Elapsed times and "this has said nothing for six minutes" are the only
	 * things here that change without an event, and they should change
	 * together: two intervals would drift and show two different nows in one
	 * header. It ticks only while something is running.
	 */
	let now = $state(Date.now());
	$effect(() => {
		if (running.length === 0 && !planning) return;
		const tick = setInterval(() => (now = Date.now()), 5000);
		return () => clearInterval(tick);
	});

	/** What the ring shows. */
	const tally = $derived({
		done: tasks.filter((task) => task.status === 'done').length,
		running: tasks.filter((task) => task.status === 'running' || task.status === 'verification')
			.length,
		blocked: tasks.filter((task) => task.status === 'blocked' || task.status === 'failed').length,
		total: tasks.length
	});

	/**
	 * Hand a finished task to the delight layer, once (FEAT-077).
	 *
	 * Watched here rather than pushed from the store, because the panel is
	 * where verification and review are already loaded — and the event wants
	 * all three. `counted` is what stops a refresh from awarding the same task
	 * twice; it is per session, like the rest of the screen's state.
	 */
	const counted = new Set<string>();
	$effect(() => {
		const finished = detail;
		if (!finished || finished.task.status !== 'done' || counted.has(finished.task.id)) return;
		counted.add(finished.task.id);
		farmDelight.taskCompleted(finished.task, finished.verification, finished.review);
		if (finished.review && finished.task.implementedBy) {
			farmDelight.reviewCompleted(finished.task.implementedBy, finished.review);
		}
	});

	/**
	 * Which proposed tasks are being kept (FEAT-075).
	 *
	 * A plan arrives as a set of drafts, and accepting one used to mean opening
	 * each task and pressing a button in its panel — eight clicks for an
	 * eight-task plan, with nothing on the list itself saying they were waiting
	 * for a decision. Everything is picked by default, because a plan that was
	 * asked for is usually a plan that is wanted.
	 */
	let picked = $state<string[]>([]);
	let pickedFor = $state('');

	$effect(() => {
		const signature = drafts.map((task) => task.id).join(',');
		if (signature === pickedFor) return;
		pickedFor = signature;
		picked = drafts.map((task) => task.id);
	});

	function toggle(id: string): void {
		picked = picked.includes(id) ? picked.filter((kept) => kept !== id) : [...picked, id];
	}

	async function acceptPlan(): Promise<void> {
		const ids = [...picked];
		if (ids.length === 0) return;
		await act('Could not accept the plan', () => api.readyTasks(ids));
	}

	async function discardPlan(): Promise<void> {
		const ids = [...picked];
		if (ids.length === 0) return;
		const agreed = await dialog.confirm({
			title: ids.length === 1 ? `Discard ${ids[0]}` : `Discard ${ids.length} proposed tasks`,
			body:
				'They were proposed by an agent and never started, so nothing is lost but the ' +
				'proposal. Planning again produces a new one.',
			confirmLabel: 'Discard',
			danger: true
		});
		if (!agreed) return;
		await act('Could not discard the plan', () => api.discardTasks(ids));
	}
	/** The activity list: everything except the transcript flood, which has its own tab. */
	const activity = $derived(farmStore.activity.filter((event) => event.kind !== 'agentOutput'));

	/**
	 * Follow the open repository.
	 *
	 * The farm belongs to a repository, so opening one points the farm at it and
	 * closing one lets it go. The running agents are not stopped — they are in
	 * worktrees of their own and their work is on disk.
	 */
	$effect(() => {
		const path = repo.info?.path ?? null;
		if (path) {
			void farmStore.open(path);
		} else {
			farmStore.reset();
		}
	});

	// The settings fields are seeded from the farm when it arrives, and left
	// alone afterwards so typing is never overwritten by an event.
	let seeded = $state<string | null>(null);
	$effect(() => {
		if (farm && seeded !== farm.id) {
			seeded = farm.id;
			verificationText = text(farm.verification);
			goalTitle = farm.goal.title;
			goalDescription = farm.goal.description;
		}
	});

	$effect(() => {
		const id = selected;
		if (!id) {
			detail = null;
			return;
		}
		// Re-read whenever the task changes, so the panel follows the farm.
		void farmStore.tasks.find((task) => task.id === id)?.updatedMs;
		void api
			.taskDetail(id)
			.then((found) => (detail = found))
			.catch(() => (detail = null));
	});

	/** Run an action, keeping the screen honest about failure. */
	async function act(what: string, run: () => Promise<unknown>): Promise<void> {
		busy = true;
		try {
			await run();
			await farmStore.refresh();
		} catch (cause) {
			notice.failed(what, farmStore.fail(cause));
		} finally {
			busy = false;
		}
	}

	async function createFarm(): Promise<void> {
		await startFarm(goalTitle.trim(), goalDescription.trim());
	}

	/**
	 * Start a farm from a goal typed anywhere on the screen.
	 *
	 * The starter page and the Settings pane both ask for the same two fields,
	 * so they call the same function rather than each having their own idea of
	 * what a blank title means. The fields are seeded back into Settings so the
	 * goal a person just typed is the goal Settings shows.
	 */
	async function startFarm(title: string, description: string): Promise<void> {
		if (!title) return;
		goalTitle = title;
		goalDescription = description;
		await act('Could not start a farm', async () => {
			await api.create(title, description);
			// Verification is a property of a farm, so a command typed in
			// Settings before there was one had nowhere to go and was silently
			// dropped by the create. It is applied here instead, in the same
			// action, because the alternative is a screen that accepted a
			// setting and did not keep it.
			const commands = lines(verificationText);
			if (commands.length > 0) await api.configure({ verification: commands });
		});
		pane = 'tasks';
	}

	async function saveTask(draft: TaskDraft): Promise<void> {
		const target = editing;
		await act('Could not save the task', () =>
			target && target !== 'new' ? api.editTask(target.id, draft) : api.addTask(draft)
		);
		editing = null;
	}

	async function deleteTask(id: string): Promise<void> {
		const agreed = await dialog.confirm({
			title: `Delete ${id}`,
			body:
				"The task and its worktree are removed. Commits on its branch are kept: an unmerged " +
				'branch survives, and stays visible in the graph.',
			confirmLabel: 'Delete',
			danger: true
		});
		if (!agreed) return;
		await act('Could not delete the task', () => api.deleteTask(id));
		if (selected === id) selected = null;
	}

	async function saveSettings(): Promise<void> {
		await act('Could not save the farm', () =>
			api.configure({
				verification: lines(verificationText),
				goalTitle: goalTitle.trim(),
				goalDescription: goalDescription.trim()
			})
		);
		notice.ok('Farm settings saved');
	}

	async function writePolicy(): Promise<void> {
		try {
			const path = await api.writePolicy();
			notice.ok(`Wrote ${path}`);
			await farmStore.refresh();
		} catch (cause) {
			notice.failed('Could not write AGENTS.md', farmStore.fail(cause));
		}
	}

	function openDiff(branch: string): void {
		// The graph is the diff. Being a Git client is the advantage here.
		void goto(`/?ref=${encodeURIComponent(branch)}`);
	}
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Farm</span>
			{#if farm}
				<Chip title="What the farm is doing">{FARM_STATUS_LABELS[farm.status]}</Chip>
				<ProgressRing
					done={tally.done}
					running={tally.running}
					blocked={tally.blocked}
					total={tally.total}
				/>
			{/if}
		</div>
		<div class="right">
			<Chip active={pane === 'tasks'} onclick={() => (pane = 'tasks')}>Tasks</Chip>
			<Chip active={pane === 'agents'} onclick={() => (pane = 'agents')}>
				Agents<span class="note">&nbsp;{usable.length}</span>
			</Chip>
			<Chip active={pane === 'settings'} onclick={() => (pane = 'settings')}>Settings</Chip>
		</div>
	</header>

	{#if planning}
		<PlanningCard
			lines={farmStore.planning}
			startedMs={planning.startedMs}
			{busy}
			oncancel={() => act('Could not stop the planner', api.cancelPlan)}
		/>
	{/if}

	<AgentStrip
		runs={farmStore.runs}
		byId={farmStore.byId}
		{now}
		onselect={(id) => {
			selected = id;
			editing = null;
		}}
	/>

	<div class="body">
		{#if repo.info === null}
			<!--
				A farm belongs to a repository, and this screen is now the first
				thing in the rail — so the state a new window opens in is this
				one, and "No repository open." was the whole of it. It says what
				a farm is for and offers the one action that makes the screen
				work, which is also the action the rail stops offering once a
				repository is open.
			-->
			<div class="empty">
				<div class="nothing">
					<h2 class="heading">A farm lives in a repository</h2>
					<p class="note">
						Open one and the farm follows it: its goal, its tasks and its agents are
						stored with that repository, and the branches its agents produce are the
						branches you read in the graph.
					</p>
					<div class="actions">
						<Btn primary onclick={() => repo.choose()}>Open repository…</Btn>
					</div>
				</div>
			</div>
		{:else if !farmStore.loaded && farmStore.loading}
			<div class="empty"><p class="note">Reading the farm…</p></div>
		{:else if pane === 'agents'}
			<section class="pane single">
				<div class="row-between">
					<h2 class="heading">Agents on this machine</h2>
					<Btn disabled={busy} onclick={() => act('Detection failed', api.detectAgents)}>
						Look again
					</Btn>
				</div>
				<p class="note">
					Spagitty runs these; it does not contain them. What is configured here is which of
					them this repository should use, and what each is good at.
				</p>

				{#each farmStore.agents as agent (agent.definition.id)}
					<AgentCard
						{agent}
						onsave={(definition) =>
							act('Could not save the agent', () => api.saveAgent(definition))}
						onremove={(id) => act('Could not forget the agent', () => api.removeAgent(id))}
					/>
				{/each}

				{#if farmStore.agents.length === 0}
					<p class="note">
						Nothing detected yet. Claude Code, Codex, Cursor and Oh My Pi are found on
						<span class="mono">PATH</span>.
					</p>
				{/if}

				{#if farmStore.undetected.length > 0}
					<p class="note">
						Not found: {farmStore.undetected.map((id) => PROVIDER_LABELS[id]).join(', ')}.
					</p>
				{/if}

				{#if farmStore.scoreboard.length > 0}
					<h2 class="heading">What has happened here</h2>
					<p class="note">
						Counted in this repository, not a claim about which model is better.
					</p>
					<div class="table">
						{#each farmStore.scoreboard as row (row.agent)}
							<div class="score">
								<span>{row.agent}</span>
								<span class="note">{row.completed} completed</span>
								<span class="note">{row.failed} failed</span>
								<span class="note">{row.changesRequested} sent back</span>
								<span class="note">
									{row.successRate === null
										? '—'
										: `${Math.round(row.successRate * 100)}%`}
								</span>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{:else if pane === 'settings'}
			<section class="pane single">
				<h2 class="heading">{farm ? 'The goal' : 'Start a farm'}</h2>
				<label class="field">
					<span class="note">What is this farm for?</span>
					<input bind:value={goalTitle} placeholder="e.g. Implement dark mode support" />
				</label>
				<label class="field">
					<span class="note">Anything the agents should know</span>
					<textarea bind:value={goalDescription} rows="3" placeholder="Constraints, conventions, files to avoid"></textarea>
				</label>

				{#if !farm}
					<div class="actions">
						<Btn primary disabled={busy || goalTitle.trim().length === 0} onclick={createFarm}>
							Start a farm
						</Btn>
					</div>
				{/if}

				<h2 class="heading">How much it may do on its own</h2>
				<div class="levels">
					{#each AUTONOMY_LEVELS as level (level.id)}
						<button
							class="level"
							class:on={(farm ? farm.autonomy : 'semiAuto') === level.id}
							disabled={busy}
							onclick={() =>
								farm && act('Could not change the autonomy level', () =>
									api.configure({ autonomy: level.id, permissions: { ...farm.permissions, merge: level.id === 'auto' || level.id === 'yolo' } })
								)}
						>
							<span class="level-name">{level.label}</span>
							<span class="note">{level.detail}</span>
						</button>
					{/each}
				</div>

				{#if farm}
					<h2 class="heading">Agents at once</h2>
					<p class="note">
						What keeps a farm supervisable is how many run at once, not how many tasks
						there are. Every one of them is a model you are paying for and a worktree on
						your disk.
					</p>
					<div class="chips">
						{#each [1, 2, 3, 4, 5, 6, 7, 8] as count (count)}
							<Chip
								active={farm.maxParallel === count}
								disabled={busy}
								onclick={() =>
									act('Could not change the limit', () =>
										api.configure({ maxParallel: count })
									)}
							>
								{count}
							</Chip>
						{/each}
					</div>

					<h2 class="heading">Attempts before a person is needed</h2>
					<p class="note">
						A task sent back by verification or review is tried again, up to this many
						times. The first failure is normal, the second is usually a bad prompt, and
						the third is a task nobody has understood yet.
					</p>
					<div class="chips">
						{#each [1, 2, 3, 5, 10] as count (count)}
							<Chip
								active={farm.maxAttempts === count}
								disabled={busy}
								onclick={() =>
									act('Could not change the attempts', () =>
										api.configure({ maxAttempts: count })
									)}
							>
								{count}
							</Chip>
						{/each}
					</div>
				{/if}

				<h2 class="heading">Verification</h2>
				<p class="note">
					Run against every task's worktree before it can be accepted. With none, a task
					reaches review having been checked by nobody, and the screen says so.
				</p>
				<label class="field">
					<textarea bind:value={verificationText} rows="3" placeholder="cargo test"></textarea>
				</label>

				<h2 class="heading">Repository rules</h2>
				{#if farmStore.policy.sources.length > 0}
					<p class="note">
						Read from {farmStore.policy.sources.map((source) => source.path).join(', ')} and
						attached to every prompt.
					</p>
				{:else}
					<p class="note">
						This repository has no agent rules file, so agents follow whatever
						conventions they find in the code.
					</p>
					<div class="actions">
						<Btn disabled={busy} onclick={writePolicy}>Write a starter AGENTS.md</Btn>
					</div>
				{/if}

				{#if farm}
					<div class="actions">
						<Btn primary disabled={busy} onclick={saveSettings}>Save</Btn>
					</div>

					{#if farmStore.stale.length > 0}
						<h2 class="heading">Left behind</h2>
						<p class="note">
							{farmStore.stale.length}
							{farmStore.stale.length === 1 ? 'worktree' : 'worktrees'} from tasks this farm
							no longer has. Anything with uncommitted work is kept.
						</p>
						<div class="actions">
							<Btn
								disabled={busy}
								onclick={() =>
									act('Could not clean up', async () => {
										await api.sweep();
										// The leftovers list is not part of a
										// snapshot any more, so the action that
										// changes it asks for it again.
										await farmStore.leftovers();
									})}
							>
								Clean up
							</Btn>
						</div>
					{/if}
				{/if}
			</section>
		{:else if !farm}
			<section class="pane single wide">
				<Starter
					ready={usable}
					undetected={farmStore.undetected}
					policySources={farmStore.policy.sources.map((source) => source.path)}
					verificationCount={lines(verificationText).length}
					{busy}
					onstart={startFarm}
					ondetect={() => act('Detection failed', api.detectAgents)}
					onwritePolicy={writePolicy}
					onsettings={() => (pane = 'settings')}
				/>
			</section>
		{:else}
			<section class="pane list">
				<div class="row-between">
					<h2 class="heading">{farm.goal.title}</h2>
					<div class="chips">
						{#if farm.status === 'running'}
							<Btn disabled={busy} onclick={() => act('Could not pause', api.pause)}>Pause</Btn>
						{:else}
							<Btn
								primary
								disabled={busy || tasks.length === 0}
								onclick={() => act('Could not start the farm', api.start)}
							>
								Start
							</Btn>
						{/if}
						<Btn
							disabled={busy || usable.length === 0}
							title={usable.length === 0 ? 'No agent is available' : 'Ask an agent to plan'}
							onclick={() => act('Could not plan', () => api.plan(null))}
						>
							Plan it
						</Btn>
						<Btn disabled={busy} onclick={() => (editing = 'new')}>Add task</Btn>
					</div>
				</div>

				{#if drafts.length > 0}
					<!--
						A plan is one decision, so it gets one band rather than a
						button inside each task's panel.
					-->
					<div class="proposed">
						<span class="note">
							{drafts.length}
							{drafts.length === 1 ? 'task was proposed' : 'tasks were proposed'} and
							nothing has started them.
						</span>
						<div class="chips">
							<Chip
								onclick={() =>
									(picked =
										picked.length === drafts.length ? [] : drafts.map((task) => task.id))}
							>
								{picked.length === drafts.length ? 'None' : 'All'}
							</Chip>
							<Btn primary disabled={busy || picked.length === 0} onclick={acceptPlan}>
								Add {picked.length} to the plan
							</Btn>
							<Btn danger quiet disabled={busy || picked.length === 0} onclick={discardPlan}>
								Discard
							</Btn>
						</div>
					</div>
				{/if}

				{#if farmStore.needsYou.length > 0}
					<p class="note attention">
						{farmStore.needsYou.length}
						{farmStore.needsYou.length === 1 ? 'task needs' : 'tasks need'} you.
					</p>
				{/if}

				{#if tasks.length === 0}
					<p class="note">
						No tasks yet. Write one, or ask an agent to break the goal into some.
					</p>
				{:else}
					<div class="tasks">
						{#each farmStore.outline as row (row.task.id)}
							{@const task = row.task}
							<TaskRow
								{task}
								depth={row.depth}
								progress={row.total > 0 ? { done: row.done, total: row.total } : null}
								selected={selected === task.id}
								byId={farmStore.byId}
								blocked={quietLine(
									running.find((run) => run.task === task.id) ?? null,
									now
								) ?? farmStore.waitingFor(task.id)}
								pick={task.status === 'draft'
									? {
											on: picked.includes(task.id),
											ontoggle: () => toggle(task.id)
										}
									: null}
								onselect={(id) => {
									selected = id;
									editing = null;
								}}
							/>
						{/each}
					</div>
				{/if}
			</section>

			<section class="pane side">
				{#if editing !== null}
					{#key editing === 'new' ? 'new' : editing.id}
						<TaskEditor
							task={editing === 'new' ? null : editing}
							candidates={tasks.filter(
								(task) => editing === 'new' || task.id !== (editing as Task).id
							)}
							{busy}
							onsave={saveTask}
							oncancel={() => (editing = null)}
						/>
					{/key}
				{:else if detail}
					<TaskDetailPanel
						{detail}
						agents={usable}
						transcript={farmStore.transcript(detail.task.id)}
						{busy}
						onrun={(agent) =>
							act('Could not run the task', () => api.runTask(detail!.task.id, agent))}
						onstop={() => act('Could not stop the task', () => api.cancelTask(detail!.task.id))}
						onretry={() => act('Could not retry', () => api.retryTask(detail!.task.id))}
						onverify={() => act('Could not verify', () => api.verifyTask(detail!.task.id))}
						onreview={() => act('Could not request a review', () => api.reviewTask(detail!.task.id))}
						onmerge={() => act('Could not merge', () => api.mergeTask(detail!.task.id))}
						onready={() => act('Could not add to the plan', () => api.readyTask(detail!.task.id))}
						ondecompose={() =>
							act('Could not break the task down', () =>
								api.decompose(detail!.task.id, null)
							)}
						children={farmStore.outline.find((row) => row.task.id === detail!.task.id) ??
							null}
						onedit={() => (editing = detail!.task)}
						ondelete={() => deleteTask(detail!.task.id)}
						onopenDiff={openDiff}
					/>
				{:else}
					<div class="empty"><p class="note">Pick a task.</p></div>
				{/if}
			</section>
		{/if}
	</div>

	{#if farm}
		{#if !panels.isHidden('farmLog')}
			<Splitter panel="farmLog" label="Resize the log" />
		{/if}
		<ActivityDrawer
			events={activity}
			{tasks}
			transcript={(id) => farmStore.transcript(id)}
			onwholeLog={async (id) => {
				// The most recent run of that task, whose log is the one on disk.
				const run = farmStore.runs.filter((entry) => entry.task === id).at(-1);
				return run ? await api.transcript(run.id, id) : '';
			}}
			selected={selected}
			planning={planning !== null}
			collapsed={panels.isHidden('farmLog')}
			ontoggle={() => panels.toggleHidden('farmLog')}
		/>
	{/if}
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		min-height: 0;
		height: 100%;
		width: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.body {
		flex: 1;
		min-height: 0;
		height: 100%;
		width: 100%;
		display: flex;
		overflow: hidden;
	}

	.pane {
		min-width: 0;
		min-height: 0;
		height: 100%;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.single {
		flex: 1;
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow-y: auto;
		padding: 16px 28px 24px 20px;
		scrollbar-gutter: stable;
	}

	.single > * {
		max-width: 76ch;
	}

	/* The starter page sets its own measure and centres itself, so the pane
	   must not clamp it to the Settings pane's column. */
	.wide > :global(*) {
		max-width: none;
	}

	.list {
		flex: 1;
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow-y: auto;
		padding: 16px 24px 24px 16px;
		scrollbar-gutter: stable;
	}

	/* The detail column. Fixed-ish so the task list does not reflow every time
	   a longer note arrives — a list that jumps while an agent talks is
	   unreadable. */
	.side {
		flex: none;
		width: 30rem;
		max-width: 45%;
		height: 100%;
		min-height: 0;
		border-left: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	/* The no-repository state: a short column rather than one grey line. */
	.nothing {
		display: flex;
		flex-direction: column;
		gap: 8px;
		align-items: center;
		max-width: 46ch;
	}

	.empty {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px 20px;
		text-align: center;
		flex: 1;
		height: 100%;
		min-height: 0;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: 600;
	}

	.row-between {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		flex-wrap: wrap;
	}

	.chips,
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		align-items: center;
	}

	.tasks {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.attention {
		color: var(--warn);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.field input,
	.field textarea {
		width: 100%;
		padding: 5px 8px;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		background-color: var(--surface-veil);
		color: var(--ink);
		font: inherit;
		font-size: var(--fs-secondary);
	}

	.field textarea {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		resize: vertical;
	}

	.levels {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	/*
	 * A row per level rather than a slider.
	 *
	 * A slider would say "more" and "less", and the thing that changes between
	 * levels is *where the human is*, which is a sentence and not a magnitude.
	 */
	.level {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1px;
		padding: 7px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-row);
		background-color: transparent;
		text-align: left;
		transition:
			background-color var(--t-fast) var(--ease),
			border-color var(--t-fast) var(--ease);
	}

	.level:hover {
		background-color: var(--stripe);
	}

	.level.on {
		background-color: var(--selection);
		border-color: color-mix(in srgb, var(--accent) 35%, transparent);
	}

	.level-name {
		font-weight: 550;
	}

	.table {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.score {
		display: flex;
		gap: 10px;
		align-items: baseline;
		padding: 4px 8px;
		border-radius: var(--r-row);
		font-size: var(--fs-secondary);
	}

	.score:nth-child(odd) {
		background-color: var(--stripe);
	}

	/* The band that turns a proposed plan into one decision. */
	.proposed {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		flex-wrap: wrap;
		padding: 8px 10px;
		border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
		border-radius: var(--r-row);
		background-color: var(--selection);
	}

</style>
