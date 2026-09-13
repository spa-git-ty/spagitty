<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { DiffView } from '$lib/diff/store.svelte';
	import { splitRows } from '$lib/diff/split';
	import type { DiffLine, FileDiff } from '$lib/types';
	import ImageDiff from '$lib/diff/ImageDiff.svelte';
	import BinaryDiffView from '$lib/diff/BinaryDiff.svelte';
	import * as api from '$lib/api';
	import type { BinaryDiff, DiffSide } from '$lib/types';
	import { detectLanguage, highlightLine } from '$lib/diff/highlight';

	/**
	 * The hunks of the selected file, unified or side by side.
	 *
	 * Line numbers come from the core, so both views can label a line without
	 * counting rows themselves — which is what makes the split view's blank
	 * cells harmless.
	 *
	 * Given its file rather than reading a store, so the Stash screen renders an
	 * entry's files with this component (FEAT-034) rather than a second diff
	 * renderer that would have to be kept in step with this one.
	 */

	interface Props {
		/** The file to render, or null while it is being fetched. */
		file: FileDiff | null;
		/** The selected path. Null means nothing is selected yet. */
		path: string | null;
		error: string | null;
		loading: boolean;
		view: DiffView;
		/** Index of the hunk to bring into view. Driven by `j` / `k`. */
		focus?: number;
		binaryDiff?: BinaryDiff | null;
		commitId?: string | null;
		side?: DiffSide | null;
	}

	let {
		file,
		path,
		error,
		loading,
		view,
		focus = 0,
		binaryDiff = null,
		commitId = null,
		side = null
	}: Props = $props();
	const language = $derived(detectLanguage(path));

	let binaryData = $state<BinaryDiff | null>(null);
	let binaryLoading = $state(false);

	$effect(() => {
		if (file?.binary && path) {
			if (binaryDiff) {
				binaryData = binaryDiff;
				return;
			}
			binaryLoading = true;
			if (commitId) {
				void api.binaryFileDiff(commitId, path)
					.then((res) => { binaryData = res; })
					.catch(() => { binaryData = null; })
					.finally(() => { binaryLoading = false; });
			} else if (side) {
				void api.binaryWorkingDiff(path, side)
					.then((res) => { binaryData = res; })
					.catch(() => { binaryData = null; })
					.finally(() => { binaryLoading = false; });
			} else {
				binaryLoading = false;
			}
		} else {
			binaryData = null;
		}
	});

	let hunkEls: HTMLElement[] = [];

	$effect(() => {
		// Reading both makes this run on a new file as well as a new focus.
		const target = hunkEls[focus];
		if (file && target) target.scrollIntoView({ block: 'start' });
	});

	function sign(line: DiffLine): string {
		if (line.origin === 'added') return '+';
		if (line.origin === 'removed') return '−';
		return ' ';
	}
</script>

<div class="pane">
	{#if error}
		<div class="pad note error">{error}</div>
	{:else if path === null}
		<div class="pad note">Select a file.</div>
	{:else if file === null}
		<div class="pad note">{loading ? 'Reading…' : ''}</div>
	{:else if file.binary}
		{#if binaryData?.isImage}
			<ImageDiff diff={binaryData} />
		{:else if binaryData}
			<BinaryDiffView diff={binaryData} />
		{:else if binaryLoading}
			<div class="pad note">Reading binary metadata…</div>
		{:else}
			<div class="pad note">Binary file. There are no lines to show.</div>
		{/if}
	{:else if file.tooLarge}
		<div class="pad note">This file is too large to diff.</div>
	{:else if file.hunks.length === 0}
		<div class="pad note">No line changes — only the file's mode changed.</div>
	{:else if view === 'unified'}
		{#each file.hunks as hunk, index (hunk.header + index)}
			<section class="hunk" bind:this={hunkEls[index]}>
				<div class="hunk-head mono">{hunk.header}</div>
				{#each hunk.lines as line, row (row)}
					<div class="line {line.origin}">
						<span class="num">{line.old ?? ''}</span>
						<span class="num">{line.new ?? ''}</span>
						<span class="sign">{sign(line)}</span>
						<span class="text">{@html highlightLine(line.text, language)}</span>
					</div>
				{/each}
			</section>
		{/each}
	{:else}
		<div class="split-head">
			<span class="note">before</span>
			<span class="note">after</span>
		</div>
		{#each file.hunks as hunk, index (hunk.header + index)}
			<section class="hunk" bind:this={hunkEls[index]}>
				<div class="hunk-head mono">{hunk.header}</div>
				{#each splitRows(hunk.lines) as row, i (i)}
					<div class="pair">
						<div class="side {row.left?.origin ?? 'blank'}">
							{#if row.left}
								<span class="num">{row.left.old ?? ''}</span>
								<span class="text">{@html highlightLine(row.left.text, language)}</span>
							{/if}
						</div>
						<div class="side {row.right?.origin ?? 'blank'}">
							{#if row.right}
								<span class="num">{row.right.new ?? ''}</span>
								<span class="text">{@html highlightLine(row.right.text, language)}</span>
							{/if}
						</div>
					</div>
				{/each}
			</section>
		{/each}
	{/if}
</div>

<style>
	.pane {
		flex: 1;
		min-width: 0;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: var(--fs-code);
	}

	.pad {
		padding: 10px 12px;
	}

	.error {
		color: var(--danger);
	}

	/*
	 * Same fill as the lines, not `--panel`. The file list is `--panel` with a
	 * right rule; a panel-coloured header sits on that rule and the `@@` row
	 * looks unframed while the code below it does not.
	 */
	.hunk-head {
		color: var(--muted);
		background: var(--bg);
		border-top: 1px solid var(--soft);
		border-bottom: 1px solid var(--soft);
		padding: 2px 8px;
		position: sticky;
		top: 0;
		z-index: 1;
	}

	.split-head {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5px;
		padding: 4px 8px;
		font-family: var(--font-ui);
	}

	/* Unified rows are as wide as their longest line, so the tint spans the
	   whole row when the pane is scrolled sideways. */
	.line {
		display: flex;
		min-width: 100%;
		width: max-content;
	}

	.num {
		flex: none;
		width: var(--diff-gutter-w);
		padding-right: 8px;
		text-align: right;
		color: var(--muted);
		user-select: none;
	}

	.sign {
		flex: none;
		width: 14px;
		text-align: center;
		color: var(--muted);
		user-select: none;
	}

	.text {
		white-space: pre;
		tab-size: 4;
		padding-right: 12px;
	}

	.line.added,
	.side.added {
		background: color-mix(in srgb, var(--ok) 14%, transparent);
	}

	.line.removed,
	.side.removed {
		background: color-mix(in srgb, var(--danger) 14%, transparent);
	}

	/* The empty half of an uneven pairing is not a line that exists. */
	.side.blank {
		background: var(--stripe);
	}

	/*
	 * Two columns of equal width. Both cells of a row are in the same grid row,
	 * so a line that wraps takes its opposite number down with it and the two
	 * sides stay level.
	 */
	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5px;
	}

	.side {
		display: flex;
		min-width: 0;
	}

	/* Split columns are half as wide, so long lines wrap rather than forcing a
	   sideways scroll that would move both sides at once. */
	.side .text {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		min-width: 0;
	}
</style>
