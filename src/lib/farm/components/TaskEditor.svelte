<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { lines, TASK_KINDS, TASK_PRIORITIES, text } from '../options';
	import type { Task, TaskDraft, TaskKind, TaskPriority } from '../types';

	/**
	 * Writing or editing a task (FEAT-073).
	 *
	 * The fields are the plan's structured task, in the order somebody writes
	 * one: what it is, what "done" means, where it may touch, what checks it.
	 *
	 * **Allowed paths carries a warning rather than a default.** A task with
	 * none runs on its own, because an undeclared scope is treated as the whole
	 * tree — see the lease module. Guessing a scope would be worse than saying
	 * so: a wrong guess produces the merge conflict the whole mechanism exists
	 * to prevent.
	 */
	interface Props {
		/** The task being edited, or nothing for a new one. */
		task: Task | null;
		initial?: TaskDraft | null;
		/** Tasks that can be depended on — everything except this one. */
		candidates: Task[];
		busy: boolean;
		onsave: (draft: TaskDraft) => void;
		oncancel: () => void;
	}

	let { task, initial = null, candidates, busy, onsave, oncancel }: Props = $props();

	/*
	 * Seeded once, on purpose, and `untrack` says so.
	 *
	 * The form is the user's working copy of the task, not a view of it: a
	 * field that re-derived itself from the prop would throw away what somebody
	 * was typing the moment an event arrived for that task — and events arrive
	 * constantly while a farm runs. The screen mounts this keyed by task
	 * identifier, so choosing a different task builds a new form rather than
	 * mutating this one.
	 */
	let title = $state(untrack(() => task?.title ?? initial?.title ?? ''));
	let description = $state(untrack(() => task?.description ?? initial?.description ?? ''));
	let kind = $state<TaskKind>(untrack(() => task?.kind ?? 'general'));
	let priority = $state<TaskPriority>(untrack(() => task?.priority ?? 'normal'));
	let dependsOn = $state<string[]>(untrack(() => task?.dependsOn ?? []));
	let paths = $state(untrack(() => text(task?.allowedPaths ?? [])));
	let reads = $state(untrack(() => text(task?.reads ?? [])));
	let criteria = $state(untrack(() => text(task?.acceptanceCriteria ?? [])));
	let checks = $state(untrack(() => text(task?.verification ?? [])));
	let overrides = $state(untrack(() => task?.verificationOverrides ?? false));
	let ready = $state(untrack(() => initial?.ready ?? task === null));

	const canSave = $derived(title.trim().length > 0 && !busy);

	function toggleDependency(id: string): void {
		dependsOn = dependsOn.includes(id)
			? dependsOn.filter((entry) => entry !== id)
			: [...dependsOn, id];
	}

	function save(): void {
		onsave({
			title: title.trim(),
			description: description.trim(),
			kind,
			priority,
			dependsOn,
			allowedPaths: lines(paths),
			reads: lines(reads),
			acceptanceCriteria: lines(criteria),
			verification: lines(checks),
			verificationOverrides: overrides,
			assignedAgent: task?.assignedAgent ?? null,
			ready
		});
	}
</script>

<form
	class="editor"
	onsubmit={(event) => {
		event.preventDefault();
		if (canSave) save();
	}}
>
	<label class="field">
		<span class="note">Title</span>
		<input bind:value={title} placeholder="Implement refresh-token rotation" />
	</label>

	<label class="field">
		<span class="note">What the agent needs to know</span>
		<textarea bind:value={description} rows="3"></textarea>
	</label>

	<div class="field">
		<span class="note">Kind</span>
		<div class="chips">
			{#each TASK_KINDS as option (option.id)}
				<Chip active={kind === option.id} onclick={() => (kind = option.id)}>
					{option.label}
				</Chip>
			{/each}
		</div>
	</div>

	<div class="field">
		<span class="note">Priority</span>
		<div class="chips">
			{#each TASK_PRIORITIES as option (option.id)}
				<Chip active={priority === option.id} onclick={() => (priority = option.id)}>
					{option.label}
				</Chip>
			{/each}
		</div>
	</div>

	{#if candidates.length > 0}
		<div class="field">
			<span class="note">Waits for</span>
			<div class="chips">
				{#each candidates as candidate (candidate.id)}
					<Chip
						active={dependsOn.includes(candidate.id)}
						onclick={() => toggleDependency(candidate.id)}
						title={candidate.title}
					>
						{candidate.id}
					</Chip>
				{/each}
			</div>
		</div>
	{/if}

	<label class="field">
		<span class="note">Allowed paths, one per line</span>
		<textarea bind:value={paths} rows="3" placeholder="backend/src/auth/**"></textarea>
		{#if lines(paths).length === 0}
			<span class="note warn">
				With no paths, this task runs on its own — nothing else can run beside it.
			</span>
		{/if}
	</label>

	<!-- FEAT-143: what the task reads and may not change. A slice holds these
	     as well as the allowed paths. -->
	<label class="field">
		<span class="note">Reads, one per line · may not change</span>
		<textarea bind:value={reads} rows="2" placeholder="backend/src/session/**"></textarea>
	</label>

	<label class="field">
		<span class="note">Acceptance criteria, one per line</span>
		<textarea bind:value={criteria} rows="3" placeholder="Refresh tokens are single-use"></textarea>
	</label>

	<label class="field">
		<span class="note">Extra checks for this task, one per line</span>
		<textarea bind:value={checks} rows="2" placeholder="cargo test -p auth"></textarea>
	</label>

	<label class="check">
		<input type="checkbox" bind:checked={overrides} />
		<span class="note">Run only these checks, not the farm's</span>
	</label>

	{#if task === null}
		<label class="check">
			<input type="checkbox" bind:checked={ready} />
			<span class="note">Add to the plan straight away</span>
		</label>
	{/if}

	<div class="actions">
		<Btn primary disabled={!canSave} onclick={save}>{task ? 'Save' : 'Add task'}</Btn>
		<Btn disabled={busy} onclick={oncancel}>Cancel</Btn>
	</div>
</form>

<style>
	.editor {
		flex: 1;
		width: 100%;
		height: 100%;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 9px;
		padding: 16px 24px 24px 16px;
		overflow-y: auto;
		scrollbar-gutter: stable;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
	}

	input:not([type='checkbox']),
	textarea {
		width: 100%;
		padding: 5px 8px;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		background-color: var(--surface-veil);
		color: var(--ink);
		font: inherit;
		font-size: var(--fs-secondary);
	}

	textarea {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		resize: vertical;
	}

	.check {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.warn {
		color: var(--warn);
	}

	.actions {
		display: flex;
		gap: 6px;
		margin-top: 2px;
	}
</style>
