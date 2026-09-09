<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { worktrees } from './store.svelte';
	import { worktreeModal } from './modal.svelte';
	import { workspace } from '$lib/workspace.svelte';
	import { repo } from '$lib/repo.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import type { Worktree } from '$lib/types';

	let removingPath = $state<string | null>(null);
	let forceRemove = $state(false);
	let removeError = $state<string | null>(null);
	let isActionRunning = $state(false);

	const list = $derived(worktrees.list);

	async function openWorktree(wt: Worktree): Promise<void> {
		try {
			await repo.open(wt.path);
			worktreeModal.hideManager();
			notice.ok('Switched worktree', wt.name);
		} catch (err) {
			notice.failed('Could not switch worktree', err);
		}
	}

	async function toggleLock(wt: Worktree): Promise<void> {
		isActionRunning = true;
		try {
			if (wt.lockedReason) {
				await worktrees.unlock(wt.path);
				notice.ok('Worktree unlocked', wt.name);
			} else {
				await worktrees.lock(wt.path, 'locked via Spagitty');
				notice.ok('Worktree locked', wt.name);
			}
		} catch (err) {
			notice.failed('Lock operation failed', err);
		} finally {
			isActionRunning = false;
		}
	}

	async function confirmRemove(): Promise<void> {
		if (!removingPath) return;
		isActionRunning = true;
		removeError = null;

		try {
			await worktrees.remove(removingPath, forceRemove);
			notice.ok('Worktree removed', removingPath);
			removingPath = null;
			forceRemove = false;
		} catch (err) {
			removeError = err instanceof Error ? err.message : String(err);
		} finally {
			isActionRunning = false;
		}
	}

	async function pruneStale(): Promise<void> {
		isActionRunning = true;
		try {
			await worktrees.prune();
			notice.ok('Pruned stale worktree metadata');
		} catch (err) {
			notice.failed('Pruning failed', err);
		} finally {
			isActionRunning = false;
		}
	}

	function keydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault();
			if (removingPath) {
				removingPath = null;
			} else {
				worktreeModal.hideManager();
			}
		}
	}
</script>

