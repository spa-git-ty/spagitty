<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { FARM_COPY as C, whereYouComeIn } from '../describe';
	import { LEVELS } from '../wizard';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import type {
		Autonomy,
		ConflictResolver,
		FarmSettings,
		Permissions,
		SupplementalPolicy,
		WorkspaceMode
	} from '../types';

	/**
	 * The farm's rules, in Setup and in the Rules sheet (FEAT-111, FEAT-109).
	 *
	 * Where the person comes in, what every task must pass, how many agents
	 * work at once and how many tries a task gets. The rarer rules — CodeRabbit,
	 * permissions, leftover worktrees — wait behind `More rules`.
	 *
	 * The form is a working copy seeded once from `settings` and written back
	 * whole on every change, so the parent decides when to save it.
	 */
	interface Props {
		settings?: FarmSettings;
		busy?: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		/** Offer *Where each task works* (FEAT-143). */
		workspaces?: boolean;
	}

	let { settings = $bindable({}), busy = false, act, workspaces = false }: Props = $props();

	const SPACES: { id: WorkspaceMode; name: string; detail: string; isNew: boolean }[] = [
		{ id: 'worktree', name: 'A worktree per task', detail: 'The whole repository, checked out for the task. What you have today.', isNew: false },
		{ id: 'slice', name: 'A slice per task', detail: 'Only the paths the task may touch. Checks still run on the whole tree.', isNew: true },
		{ id: 'copy', name: 'A copy per task', detail: 'A repository of its own with no remote. Its commits come in only after they are checked against the paths it may touch.', isNew: true }
	];

	const AT_ONCE = [1, 2, 3, 4];
	const TRIES = [1, 2, 3, 5];

	const PERMISSIONS: { key: keyof Permissions; label: string }[] = [
		{ key: 'writeFiles', label: 'Write files' },
		{ key: 'runCommands', label: 'Run commands' },
		{ key: 'network', label: 'Use the network' },
		{ key: 'commit', label: 'Commit' },
		{ key: 'push', label: 'Push' },
		{ key: 'merge', label: 'Merge' },
		{ key: 'deleteBranch', label: 'Delete branches' }
	];

	const DEFAULT_PERMISSIONS: Permissions = {
		writeFiles: true,
		runCommands: true,
		network: false,
		commit: true,
		push: false,
		merge: false,
		deleteBranch: false
	};

	const DEFAULT_SUPPLEMENTAL: SupplementalPolicy = {
		mode: 'off',
		provider: 'spagitty.coderabbit/review',
		threshold: 'medium',
		maxRepairs: 2,
		revision: 1
	};

	let autonomy = $state<Autonomy>(untrack(() => settings.autonomy ?? 'semiAuto'));
	let checks = $state<string[]>(untrack(() => [...(settings.verification ?? [])]));
	let parallel = $state(untrack(() => settings.maxParallel ?? 2));
	let attempts = $state(untrack(() => settings.maxAttempts ?? 3));
	let permissions = $state<Permissions>(
		untrack(() => ({ ...DEFAULT_PERMISSIONS, ...settings.permissions }))
	);
	let supplemental = $state<SupplementalPolicy>(
		untrack(() => ({ ...DEFAULT_SUPPLEMENTAL, ...settings.supplemental }))
	);
	let workspace = $state<WorkspaceMode>(untrack(() => settings.workspace ?? 'worktree'));
	/** Who resolves a conflict (FEAT-146): you, the task's agent, or a command. */
	let resolver = $state<ConflictResolver['by']>(untrack(() => settings.conflicts?.by ?? 'ask'));
	let resolverCommand = $state(
		untrack(() => (settings.conflicts?.by === 'tool' ? settings.conflicts.command : ''))
	);
	const conflicts = $derived<ConflictResolver>(
		resolver === 'tool' ? { by: 'tool', command: resolverCommand.trim() } : { by: resolver }
	);

	$effect(() => {
		settings = {
			...untrack(() => settings),
			autonomy,
			permissions: { ...permissions },
			supplemental: { ...supplemental },
			verification: checks.map((check) => check.trim()).filter(Boolean),
			maxParallel: parallel,
			maxAttempts: attempts,
			workspace,
			conflicts
		};
	});

	/**
	 * Choosing a level that merges by itself grants the merge permission, and
	 * choosing one that does not takes it back. The backend merges only when
	 * both say so, and a person who picks Automatic means it.
	 */
	function choose(level: Autonomy) {
		autonomy = level;
		permissions = { ...permissions, merge: level === 'auto' || level === 'yolo' };
	}

	const hasPolicy = $derived(farmStore.policy.sources.some((s) => s.path.endsWith('AGENTS.md')));
	const atOnce = $derived(AT_ONCE.includes(parallel) ? AT_ONCE : [...AT_ONCE, parallel]);
	const tries = $derived(TRIES.includes(attempts) ? TRIES : [...TRIES, attempts].sort());
