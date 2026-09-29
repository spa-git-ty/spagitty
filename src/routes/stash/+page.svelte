<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import RefTabs from '$lib/branches/RefTabs.svelte';
	import { onMount, untrack } from 'svelte';
	import DiffPane from '$lib/diff/DiffPane.svelte';
	import FileList from '$lib/diff/FileList.svelte';
	import { diff } from '$lib/diff/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import StashDetail from '$lib/stash/StashDetail.svelte';
	import StashList from '$lib/stash/StashList.svelte';
	import { stash } from '$lib/stash/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';

	/**
	 * Stash entries, each hanging off the commit it was made on, and what is in
	 * the selected one — file by file (FEAT-034).
	 *
	 * The files and the pane are the Diff screen's own components. A stash *is*
	 * a commit whose first parent is the commit the work was made on, so
	 * `commitDiff` and `fileDiff` on the entry's id answer both questions
	 * already; there was nothing here to read that FEAT-002 had not read.
	 *
	 * The unified/split choice is the Diff screen's too, and deliberately the
	 * same setting rather than a second one: it is a preference about reading
	 * diffs, not about a screen.
	 *
	 * Stashing is the only write here. Pop, apply and drop are FEAT-014 — the
	 * first two write to the working copy and can conflict, and the third
	 * destroys an entry whose only other reference is a reflog that expires.
	 */

	/** Which hunk `j` / `k` last moved to, as on the Diff screen. */
	let focus = $state(0);

	onMount(() => diff.init());

	let generation: number | null = null;
	$effect(() => {
		const current = repo.generation;
		if (repo.info === null) return;
		if (generation === current) return;

		generation = current;
		untrack(() => {
			stash.clear();
			stash.load();
		});
	});

	// A new file starts at its first hunk rather than wherever the last one was.
	let lastPath: string | null = null;
	$effect(() => {
		if (stash.path !== lastPath) {
			lastPath = stash.path;
			focus = 0;
		}
	});

	const count = $derived(
		stash.entries.length === 1 ? '1 entry' : `${stash.entries.length} entries`
	);

	const hunkCount = $derived(stash.file?.hunks.length ?? 0);

	/**
	 * `j` and `k` move through the hunks of the open file.
	 *
	 * On the window rather than the pane, matching the Diff screen — and it
	 * steps aside for anything being typed into, because this screen has a
	 * message field along the bottom.
	 */
	function onkeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement | null;
		if (target && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))) {
			return;
		}
		if (event.metaKey || event.ctrlKey || event.altKey) return;

		if (event.key === 'j') {
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
			<RefTabs />
			{#if stash.loaded}<span class="note">{count}</span>{/if}
		</div>
		<div class="right">
			{#if stash.loading}<span class="note">Reading…</span>{/if}
			<Chip active={diff.view === 'unified'} onclick={() => diff.setView('unified')}>
				unified
			</Chip>
			<Chip active={diff.view === 'split'} onclick={() => diff.setView('split')}>split</Chip>
			<Btn disabled={stash.busy} onclick={() => stash.load()}>Refresh</Btn>
		</div>
	</header>

	{#if repo.info === null}
		<div class="empty"><p class="note">No repository open.</p></div>
	{:else if stash.error}
		<div class="empty"><p class="note error">{stash.error}</p></div>
	{:else if !stash.loaded}
		<div class="empty"><span class="note">Reading the stash…</span></div>
	{:else}
		<div class="body">
			<StashList />
			<Splitter panel="stashEntries" label="Resize the stash entries list" />
			<FileList
				files={stash.contents?.files ?? []}
				selected={stash.path}
				onselect={(path) => stash.selectFile(path)}
				onstep={(delta) => stash.stepFile(delta)}
				label="Files in this stash entry"
				empty="This entry changed nothing."
			/>
			<Splitter panel="diffFiles" label="Resize the file list" />
			<DiffPane
				file={stash.file}
				path={stash.path}
				error={stash.fileError}
				loading={stash.fileLoading}
				view={diff.view}
				{focus}
			/>
			<Splitter panel="detail" label="Resize the stash detail panel" />
			<StashDetail />
		</div>

		<div class="push">
			<span class="note">Stash the working copy</span>
			<input
				class="field"
				type="text"
				placeholder="message (optional)"
				value={stash.message}
				oninput={(event) => stash.setMessage(event.currentTarget.value)}
				aria-label="Stash message"
			/>
			<Chip
				active={stash.includeUntracked}
				onclick={() => stash.setIncludeUntracked(!stash.includeUntracked)}
				title="Take files git is not tracking yet as well"
			>
				include untracked
			</Chip>
			<Btn primary disabled={stash.busy} onclick={() => stash.push()}>Stash</Btn>
		</div>
	{/if}

	<!--
		The footer appears only when there is a failure to report. Its other
		sentence explained what stashing is and claimed restoring it was unbuilt;
		the first is not this screen's job and the second was untrue — the whole
		pop / apply / drop path exists. Left rendering unconditionally it would
		be a bordered empty strip.
	-->
	{#if stash.writeError}
		<footer class="foot">
			<span class="note error">{stash.writeError}</span>
		</footer>
	{/if}
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
	.foot,
	.push {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.head {
		justify-content: space-between;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.push,
	.foot {
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-top: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
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


	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		overflow: hidden;
	}

	.field {
		background: transparent;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		padding: 3px 6px;
		width: 280px;
	}

	.field:focus {
		outline: none;
		border-color: var(--accent);
	}

	.field::placeholder {
		color: var(--placeholder);
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 0 20px;
		text-align: center;
	}

	.error {
		color: var(--danger);
	}
</style>