{#if worktreeModal.isManagerOpen}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="scrim"
		role="dialog"
		aria-modal="true"
		aria-label="Manage git worktrees"
		tabindex="-1"
		onkeydown={keydown}
	>
		<div class="modal">
			<header class="head">
				<div class="head-left">
					<span class="title">Worktrees</span>
					<span class="badge">{list.length}</span>
				</div>
				<div class="head-actions">
					<Btn
						disabled={isActionRunning}
						onclick={() => {
							worktreeModal.hideManager();
							worktreeModal.showAdd();
						}}
					>+ Add Worktree</Btn>
					<Btn disabled={isActionRunning} onclick={pruneStale}>Prune Stale</Btn>
					<button
						class="close"
						aria-label="Close"
						onclick={() => worktreeModal.hideManager()}
					>✕</button>
				</div>
			</header>

			<div class="body">
				{#if list.length === 0}
					<div class="empty">No worktrees found for this repository.</div>
				{:else}
					<div class="worktree-list">
						{#each list as wt (wt.path)}
							<div class="worktree-card" class:main={wt.isMain}>
								<div class="card-info">
									<div class="card-top">
										<span class="name">{wt.name}</span>
										{#if wt.isMain}
											<span class="pill main-pill">main</span>
										{/if}
										{#if wt.branch}
											<span class="pill branch-pill">{wt.branch}</span>
										{:else}
											<span class="pill detached-pill">detached</span>
										{/if}
										{#if wt.lockedReason}
											<span class="pill locked-pill" title={wt.lockedReason}>🔒 locked</span>
										{/if}
										{#if wt.prunableReason}
											<span class="pill prunable-pill" title={wt.prunableReason}>⚠️ prunable</span>
										{/if}
									</div>
									<div class="card-bottom">
										<span class="path mono" title={wt.path}>{wt.path}</span>
										<span class="sha mono">{wt.headShort}</span>
									</div>
								</div>

								<div class="card-actions">
									{#if repo.info?.path === wt.path}
										<span class="active-label">Active</span>
									{:else}
										<Btn
											disabled={isActionRunning}
											onclick={() => void openWorktree(wt)}
										>Open</Btn>
									{/if}
									{#if !wt.isMain}
										<button
											type="button"
											class="icon-btn"
											title={wt.lockedReason ? 'Unlock worktree' : 'Lock worktree'}
											disabled={isActionRunning}
											onclick={() => void toggleLock(wt)}
										>
											{wt.lockedReason ? '🔓' : '🔒'}
										</button>
										<button
											type="button"
											class="icon-btn danger"
											title="Remove worktree"
											disabled={isActionRunning}
											onclick={() => (removingPath = wt.path)}
										>
											🗑️
										</button>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			{#if removingPath}
				<div class="remove-dialog">
					<div class="remove-title">Remove Worktree</div>
					<div class="remove-body">
						Are you sure you want to remove <span class="mono">{removingPath}</span>?
					</div>
					<label class="force-checkbox">
						<input type="checkbox" bind:checked={forceRemove} />
						<span>Force remove (even if working tree is dirty or locked)</span>
					</label>
					{#if removeError}
						<div class="error" role="alert">{removeError}</div>
					{/if}
					<div class="remove-actions">
						<Btn disabled={isActionRunning} onclick={() => (removingPath = null)}>Cancel</Btn>
						<Btn danger disabled={isActionRunning} onclick={confirmRemove}>
							{isActionRunning ? 'Removing…' : 'Remove'}
						</Btn>
					</div>
				</div>
			{/if}
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
		width: 640px;
		max-width: 90vw;
		max-height: 80vh;
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

	.head-left {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.title {
		font-weight: 600;
		font-size: var(--fs-ui);
		color: var(--ink);
	}

	.badge {
		background: var(--soft);
		color: var(--muted);
		font-size: var(--fs-mono);
		padding: 2px 6px;
		border-radius: 10px;
	}

	.head-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.close {
		background: transparent;
		border: none;
		color: var(--muted);
		cursor: pointer;
		font-size: var(--fs-ui);
		padding: 4px;
		margin-left: 4px;
	}

	.close:hover {
		color: var(--ink);
	}

	.body {
		padding: 16px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.empty {
		text-align: center;
		padding: 32px;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.worktree-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.worktree-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 12px;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 6px;
		gap: 12px;
	}

	.worktree-card.main {
		border-left: 3px solid var(--accent);
	}

	.card-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
		overflow: hidden;
		flex: 1;
	}

	.card-top {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
	}

	.name {
		font-weight: 500;
		font-size: var(--fs-secondary);
		color: var(--ink);
	}

	.pill {
		font-size: var(--fs-mono);
		padding: 1px 6px;
		border-radius: 4px;
	}

	.main-pill {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.branch-pill {
		background: var(--soft);
		color: var(--ink);
	}

	.detached-pill {
		background: var(--soft);
		color: var(--muted);
	}

	.locked-pill {
		background: var(--warn-soft);
		color: var(--warn);
	}

	.prunable-pill {
		background: var(--danger-soft);
		color: var(--danger);
	}

	.card-bottom {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.path {
		font-size: var(--fs-mono);
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 380px;
	}

	.sha {
		font-size: var(--fs-mono);
		color: var(--accent);
	}

	.mono {
		font-family: var(--font-mono);
	}

	.card-actions {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.active-label {
		font-size: var(--fs-mono);
		color: var(--accent);
		font-weight: 500;
		padding: 4px 8px;
	}

	.icon-btn {
		background: var(--soft);
		border: 1px solid var(--line);
		border-radius: 4px;
		padding: 4px 6px;
		cursor: pointer;
		font-size: var(--fs-secondary);
	}

	.icon-btn:hover {
		background: var(--hover);
	}

	.icon-btn.danger:hover {
		background: color-mix(in srgb, var(--danger) 22%, transparent);
		border-color: var(--danger);
	}

	.remove-dialog {
		background: var(--surface);
		border-top: 1px solid var(--line);
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.remove-title {
		font-weight: 600;
		font-size: var(--fs-secondary);
		color: var(--danger);
	}

	.remove-body {
		font-size: var(--fs-secondary);
		color: var(--ink);
	}

	.force-checkbox {
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

	.remove-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
