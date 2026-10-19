<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { fileAttrs } from '$lib/files/open';
	import { repo } from '$lib/repo.svelte';
	import type { FileStatus, StatusEntry } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';

	/** Working copy (FEAT-140): what is changed, the first three paths, Commit and Stash. */
	interface Props {
		changed: StatusEntry[];
		stashes: number | null;
		busy: boolean;
		oncommit: () => void;
		onstash: () => void;
	}

	let { changed, stashes, busy, oncommit, onstash }: Props = $props();

	const LETTERS: Record<FileStatus, { letter: string; colour: string }> = {
		added: { letter: 'A', colour: 'var(--ok)' },
		modified: { letter: 'M', colour: 'var(--warn)' },
		deleted: { letter: 'D', colour: 'var(--danger)' },
		renamed: { letter: 'R', colour: 'var(--lane-5)' },
		untracked: { letter: 'U', colour: 'var(--ok)' }
	};

	/** Files reads commits: a changed path opens as the branch has it (FEAT-150). */
	const head = $derived(repo.info?.head.branch ?? repo.info?.head.id ?? null);

	function inFiles(entry: StatusEntry) {
		const fresh = entry.status === 'added' || entry.status === 'untracked';
		return fileAttrs(entry.path, head, fresh ? 'Not in a commit yet' : undefined, { click: !fresh });
	}
</script>

<section class="card home-card roomy" aria-labelledby="home-working">
	<div class="home-card-head">
		<h2 id="home-working">Working copy</h2>
		<span class="note">
			{changed.length} changed{stashes ? ` · ${stashes} ${stashes === 1 ? 'stash' : 'stashes'}` : ''}
		</span>
	</div>
	{#if changed.length}
		<div class="paths mono">
			{#each changed.slice(0, 3) as entry (entry.path)}
				<div class="path" {...inFiles(entry)}>
					<span class="letter" style:color={LETTERS[entry.status].colour}>{LETTERS[entry.status].letter}</span>
					<span class="home-ellipsis">{entry.path}</span>
				</div>
			{/each}
			{#if changed.length > 3}<div class="home-quiet">and {changed.length - 3} more</div>{/if}
		</div>
	{:else}
		<p class="home-quiet">Nothing changed. The working copy matches HEAD.</p>
	{/if}
	<div class="actions">
		<Btn onclick={oncommit}>Commit…</Btn>
		<Btn disabled={busy || changed.length === 0} onclick={onstash}>Stash</Btn>
	</div>
</section>

<style>
	.paths {
		display: flex;
		flex-direction: column;
		gap: 7px;
		font-size: var(--fs-mono);
	}

	.path {
		display: flex;
		gap: 10px;
		min-width: 0;
	}

	.letter {
		flex: none;
		width: 12px;
	}

	.actions {
		display: flex;
		gap: 8px;
	}

	p {
		margin: 0;
	}
</style>
