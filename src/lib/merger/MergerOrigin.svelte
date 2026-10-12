<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import type { MergeMethod } from '$lib/types';
	import { merger } from './store.svelte';

	/**
	 * What Merger was opened for (FEAT-117): a pull request, merged through its
	 * host with the host's methods, or a Farm task. Every merge is made here;
	 * the screens that used to make one open Merger on this instead.
	 *
	 * A pull request that conflicts is resolved here, on its own branch, like
	 * any two branches. The plan below this band is that merge; pushing it and
	 * merging the pull request come after.
	 */
	const origin = $derived(merger.origin);
	const pull = $derived(origin?.kind === 'pullRequest' ? origin : null);
	const task = $derived(origin?.kind === 'farmTask' ? origin : null);

	const METHODS: { id: MergeMethod; label: string; note: string }[] = [
		{ id: 'merge', label: 'Merge commit', note: 'All commits are added with a merge commit.' },
		{ id: 'squash', label: 'Squash', note: 'The commits are squashed into one.' },
		{ id: 'rebase', label: 'Rebase', note: 'The commits are replayed onto the target.' }
	];

	const conflicts = $derived(pull !== null && pull.pr.mergeable === false);

	async function merge() {
		const number = pull?.pr.number;
		const title = pull?.pr.title ?? '';
		if (await merger.mergePullRequest()) notice.ok(`#${number} merged`, title);
	}
</script>

{#if pull}
	<section class="origin" aria-label="Pull request">
		<header class="head">
			<span class="tile" aria-hidden="true"><Icon name="merge" size="1.1em" weight={1.9} /></span>
			<div class="titles">
				<h1 class="title">Pull request <span class="mono">#{pull.pr.number}</span></h1>
				<span class="note">{pull.pr.title}</span>
				<span class="note mono">{pull.pr.sourceBranch} → {pull.pr.targetBranch}</span>
			</div>
			<Btn onclick={() => void goto('/requests')}>Back to pull requests</Btn>
		</header>

		{#if merger.prMerged}
			<p class="done" role="status">
				<Icon name="check" size="1em" weight={2.4} />
				#{pull.pr.number} is merged into {pull.pr.targetBranch}.
			</p>
		{:else if pull.local}
			<p class="note">
				Resolving here, on <span class="mono">{pull.local.branch}</span>: {pull.local.target} comes in.
				The merge is written on that branch. Push it, then come back to merge the pull request.
			</p>
		{:else}
			{#if conflicts}
				<p class="conflict" role="status">
					{pull.pr.targetBranch} has changed what this pull request changes, so the host will not merge it
					as it is. Resolve the conflicts here first. This checks the pull request's branch out in this
					repository.
				</p>
				<div class="row">
					<Btn primary busy={merger.preparing} onclick={() => merger.resolvePullRequestLocally()}>
						Resolve the conflicts here
					</Btn>
				</div>
				{#if merger.prepareError}<p class="note error" role="alert">{merger.prepareError}</p>{/if}
			{/if}

			<div class="methods" role="radiogroup" aria-label="How to merge">
				{#each METHODS as method (method.id)}
					<Chip
						active={merger.prMethod === method.id}
						title={method.note}
						onclick={() => merger.setPrMethod(method.id)}
					>
						{method.label}
					</Chip>
				{/each}
			</div>
			<span class="note">{METHODS.find((m) => m.id === merger.prMethod)?.note}</span>

			{#if merger.prMethod !== 'rebase'}
				<label class="field">
					<span class="note">Commit title (optional)</span>
					<input
						type="text"
						class="mono"
						placeholder="Merge pull request #{pull.pr.number}"
						value={merger.prTitle}
						oninput={(event) => merger.setPrTitle(event.currentTarget.value)}
					/>
				</label>
				<label class="field">
					<span class="note">Commit message (optional)</span>
					<textarea
						class="mono"
						rows="3"
						value={merger.prMessage}
						oninput={(event) => merger.setPrMessage(event.currentTarget.value)}
					></textarea>
				</label>
			{/if}

			{#if merger.prError}<p class="note error" role="alert">{merger.prError}</p>{/if}
			<div class="row">
				<Btn primary={!conflicts} busy={merger.prMerging} onclick={merge}>Merge pull request</Btn>
			</div>
		{/if}
	</section>
{:else if task}
	<section class="origin" aria-label="Farm task">
		<header class="head">
			<span class="tile" aria-hidden="true"><Icon name="merge" size="1.1em" weight={1.9} /></span>
			<div class="titles">
				<h1 class="title">Farm task <span class="mono">{task.id}</span></h1>
				<span class="note">{task.title}</span>
				<span class="note mono">{task.branch} → {task.target}</span>
			</div>
			<Btn onclick={() => void goto('/farm')}>Back to the Farm</Btn>
		</header>
		<p class="note">
			Merged here like any two branches. When it is done, the Farm is told and finishes the task.
		</p>
	</section>
{/if}

<style>
	.origin {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px 20px;
		border-bottom: 1px solid var(--line);
		background: var(--panel);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.tile {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent);
		flex: none;
	}

	.titles {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.methods,
	.row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.field input,
	.field textarea {
		font: inherit;
		padding: 6px 8px;
		border: 1px solid var(--line);
		border-radius: 8px;
		background: var(--bg);
		color: var(--ink);
	}

	.conflict {
		margin: 0;
		padding: 8px 10px;
		border-radius: 10px;
		background: color-mix(in srgb, var(--warn) 14%, transparent);
	}

	.done {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--ok);
		font-weight: 600;
	}

	.error {
		color: var(--danger);
		margin: 0;
	}
</style>
