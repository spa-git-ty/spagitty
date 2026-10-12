<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import type { DiffView } from '$lib/diff/store.svelte';
	import { detectLanguage, tokenizeDiff } from '$lib/diff/highlight';
	import { splitRows } from '$lib/diff/split';
	import { relativeTime } from '$lib/format';
	import { requests } from '$lib/requests/store.svelte';
	import type { DiffLine, FileDiff, PullRequestComment } from '$lib/types';

	interface Props {
		file: FileDiff | null;
		path: string | null;
		error: string | null;
		loading: boolean;
		view: DiffView;
	}

	let { file, path, error, loading, view }: Props = $props();

	/** Each line's colours: a hunk is read as the two sides of its diff, so a comment or string that spans lines stays coloured. */
	const syntax = $derived.by(() => {
		const colours = new Map<DiffLine, ReturnType<typeof tokenizeDiff>[number]>();
		if (!file) return colours;
		const language = detectLanguage(path);
		for (const hunk of file.hunks) {
			const runs = tokenizeDiff(hunk.lines, language);
			hunk.lines.forEach((line, i) => colours.set(line, runs[i]));
		}
		return colours;
	});

	function sign(line: DiffLine): string {
		if (line.origin === 'added') return '+';
		if (line.origin === 'removed') return '−';
		return ' ';
	}

	function lineKey(line: DiffLine): { num: number; side: 'LEFT' | 'RIGHT' } | null {
		if (line.origin === 'removed' && line.old !== null) {
			return { num: line.old, side: 'LEFT' };
		}
		if (line.origin === 'added' && line.new !== null) {
			return { num: line.new, side: 'RIGHT' };
		}
		if (line.origin === 'context') {
			return { num: line.new ?? line.old ?? 1, side: 'RIGHT' };
		}
		return null;
	}

	/** The comments already on a line: read here, answered in Review (FEAT-116). */
	function getPublishedComments(line: DiffLine): PullRequestComment[] {
		const key = lineKey(line);
		if (!key || !path) return [];
		return requests.comments.filter(
			(c) => c.path === path && c.line === key.num && c.side === key.side
		);
	}
</script>

{#snippet code(line: DiffLine)}<span class="text"
	>{#each syntax.get(line) ?? [{ type: 'plain', text: line.text }] as run, r (r)}<span class="tok-{run.type}">{run.text}</span
		>{/each}</span
>{/snippet}

<div class="pr-diff-pane">
	{#if error}
		<div class="pad note error">{error}</div>
	{:else if path === null}
		<div class="pad note">Select a file.</div>
	{:else if file === null}
		<div class="pad note">{#if loading}<Loader label="Reading diff…" />{:else}{/if}</div>
	{:else if file.binary}
		<div class="pad note">Binary file. There are no lines to show.</div>
	{:else if file.tooLarge}
		<div class="pad note">This file is too large to diff.</div>
	{:else if file.hunks.length === 0}
		<div class="pad note">No line changes — only the file's mode changed.</div>
	{:else if view === 'unified'}
		{#each file.hunks as hunk, index (hunk.header + index)}
			<section class="hunk">
				<div class="hunk-head mono">{hunk.header}</div>
				{#each hunk.lines as line, row (row)}
					{@const published = getPublishedComments(line)}
					{@const hasComments = published.length > 0}

					<div class="line-row {line.origin}" class:has-comments={hasComments}>
						<div class="line-content">
							<span class="num">{line.old ?? ''}</span>
							<span class="num">{line.new ?? ''}</span>
							<span class="sign">{sign(line)}</span>
							{@render code(line)}
						</div>

						{#if hasComments}
							<div class="inline-threads">
								{#each published as comment (comment.id)}
									<div class="comment-card" class:resolved={comment.resolved}>
										<div class="comment-header">
											<span class="author-badge mono">{comment.author}</span>
											<span class="comment-time note">{relativeTime(comment.createdAt)}</span>
											{#if comment.resolved}
												<span class="resolved-chip">✓ Resolved</span>
											{/if}
										</div>
										<div class="comment-body">{comment.body}</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			</section>
		{/each}
	{:else}
		<!-- Split View -->
		<div class="split-head">
			<span class="note">before</span>
			<span class="note">after</span>
		</div>
		{#each file.hunks as hunk, index (hunk.header + index)}
			<section class="hunk">
				<div class="hunk-head mono">{hunk.header}</div>
				{#each splitRows(hunk.lines) as row, i (i)}
					<div class="pair">
						<div class="side {row.left?.origin ?? 'blank'}">
							{#if row.left}
								<span class="num">{row.left.old ?? ''}</span>
								{@render code(row.left)}
							{/if}
						</div>
						<div class="side {row.right?.origin ?? 'blank'}">
							{#if row.right}
								<span class="num">{row.right.new ?? ''}</span>
								{@render code(row.right)}
							{/if}
						</div>
					</div>
				{/each}
			</section>
		{/each}
	{/if}
</div>

<style>
	.pr-diff-pane {
		flex: 1;
		min-width: 0;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: var(--fs-code);
		background: var(--bg);
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
		border-top: 1px solid var(--line);
		border-bottom: 1px solid var(--line);
		padding: 3px 8px;
		position: sticky;
		top: 0;
		z-index: 2;
	}

	.line-row {
		display: flex;
		flex-direction: column;
		min-width: 100%;
		width: max-content;
		position: relative;
	}

	.line-content {
		display: flex;
		align-items: center;
		min-width: 100%;
		position: relative;
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

	.line-row.added,
	.side.added {
		background: color-mix(in srgb, var(--ok) 14%, transparent);
	}

	.line-row.removed,
	.side.removed {
		background: color-mix(in srgb, var(--danger) 14%, transparent);
	}

	.side.blank {
		background: var(--stripe);
	}

	.split-head {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5px;
		padding: 4px 8px;
		font-family: var(--font-ui);
	}

	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5px;
	}

	.side {
		display: flex;
		position: relative;
		min-width: 0;
	}

	.side .text {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		min-width: 0;
	}

	/* Inline Thread Styling */
	.inline-threads {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 6px 12px 6px calc(var(--diff-gutter-w) * 2 + 20px);
		background: var(--panel);
		border-top: 1px solid var(--line);
		border-bottom: 1px solid var(--line);
		font-family: var(--font-ui);
	}

	.comment-card,
	.comment-card.resolved {
		opacity: 0.75;
		background: var(--hover);
	}

	.comment-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.author-badge {
		font-weight: 600;
		font-size: var(--fs-secondary);
		color: var(--ink);
	}

	.resolved-chip {
		font-size: var(--fs-secondary);
		color: var(--ok);
		font-weight: 500;
	}

	.comment-body {
		font-size: var(--fs-ui);
		white-space: pre-wrap;
		color: var(--ink);
	}

</style>
