<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { changes } from '$lib/changes/store.svelte';
	import FileColumn from '$lib/changes/FileColumn.svelte';
	import HunkPane from '$lib/changes/HunkPane.svelte';
	import MessageBox from '$lib/changes/MessageBox.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';

	/**
	 * Stage what you mean to commit, write the message, commit.
	 *
	 * Nothing on this screen writes to the repository until a button is pressed,
	 * and nothing it offers can discard work: staging, unstaging and committing
	 * only ever move changes forward. Discarding is a different screen's job and
	 * a different decision.
	 */

	// The open repository is what drives a load: this screen can be the first
	// one painted, and the walk that opens the repository finishes after the
	// component mounts. The call is untracked because it writes to the store it
	// reads, which would otherwise fetch twice.
	let generation: number | null = null;
	$effect(() => {
		const current = repo.generation;
		if (repo.info === null) return;
		if (generation === current) return;

		generation = current;
		untrack(() => {
			changes.clear();
			changes.load();
		});
	});

	const work = $derived(changes.work);
	const clean = $derived(
		changes.loaded &&
			work.staged.length === 0 &&
			work.unstaged.length === 0 &&
			work.conflicted.length === 0
	);

	const summary = $derived.by(() => {
		const staged = work.staged.length;
		const unstaged = work.unstaged.length;
		const files = (n: number) => (n === 1 ? '1 file' : `${n} files`);
		return `${files(staged)} staged · ${files(unstaged)} not staged`;
	});

	function commitLabel(): string {
		if (changes.amend) return 'Amend the previous commit';
		const staged = work.staged.length;
		if (staged === 0) return 'Commit';
		return staged === 1 ? 'Commit 1 file' : `Commit ${staged} files`;
	}
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Commit</span>
			{#if changes.loaded}<span class="note">{summary}</span>{/if}
		</div>
		<div class="right">
			{#if changes.loading}<span class="note">Reading…</span>{/if}
			<Btn disabled={changes.busy} onclick={() => changes.load()}>Refresh</Btn>
		</div>
	</header>

	{#if repo.info === null}
		<div class="empty"><p class="note">No repository open.</p></div>
	{:else if changes.error}
		<div class="empty"><p class="note error">{changes.error}</p></div>
	{:else if !changes.loaded}
		<div class="empty"><span class="note">Reading the working copy…</span></div>
	{:else if clean}
		<div class="empty">
			<p class="note">Nothing to commit. The working copy matches the last commit.</p>
		</div>
	{:else}
		<div class="body">
			<FileColumn />
			<Splitter panel="changesFiles" label="Resize the file list" />
			<div class="right-pane">
				<MessageBox />
				<HunkPane />
			</div>
		</div>
	{/if}

	<footer class="foot">
		<div class="left">
			{#if changes.writeError}
				<span class="note error">{changes.writeError}</span>
			{:else if work.conflicted.length > 0}
				<span class="note">Resolve the conflicts before committing.</span>
			{/if}
		</div>
		<Btn primary disabled={!changes.canCommit} onclick={() => changes.commit()}>
			{commitLabel()}
		</Btn>
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
		border-bottom: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

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

	.title {
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		overflow: hidden;
	}

	.right-pane {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
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
