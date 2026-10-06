<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { changes } from '$lib/changes/store.svelte';
	import { discardHunk } from '$lib/changes/discard';
	import Chip from '$lib/ui/Chip.svelte';
	import type { DiffLine } from '$lib/types';
	import { pairWords } from '$lib/diff/words';

	/**
	 * The hunks of the selected file, each with the actions that make sense for
	 * the side it is on: stage it or throw it away on the unstaged side, take it
	 * back out on the staged side.
	 *
	 * Discard is only offered where there is something to discard. On the staged
	 * side the change has been kept once already, and unstaging it is one click
	 * away from being discardable — offering both there would put a destructive
	 * button next to a reversible one that reads almost the same.
	 *
	 * Deliberately unified only. Split is for reading a commit someone else
	 * wrote; here the question is "does this hunk belong in the commit", and one
	 * column keeps the answer next to the button.
	 */

	const file = $derived(changes.file);
	/** The changed words in each hunk's changed lines (FEAT-090). */
	const marks = $derived(file ? file.hunks.map((hunk) => pairWords(hunk.lines)) : []);
	const side = $derived(changes.selection?.side ?? 'unstaged');

	function sign(line: DiffLine): string {
		if (line.origin === 'added') return '+';
		if (line.origin === 'removed') return '−';
		return ' ';
	}
</script>

<div class="pane">
	{#if changes.fileError}
		<div class="pad note error">{changes.fileError}</div>
	{:else if changes.selection === null}
		<div class="pad note">Select a file to see what changed in it.</div>
	{:else if file === null}
		<div class="pad note">{#if changes.fileLoading}<Loader label="Reading…" />{:else}Could not read this diff.{/if}</div>
	{:else if file.binary}
		<div class="pad note">Binary file. There are no hunks to stage individually.</div>
	{:else if file.tooLarge}
		<div class="pad note">This file is too large to diff.</div>
	{:else if file.hunks.length === 0}
		<div class="pad note">No line changes — only the file's mode changed.</div>
	{:else}
		{#each file.hunks as hunk, index (hunk.header + index)}
			<section class="hunk">
				<div class="hunk-head">
					<span class="mono muted">{hunk.header}</span>
					<div class="acts">
						<Chip
							onclick={() => changes.hunk(index, hunk.header)}
							title={side === 'unstaged'
								? 'Stage this hunk and nothing else'
								: 'Take this hunk back out of the next commit'}
						>
							{side === 'unstaged' ? 'stage hunk' : 'unstage hunk'}
						</Chip>
						{#if side === 'unstaged'}
							<Chip
								danger
								onclick={() => discardHunk(index, hunk.header)}
								title="Throw these lines away — this cannot be undone"
							>
								discard hunk
							</Chip>
						{/if}
					</div>
				</div>
				{#each hunk.lines as line, row (row)}
					<div class="line {line.origin}">
						<span class="num">{line.old ?? ''}</span>
						<span class="num">{line.new ?? ''}</span>
						<span class="sign">{sign(line)}</span>
						<span class="text"
							>{#each marks[index]?.get(row) ?? [{ text: line.text, changed: false }] as piece, p (p)}<span
									class:word={piece.changed}>{piece.text}</span
								>{/each}</span
						>
					</div>
				{/each}
			</section>
		{/each}
	{/if}
</div>

<style>
	.acts {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	/*
	 * Nested column flex. `flex-basis: 0` is what claims the leftover height
	 * even when the commit-message well has a large intrinsic size — WebKitGTK
	 * otherwise sizes this pane to zero at 100% zoom, and the hunks only appear
	 * after a zoom change forces a relayout. `min-height: 0` is what then lets
	 * the pane scroll inside that leftover rather than overflow it.
	 */
	.pane {
		flex: 1 1 0;
		min-width: 0;
		min-height: 0;
		overflow: auto;
		font-family: var(--code-font);
		font-size: var(--fs-code);
		line-height: var(--code-lh);
		letter-spacing: var(--code-ls);
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
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		background: var(--bg);
		border-top: 1px solid var(--soft);
		border-bottom: 1px solid var(--soft);
		padding: 2px 8px;
		position: sticky;
		top: 0;
		z-index: 1;
	}

	/* Rows are as wide as their longest line, so the tint spans the whole row
	   when the pane is scrolled sideways. */
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

	/* Settings › Reading decides the strength (FEAT-090): calm is a marker
	   down the side and a faint tint, classic the full rows. */
	.line.added {
		background: var(--diff-add-bg);
		box-shadow: inset var(--diff-marker) 0 0 var(--ok);
	}

	.line.removed {
		background: var(--diff-del-bg);
		box-shadow: inset var(--diff-marker) 0 0 var(--danger);
		color: var(--diff-del-ink);
	}

	.line.added .word {
		background: var(--diff-add-hl);
		border-radius: 3px;
	}

	.line.removed .word {
		background: var(--diff-del-hl);
		border-radius: 3px;
	}
</style>
