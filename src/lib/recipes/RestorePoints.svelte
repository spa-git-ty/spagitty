<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { inTauri } from '$lib/api';
	import { relativeTime } from '$lib/format';
	import { repo } from '$lib/repo.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import * as api from './api';
	import type { RestorePoint } from './types';

	/**
	 * The restore points recipes made, on the Reflog screen (FEAT-133).
	 *
	 * Reflog is where "what did I just do, and how do I get back" is asked,
	 * and a restore point is an answer to it that the reflog cannot give: the
	 * working tree and its untracked files, the index and the stash list, as
	 * well as where the branches were. The newest twenty are kept, none
	 * longer than seven days.
	 *
	 * Nothing is drawn until a recipe has made one.
	 */
	let points = $state<RestorePoint[]>([]);
	let busy = $state(false);

	async function load() {
		if (!inTauri() || !repo.info) {
			points = [];
			return;
		}
		try {
			points = await api.restorePoints();
		} catch {
			// Not being able to list them is not worth a message on a screen
			// that is about something else.
			points = [];
		}
	}

	onMount(() => void load());

	$effect(() => {
		void repo.generation;
		void repo.info?.head.id;
		untrack(() => void load());
	});

	async function back(point: RestorePoint) {
		const sure = await dialog.confirm({
			title: `Go back to before ${point.recipe}?`,
			body: `HEAD, ${point.tips.length === 1 ? 'the branch' : 'the branches'} it named, the working tree, the index and the stash list go back to what they were ${relativeTime(point.created)}. Changes made since are lost. What was pushed stays pushed.`,
			confirmLabel: 'Go back',
			danger: true
		});
		if (!sure) return;
		busy = true;
		try {
			await api.restoreTo(point.id);
			notice.ok(`Back to before ${point.recipe}`);
			await repo.refresh();
		} catch (cause) {
			notice.failed('Could not go back', cause);
		} finally {
			busy = false;
		}
	}

	async function drop(point: RestorePoint) {
		busy = true;
		try {
			points = await api.dropRestorePoint(point.id);
		} catch (cause) {
			notice.failed('Could not drop the restore point', cause);
		} finally {
			busy = false;
		}
	}
</script>

{#if points.length > 0}
	<section class="points" aria-label="Restore points">
		<h3 class="note">
			Restore points
			<span class="muted">· made before each recipe ran · the newest 20, for 7 days</span>
		</h3>
		{#each points as point (point.id)}
			<div class="row">
				<span class="mono name">{point.recipe}</span>
				<span class="note where">
					on <span class="mono">{point.branch ?? 'a detached HEAD'}</span>
					at <span class="mono">{point.head.slice(0, 7)}</span>
				</span>
				<span class="note when">{relativeTime(point.created)}</span>
				<span class="acts">
					<Chip
						disabled={busy}
						title="Put HEAD, the branches, the working tree, the index and the stashes back as they were"
						onclick={() => back(point)}>undo to here</Chip
					>
					<Chip danger disabled={busy} title="Forget this restore point" onclick={() => drop(point)}>drop</Chip>
				</span>
			</div>
		{/each}
	</section>
{/if}

<style>
	.points {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin: 0 16px 10px;
		padding: 10px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
		background: var(--surface);
	}

	h3 {
		margin: 0 0 4px;
		font-size: var(--fs-secondary);
		font-weight: 400;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 3px 0;
	}

	.name {
		flex: 0 0 200px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.where {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.when {
		flex: none;
	}

	.acts {
		flex: none;
		display: flex;
		gap: 6px;
	}
</style>
