<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { AUTONOMY_LEVELS, FARM_COPY as C, whereYouComeIn } from '../describe';
	import { farmStore } from '../store.svelte';
	import * as api from '../api';
	import type { Autonomy, FarmSettings, Permissions, SupplementalPolicy } from '../types';

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
	}

	let { settings = $bindable({}), busy = false, act }: Props = $props();

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

	$effect(() => {
		settings = {
			...untrack(() => settings),
			autonomy,
			permissions: { ...permissions },
			supplemental: { ...supplemental },
			verification: checks.map((check) => check.trim()).filter(Boolean),
			maxParallel: parallel,
			maxAttempts: attempts
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
		<div class="levels" role="radiogroup" aria-label={C.howMuch}>
			{#each AUTONOMY_LEVELS as level (level.id)}
				<button
					class="level"
					class:on={autonomy === level.id}
					role="radio"
					aria-checked={autonomy === level.id}
					disabled={busy}
					onclick={() => choose(level.id)}
				>
					<strong>{level.label}</strong>
					<span>{level.detail}</span>
				</button>
			{/each}
		</div>
		<p class="come-in">
			<span class="muted">{C.youComeIn}</span>
			{whereYouComeIn(autonomy)}
		</p>
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
		gap: 8px;
	}

	.level {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 12px;
		text-align: left;
		color: var(--ink);
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
		background: var(--surface-veil);
		cursor: pointer;
	}

	.level span {
		color: var(--muted);
		font-size: var(--fs-secondary);
		line-height: 1.4;
	}

	.level:hover:not(:disabled) {
		border-color: var(--line);
	}

	.level.on {
		border-color: var(--accent);
		background: var(--accent-soft);
	}

	.come-in {
		margin: 10px 0 0;
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

	@media (max-width: 900px) {
		.levels {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
