<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import DiffPane from '$lib/diff/DiffPane.svelte';
	import FileList from '$lib/diff/FileList.svelte';
	import { diff } from '$lib/diff/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';

	/**
	 * One commit's changes, hunk by hunk.
	 *
	 * A full-window takeover rather than a rail screen: it is opened from a
	 * commit and answers one question, so it keeps its own back button and Esc
	 * returns to the graph the row came from.
	 */

	const commit = $derived(page.url.searchParams.get('commit'));

	/** Which hunk `j` / `k` last moved to. */
	let focus = $state(0);

	onMount(() => diff.init());

	/** What the last load was for, so the same commit is not fetched twice. */
	let loaded: { id: string; generation: number } | null = null;

	// The URL and the open repository are what drive a load. The repository is
	// in it because this screen can be the first one painted — the window is
	// opened straight onto a commit — and the walk that opens the repository
	// finishes after this component mounts. Without waiting for it, the first
	// fetch lands before there is a repository and the screen reads "no
	// repository is open" for a repository that is about to be open.
	//
	// The call itself is untracked: opening a commit writes to the store it
	// reads, and a tracked read of that would fetch the same commit twice.
	$effect(() => {
		const id = commit;
		const generation = repo.generation;
		if (!id || repo.info === null) return;
		if (loaded?.id === id && loaded.generation === generation) return;

		loaded = { id, generation };
		untrack(() => {
			// A different repository means the cached files are about someone
			// else's commit, even when the id happens to match.
			diff.clear();
			diff.open(id);
		});
	});

	// A new file starts at its first hunk rather than wherever the last one was.
	let lastPath: string | null = null;
	$effect(() => {
		if (diff.path !== lastPath) {
			lastPath = diff.path;
			focus = 0;
		}
	});

	const hunkCount = $derived(diff.file?.hunks.length ?? 0);

	const counts = $derived.by(() => {
		const loaded = diff.commit;
		if (loaded === null) return '';
		const files = loaded.files.length === 1 ? '1 file' : `${loaded.files.length} files`;
		return `${files} · +${loaded.added} −${loaded.removed}`;
	});

	const position = $derived(diff.count === 0 ? '' : `${diff.index + 1} of ${diff.count}`);

	function back() {
		goto('/');
	}

	function onkeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement | null;
		if (target && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))) {
			return;
		}
		if (event.metaKey || event.ctrlKey || event.altKey) return;

		if (event.key === 'Escape') {
			event.preventDefault();
			back();
		} else if (event.key === 'j') {
			event.preventDefault();
			if (hunkCount > 0) focus = Math.min(focus + 1, hunkCount - 1);
		} else if (event.key === 'k') {
			event.preventDefault();
			focus = Math.max(focus - 1, 0);
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="screen">
	<header class="head">
		<div class="left">
			<Btn onclick={back}>← Graph</Btn>
			<span class="title">
				Diff{#if diff.commit}<span class="mono sha"> · {diff.commit.short}</span>{/if}
			</span>
			{#if diff.commit}<span class="note summary">{diff.commit.summary}</span>{/if}
		</div>
		<div class="right">
			{#if counts}<span class="note">{counts}</span>{/if}
			<Chip active={diff.view === 'unified'} onclick={() => diff.setView('unified')}>
				unified
			</Chip>
			<Chip active={diff.view === 'split'} onclick={() => diff.setView('split')}>split</Chip>
		</div>
	</header>

	{#if repo.info === null}
		<div class="empty"><p class="note">No repository open.</p></div>
	{:else if commit === null}
		<div class="empty">
			<p class="note">No commit chosen.</p>
			<Btn onclick={back}>← Graph</Btn>
		</div>
	{:else if diff.error}
		<div class="empty"><p class="note error">{diff.error}</p></div>
	{:else if diff.commit === null}
		<div class="empty"><Loader label="Reading the commit…" /></div>
	{:else}
		<div class="body">
			<FileList
				files={diff.commit.files}
				selected={diff.path}
				onselect={(path) => diff.select(path)}
				onstep={(delta) => diff.step(delta)}
			/>
			<Splitter panel="diffFiles" label="Resize the file list" />
			<DiffPane
				file={diff.file}
				path={diff.path}
				error={diff.fileError}
				loading={diff.fileLoading}
				view={diff.view}
				{focus}
			/>
		</div>
	{/if}

	<footer class="foot">
		<div class="left">
			<Btn disabled={diff.index <= 0} onclick={() => diff.step(-1)}>Prev file</Btn>
			<Btn disabled={diff.index < 0 || diff.index >= diff.count - 1} onclick={() => diff.step(1)}>
				Next file
			</Btn>
			{#if position}<span class="note">{position}</span>{/if}
		</div>
	</footer>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head,
	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		flex: none;
	}

	.head {
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.foot {
		padding: 8px;
		background-color: var(--chrome-veil);
		border-top: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.title {
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.sha {
		color: var(--muted);
	}

	.summary {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		overflow: hidden;
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
	}

	.error {
		color: var(--danger);
	}
</style>
