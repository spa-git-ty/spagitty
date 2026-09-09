<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { worktrees } from './store.svelte';
	import { worktreeModal } from './modal.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import { branches } from '$lib/branches/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';

	let targetPath = $state('');
	let mode = $state<'new' | 'existing' | 'detached'>('new');
	let newBranchName = $state('');
	let selectedBranch = $state('');
	let isSubmitting = $state(false);
	let error = $state<string | null>(null);

	const branchList = $derived(branches.rows.map((r) => r.name));

	async function chooseFolder(): Promise<void> {
		try {
			const selected = await openDialog({
				directory: true,
				multiple: false,
				title: 'Select Worktree Directory'
			});
			if (selected && typeof selected === 'string') {
				targetPath = selected;
			}
		} catch (err) {
			notice.failed('Could not open folder picker', err);
		}
	}

	async function submit(): Promise<void> {
		if (!targetPath.trim()) {
			error = 'Target path cannot be empty';
			return;
		}

		error = null;
		isSubmitting = true;

		try {
			if (mode === 'new') {
				if (!newBranchName.trim()) {
					error = 'Branch name cannot be empty';
					isSubmitting = false;
					return;
				}
				await worktrees.add(targetPath.trim(), null, newBranchName.trim(), false);
			} else if (mode === 'existing') {
				if (!selectedBranch.trim()) {
					error = 'Please select a branch';
					isSubmitting = false;
					return;
				}
				await worktrees.add(targetPath.trim(), selectedBranch.trim(), null, false);
			} else {
				await worktrees.add(targetPath.trim(), null, null, true);
			}

			notice.ok('Worktree added', targetPath);
			worktreeModal.hideAdd();
			targetPath = '';
			newBranchName = '';
			selectedBranch = '';
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			isSubmitting = false;
		}
	}

	function keydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault();
			worktreeModal.hideAdd();
		}
	}
</script>

{#if worktreeModal.isAddOpen}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="scrim"
		role="dialog"
		aria-modal="true"
		aria-label="Add git worktree"
		tabindex="-1"
		onkeydown={keydown}
	>
		<div class="modal">
			<header class="head">
				<span class="title">Add Worktree</span>
				<button
					class="close"
					aria-label="Close"
					onclick={() => worktreeModal.hideAdd()}
				>✕</button>
			</header>

			<div class="body">
				<div class="field-row">
					<label class="label" for="wt-path">Location</label>
					<div class="path-row">
						<input
							id="wt-path"
							class="field"
							type="text"
							placeholder="/path/to/worktree"
							bind:value={targetPath}
							disabled={isSubmitting}
						/>
						<Btn disabled={isSubmitting} onclick={chooseFolder}>Browse…</Btn>
					</div>
				</div>

				<div class="field-row">
					<span class="label">Checkout Mode</span>
					<div class="mode-tabs" role="radiogroup" aria-label="Checkout mode">
						<button
							type="button"
							class="mode-btn"
							class:selected={mode === 'new'}
							onclick={() => (mode = 'new')}
						>New Branch</button>
						<button
							type="button"
							class="mode-btn"
							class:selected={mode === 'existing'}
							onclick={() => (mode = 'existing')}
						>Existing Branch</button>
						<button
							type="button"
							class="mode-btn"
							class:selected={mode === 'detached'}
							onclick={() => (mode = 'detached')}
						>Detached HEAD</button>
					</div>
				</div>

				{#if mode === 'new'}
					<div class="field-row">
						<label class="label" for="new-branch">New Branch Name</label>
						<input
							id="new-branch"
							class="field"
							type="text"
							placeholder="feature/new-worktree"
							bind:value={newBranchName}
							disabled={isSubmitting}
						/>
					</div>
				{:else if mode === 'existing'}
					<div class="field-row">
						<label class="label" for="existing-branch">Branch</label>
						{#if branchList.length > 0}
							<select
								id="existing-branch"
								class="field select"
								bind:value={selectedBranch}
								disabled={isSubmitting}
							>
								<option value="">Select a branch…</option>
								{#each branchList as b}
									<option value={b}>{b}</option>
								{/each}
							</select>
						{:else}
							<input
								id="existing-branch"
								class="field"
								type="text"
								placeholder="main"
								bind:value={selectedBranch}
								disabled={isSubmitting}
							/>
						{/if}
					</div>
				{/if}

				{#if error}
					<div class="error" role="alert">{error}</div>
				{/if}
			</div>

			<footer class="actions">
				<Btn disabled={isSubmitting} onclick={() => worktreeModal.hideAdd()}>Cancel</Btn>
				<Btn primary disabled={isSubmitting} onclick={submit}>
					{isSubmitting ? 'Adding…' : 'Add Worktree'}
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
		width: 480px;
		max-width: 90vw;
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
	}

	.field-row {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.path-row {
		display: flex;
		gap: 8px;
	}

	.label {
		font-size: var(--fs-secondary);
		font-weight: 500;
		color: var(--muted);
	}

	.field {
		flex: 1;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 4px;
		padding: 8px 10px;
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

	.mode-tabs {
		display: flex;
		border: 1px solid var(--line);
		border-radius: 4px;
		overflow: hidden;
	}

	.mode-btn {
		flex: 1;
		background: var(--surface);
		border: none;
		padding: 6px 8px;
		font-size: var(--fs-secondary);
		color: var(--muted);
		cursor: pointer;
	}

	.mode-btn:not(:last-child) {
		border-right: 1px solid var(--line);
	}

	.mode-btn.selected {
		background: var(--accent-soft);
		color: var(--accent);
		font-weight: 500;
	}

	.error {
		font-size: var(--fs-secondary);
		color: var(--danger);
		background: var(--danger-soft);
		padding: 8px;
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