</script>

<div class="rules">
	<section>
		<h4>{C.howMuch}</h4>
		<!-- One segmented row, and the line under it says where the person
		     comes in at the level chosen (FEAT-141). -->
		<div class="levels" role="radiogroup" aria-label={C.howMuch}>
			{#each LEVELS as level (level.id)}
				<button
					class="level"
					class:on={autonomy === level.id}
					role="radio"
					aria-checked={autonomy === level.id}
					disabled={busy}
					onclick={() => choose(level.id)}
				>
					{level.label}
				</button>
			{/each}
		</div>
		<p class="come-in"><strong>You come in</strong> <span>{whereYouComeIn(autonomy)}</span></p>
	</section>

	<section>
		<h4>{C.mustPass}</h4>
		<ul class="checks">
			{#each checks as _, i (i)}
				<li>
					<span class="prompt">$</span>
					<input class="mono" aria-label="Check {i + 1}" bind:value={checks[i]} disabled={busy} />
					<button
						class="remove"
						aria-label="Remove this check"
						title="Remove"
						disabled={busy}
						onclick={() => (checks = checks.filter((_, j) => j !== i))}
					>
						<Icon name="close" />
					</button>
				</li>
			{/each}
		</ul>
		<Chip disabled={busy} onclick={() => (checks = [...checks, ''])}>
			<Icon name="plus" />
			{C.addCommand}
		</Chip>
	</section>

	<div class="pickers">
		<section>
			<h4>{C.atOnce}</h4>
			<div class="segmented" role="radiogroup" aria-label={C.atOnce}>
				{#each atOnce as n (n)}
					<button
						role="radio"
						aria-checked={parallel === n}
						class:on={parallel === n}
						disabled={busy}
						onclick={() => (parallel = n)}>{n}</button
					>
				{/each}
			</div>
		</section>
		<section>
			<h4>{C.triesBefore}</h4>
			<div class="segmented" role="radiogroup" aria-label={C.triesBefore}>
				{#each tries as n (n)}
					<button
						role="radio"
						aria-checked={attempts === n}
						class:on={attempts === n}
						disabled={busy}
						onclick={() => (attempts = n)}>{n}</button
					>
				{/each}
			</div>
		</section>
	</div>

	{#if workspaces}
		<section>
			<h4>Where each task works</h4>
			<div class="spaces" role="radiogroup" aria-label="Where each task works">
				{#each SPACES as space (space.id)}
					<button
						class="space"
						class:on={workspace === space.id}
						role="radio"
						aria-checked={workspace === space.id}
						disabled={busy}
						onclick={() => (workspace = space.id)}
					>
						<strong>{space.name}{#if space.isNew}<span class="new">new</span>{/if}</strong>
						<span>{space.detail}</span>
					</button>
				{/each}
			</div>
		</section>
	{/if}

	<p class="policy muted">
		{#if hasPolicy}
			<span class="ok">{C.policyAttached}</span>
		{:else}
			{C.policyMissing}
			<Btn disabled={busy} onclick={() => act('Could not write AGENTS.md', api.writePolicy)}>
				{C.policyWrite}
			</Btn>
		{/if}
		· Extra CodeRabbit review: {supplemental.mode === 'off' ? 'off' : supplemental.mode}
	</p>

	<details>
		<summary>{C.moreRules}</summary>
		<div class="more">
			<section>
				<h4>When a merge conflicts</h4>
				<div class="inline">
					<label>
						Resolved by
						<select bind:value={resolver} disabled={busy}>
							<option value="ask">You, in Merger</option>
							<option value="agent">The task's agent</option>
							<option value="tool">A command</option>
						</select>
					</label>
					{#if resolver === 'tool'}
						<label>
							Command
							<input class="mono" bind:value={resolverCommand} placeholder="mergiraf solve" disabled={busy} />
						</label>
					{/if}
				</div>
				<p class="muted resolver-note">
					{#if resolver === 'ask'}
						The task stops and waits for you. Nothing is written until you resolve it.
					{:else}
						It is resolved in the task's own workspace, never in yours, and the task's checks
						and review run again before it is merged.{#if resolver === 'tool'} The command is
							given the conflicted paths and must leave no conflict markers.{/if}
					{/if}
				</p>
			</section>
			<section>
				<h4>{C.supplemental}</h4>
				<div class="inline">
					<label>
						Review
						<select bind:value={supplemental.mode} disabled={busy}>
							<option value="off">Off</option>
							<option value="advisory">Advisory</option>
							<option value="required">Required</option>
						</select>
					</label>
					{#if supplemental.mode !== 'off'}
						<label>
							Stops on
							<select bind:value={supplemental.threshold} disabled={busy}>
								<option value="low">Low and above</option>
								<option value="medium">Medium and above</option>
								<option value="high">High and above</option>
								<option value="critical">Critical only</option>
							</select>
						</label>
						<label>
							Repair attempts
							<input
								type="number"
								min="0"
								max="10"
								bind:value={supplemental.maxRepairs}
								disabled={busy}
							/>
						</label>
					{/if}
				</div>
			</section>

			<section>
				<h4>{C.permission}</h4>
				<div class="inline">
					{#each PERMISSIONS as permission (permission.key)}
						<Chip
							active={permissions[permission.key]}
							disabled={busy}
							onclick={() =>
								(permissions = {
									...permissions,
									[permission.key]: !permissions[permission.key]
								})}
						>
							{permission.label}
						</Chip>
					{/each}
				</div>
			</section>

			<section>
				<h4>{C.leftovers} · {farmStore.stale.length}</h4>
				{#each farmStore.stale as leftover (leftover.path)}
					<p class="mono muted">{leftover.path}</p>
				{/each}
				<Btn
					disabled={busy || !farmStore.stale.length}
					onclick={() =>
						act('Could not remove leftovers', async () => {
							await api.sweep();
							await farmStore.leftovers();
						})}
				>
					{C.removeWorktrees}
				</Btn>
			</section>

			{#if farmStore.policy.text}
				<details>
					<summary>Repository rules</summary>
					<pre>{farmStore.policy.text}</pre>
				</details>
			{/if}
		</div>
	</details>
</div>

<style>
	.rules {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	h4 {
		margin: 0 0 8px;
		font-size: var(--fs-secondary);
		font-weight: 400;
		color: var(--muted);
	}

	.levels {
		display: grid;
		grid-template-columns: repeat(5, minmax(0, 1fr));
		gap: 4px;
		padding: 4px;
		border: 1px solid var(--soft);
		border-radius: var(--r-floating);
		background: var(--sunken);
	}

	.level {
		height: 40px;
		border: 0;
		border-radius: var(--r-button);
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: var(--fs-secondary);
		cursor: pointer;
	}

	.level:hover:not(:disabled):not(.on) {
		color: var(--ink);
		background: var(--hover);
	}

	.level.on {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	.come-in {
		display: flex;
		align-items: baseline;
		gap: 10px;
		margin: 10px 0 0;
		padding: 10px 14px;
		border-radius: var(--r-panel);
		background: color-mix(in srgb, var(--accent) 8%, transparent);
	}

	.come-in strong {
		flex: none;
		color: var(--accent);
	}

	.spaces {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 10px;
	}

	.space {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 6px;
		padding: 14px 16px;
		text-align: left;
		color: var(--ink);
		border: 1.5px solid var(--soft);
		border-radius: var(--r-floating);
		background: var(--surface-veil);
		font: inherit;
		cursor: pointer;
	}

	.space strong {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.space span {
		color: var(--muted);
		font-size: var(--fs-secondary);
		line-height: 1.4;
	}

	.space.on {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 8%, transparent);
	}

	.new {
		padding: 2px 8px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--lane-4) 18%, transparent);
		color: var(--lane-4);
		font-size: calc(var(--fs-mono) * 0.92);
		font-weight: 500;
	}

	.checks {
		list-style: none;
		margin: 0 0 8px;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.checks li {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.checks input {
		flex: 1;
	}

	.prompt {
		font-family: var(--font-mono);
		color: var(--muted);
	}

	.remove {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: none;
		border-radius: var(--r-button);
		background: none;
		color: var(--muted);
		cursor: pointer;
	}

	.remove:hover:not(:disabled) {
		color: var(--ink);
		background: var(--hover);
	}

	.pickers {
		display: flex;
		gap: 32px;
		flex-wrap: wrap;
	}

	.segmented {
		display: inline-flex;
		padding: 2px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
	}

	.segmented button {
		min-width: 36px;
		padding: 4px 10px;
		border: none;
		border-radius: var(--r-pill);
		background: none;
		color: var(--muted);
		font: inherit;
		cursor: pointer;
	}

	.segmented button.on {
		color: var(--accent);
		background: var(--accent-soft);
	}

	.policy {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
		margin: 0;
	}

	summary {
		cursor: pointer;
		color: var(--muted);
	}

	.more {
		display: flex;
		flex-direction: column;
		gap: 18px;
		padding-top: 14px;
	}

	@media (max-width: 700px) {
		.spaces {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
