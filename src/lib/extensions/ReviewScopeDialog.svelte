<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Modal from './Modal.svelte';
	import type { ReviewDraft } from './store.svelte';
	import { extensions } from './store.svelte';
	import type { ReviewScope } from './types';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * What a review will cover, before anything leaves the machine: the base,
	 * the scope, every file, and what was left out and why. Choosing a
	 * different scope or base reads the preview again; starting is a separate,
	 * explicit step.
	 */
	interface Props {
		draft: ReviewDraft;
	}

	let { draft }: Props = $props();

	const SCOPES: Record<ReviewScope, string> = {
		uncommitted: 'Uncommitted changes',
		includeUntracked: 'Uncommitted changes and untracked files',
		committed: 'Committed changes',
		tracked: 'Committed and uncommitted changes'
	};
	const needsBase = $derived(draft.request.scope !== 'uncommitted');
	const destination = $derived(
		draft.extension.manifest.contributes?.reviewProviders?.find((p) => p.id === draft.provider)?.sendsCodeTo ?? null
	);
</script>

<Modal title={`Review with ${draft.extension.name}`} wide onclose={() => extensions.closeDraft()}>
	<div class="controls">
		<label>
			<span class="muted">What</span>
			<select
				value={draft.request.scope}
				disabled={draft.busy || draft.scopes.length < 2}
				onchange={(e) => extensions.updateDraft({ scope: e.currentTarget.value as ReviewScope })}
			>
				{#each draft.scopes as scope (scope)}<option value={scope}>{SCOPES[scope]}</option>{/each}
			</select>
		</label>
		{#if needsBase}
			<label>
				<span class="muted">Against</span>
				<select
					value={draft.request.base ?? ''}
					disabled={draft.busy}
					onchange={(e) => extensions.updateDraft({ base: e.currentTarget.value })}
				>
					{#if draft.request.base && !draft.bases.includes(draft.request.base)}
						<option value={draft.request.base}>{draft.request.base}</option>
					{/if}
					{#each draft.bases as base (base)}<option value={base}>{base}</option>{/each}
				</select>
			</label>
		{/if}
	</div>

	{#if draft.error}
		<p class="error">{draft.error}</p>
	{:else if draft.preview}
		<p class="muted mono" title={`${draft.preview.baseCommit} → ${draft.preview.headCommit}`}>
			{draft.preview.baseRef ?? draft.preview.baseCommit.slice(0, 8)} → {draft.preview.headCommit.slice(0, 8)}
		</p>
		{#if draft.preview.files.length === 0}
			<p>Nothing to review in this scope.</p>
		{:else}
			<ul class="files" aria-label="Files that will be reviewed">
				{#each draft.preview.files as file (file.origin + file.path)}
					<li><span class="mono">{file.path}</span> <span class="muted">{file.status}, {file.origin}</span></li>
				{/each}
			</ul>
			{#if draft.preview.truncated}<p class="muted">The list is shortened; every changed file is reviewed.</p>{/if}
		{/if}
		{#if draft.preview.excluded.length}
			<details>
				<summary>{draft.preview.excluded.length} left out</summary>
				<ul class="files">
					{#each draft.preview.excluded as file (file.path)}
						<li><span class="mono">{file.path}</span> <span class="muted">{file.reason}</span></li>
					{/each}
				</ul>
			</details>
		{/if}
		{#if destination}
			<p class="send" class:first={draft.firstUpload}>
				These files and their context go to {destination} when you start.
			</p>
		{/if}
	{:else}
		<p class="muted">Reading the changes…</p>
	{/if}

	{#snippet actions()}
		<Btn onclick={() => extensions.closeDraft()}>Cancel</Btn>
		<Btn
			primary
			disabled={draft.busy || !draft.preview || draft.preview.files.length === 0}
			onclick={() => extensions.confirmDraft()}
		>
			Start review
		</Btn>
	{/snippet}
</Modal>

<style>
	.controls {
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	select {
		font-size: var(--fs-secondary);
	}

	.files {
		margin: 0;
		padding-left: 16px;
		max-height: 220px;
		overflow: auto;
	}

	.muted {
		color: var(--muted);
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.error {
		color: var(--danger);
	}

	.send {
		padding: 6px 8px;
		border-radius: var(--r-field);
		background: var(--sunken);
	}

	.send.first {
		background: var(--warn-soft);
	}

	p {
		margin: 0;
	}
</style>
