<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { requests } from './store.svelte';
	import { branches } from '$lib/branches/store.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import Btn from '$lib/ui/Btn.svelte';

	let title = $state('');
	let body = $state('');
	let baseBranch = $state('main');
	let headBranch = $state('');
	let draft = $state(false);
	let localError = $state<string | null>(null);

	const branchList = $derived(branches.rows.map((r) => r.name));
	const currentBranch = $derived(branches.rows.find((r) => r.current)?.name ?? '');

	$effect(() => {
		if (requests.isCreateModalOpen) {
			if (currentBranch) headBranch = currentBranch;
			if (branchList.includes('main')) baseBranch = 'main';
			else if (branchList.includes('dev')) baseBranch = 'dev';
			else if (branchList.length > 0) baseBranch = branchList[0];
		}
	});

	async function submit(): Promise<void> {
		if (!title.trim()) {
			localError = 'Title is required';
			return;
		}
		if (!headBranch.trim() || !baseBranch.trim()) {
			localError = 'Source and target branches are required';
			return;
		}
		if (headBranch.trim() === baseBranch.trim()) {
			localError = 'Head branch and base branch cannot be identical';
			return;
		}

		localError = null;
		try {
			const pr = await requests.create(
				title.trim(),
				body.trim(),
				headBranch.trim(),
				baseBranch.trim(),
				draft
			);
			notice.ok('Pull request created', `#${pr.number} — ${pr.title}`);
			title = '';
			body = '';
		} catch (err) {
			localError = err instanceof Error ? err.message : String(err);
		}
	}

	function keydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault();
			requests.closeCreateModal();
		}
	}
</script>

{#if requests.isCreateModalOpen}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="scrim"
		role="dialog"
		aria-modal="true"
		aria-label="Create pull request"
		tabindex="-1"
		onkeydown={keydown}
	>
		<div class="modal">
			<header class="head">
				<span class="title">Create Pull Request</span>
				<button
					class="close"
					aria-label="Close"
					onclick={() => requests.closeCreateModal()}
				>✕</button>
			</header>

			<div class="body">
				<div class="branches-row">
					<div class="field-col">
						<label class="label" for="base-branch">Base Branch (Merge into)</label>
						{#if branchList.length > 0}
							<select
								id="base-branch"
								class="select field"
								bind:value={baseBranch}
								disabled={requests.creating}
							>
								{#each branchList as b}
									<option value={b}>{b}</option>
								{/each}
							</select>
						{:else}
							<input
								id="base-branch"
								type="text"
								class="field"
								placeholder="main"
								bind:value={baseBranch}
								disabled={requests.creating}
							/>
						{/if}
					</div>

					<span class="arrow">←</span>

					<div class="field-col">
						<label class="label" for="head-branch">Head Branch (Compare)</label>
						{#if branchList.length > 0}
							<select
								id="head-branch"
								class="select field"
								bind:value={headBranch}
								disabled={requests.creating}
							>
								{#each branchList as b}
									<option value={b}>{b}</option>
								{/each}
							</select>
						{:else}
							<input
								id="head-branch"
								type="text"
								class="field"
								placeholder="feature/branch"
								bind:value={headBranch}
								disabled={requests.creating}
							/>
						{/if}
					</div>
				</div>

				<div class="field-col">
					<label class="label" for="pr-title">Title</label>
					<input
						id="pr-title"
						type="text"
						class="field"
						placeholder="feat(topic): short description of change"
						bind:value={title}
						disabled={requests.creating}
					/>
				</div>

				<div class="field-col">
					<label class="label" for="pr-body">Description</label>
					<textarea
						id="pr-body"
						class="textarea field"
						rows="6"
						placeholder="Explain the motivation, approach, and testing for this change…"
						bind:value={body}
						disabled={requests.creating}
					></textarea>
				</div>

				<label class="checkbox-row">
					<input type="checkbox" bind:checked={draft} disabled={requests.creating} />
					<span>Create as Draft (mark as work in progress)</span>
				</label>

				{#if localError || requests.createError}
					<div class="error" role="alert">{localError ?? requests.createError}</div>
				{/if}
			</div>

			<footer class="actions">
				<Btn disabled={requests.creating} onclick={() => requests.closeCreateModal()}>Cancel</Btn>
				<Btn primary disabled={requests.creating} onclick={submit}>
					{requests.creating ? 'Creating…' : 'Create Pull Request'}
				</Btn>
			</footer>
		</div>
	</div>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		background: color-mix(in srgb, var(--umbra) 40%, transparent);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal {
		background: var(--glass-thick);
		backdrop-filter: var(--blur-thick);
		-webkit-backdrop-filter: var(--blur-thick);
		border: var(--glass-edge-line);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		box-shadow: var(--shadow-3);
		animation: rise-in var(--t-enter) var(--spring);
		width: 580px;
		max-width: 90vw;
		max-height: 85vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 16px;
		border-bottom: 1px solid var(--line);
	}

	.title {
		font-weight: 600;
		font-size: var(--fs-ui);
		color: var(--ink);
	}

	.close {
		background: transparent;
		border: none;
		color: var(--muted);
		cursor: pointer;
		font-size: var(--fs-ui);
		padding: 4px;
	}

	.close:hover {
		color: var(--ink);
	}

	.body {
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		overflow-y: auto;
	}

	.branches-row {
		display: flex;
		align-items: flex-end;
		gap: 10px;
	}

	.arrow {
		color: var(--accent);
		font-size: var(--fs-title);
		font-weight: bold;
		padding-bottom: 6px;
	}

	.field-col {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
	}

	.label {
		font-size: var(--fs-mono);
		color: var(--muted);
		font-weight: 500;
	}

	.field {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 4px;
		padding: 6px 10px;
		font-size: var(--fs-secondary);
		color: var(--ink);
		outline: none;
	}

	.field:focus {
		border-color: var(--accent);
	}

	.select {
		cursor: pointer;
	}

	.textarea {
		resize: vertical;
		font-family: inherit;
		line-height: 1.4;
	}

	.checkbox-row {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: var(--fs-secondary);
		color: var(--muted);
		cursor: pointer;
	}

	.error {
		font-size: var(--fs-secondary);
		color: var(--danger);
		background: var(--danger-soft);
		padding: 6px 8px;
		border-radius: 4px;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		padding: 12px 16px;
		border-top: 1px solid var(--line);
		background: var(--surface);
	}
</style>
