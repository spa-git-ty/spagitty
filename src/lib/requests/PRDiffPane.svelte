<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { DiffView } from '$lib/diff/store.svelte';
	import { splitRows } from '$lib/diff/split';
	import { relativeTime } from '$lib/format';
	import { requests } from '$lib/requests/store.svelte';
	import type { DiffLine, FileDiff, PullRequestComment } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';

	interface Props {
		file: FileDiff | null;
		path: string | null;
		error: string | null;
		loading: boolean;
		view: DiffView;
	}

	let { file, path, error, loading, view }: Props = $props();

	/** Line currently having an active composer open: `${side}:${lineNum}`. */
	let activeComposer = $state<string | null>(null);
	let composerText = $state('');

	/** Replies in progress: commentId -> text */
	let replyTexts = $state<Record<number, string>>({});
	let replySubmitting = $state<Record<number, boolean>>({});

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

	function getDraftComment(line: DiffLine) {
		const key = lineKey(line);
		if (!key || !path) return null;
		return (
			requests.draftComments.find(
				(c) => c.path === path && c.line === key.num && c.side === key.side
			) ?? null
		);
	}

	function getPublishedComments(line: DiffLine): PullRequestComment[] {
		const key = lineKey(line);
		if (!key || !path) return [];
		return requests.comments.filter(
			(c) => c.path === path && c.line === key.num && c.side === key.side
		);
	}

	function openComposer(line: DiffLine) {
		const key = lineKey(line);
		if (!key) return;
		activeComposer = `${key.side}:${key.num}`;
		const draft = getDraftComment(line);
		composerText = draft ? draft.body : '';
	}

	function closeComposer() {
		activeComposer = null;
		composerText = '';
	}

	function saveDraft(line: DiffLine) {
		const key = lineKey(line);
		if (!key || !path || !composerText.trim()) return;
		requests.addDraftComment(path, key.num, key.side, composerText);
		closeComposer();
	}

	function removeDraft(line: DiffLine) {
		const key = lineKey(line);
		if (!key || !path) return;
		requests.removeDraftComment(path, key.num, key.side);
	}

	async function submitReply(commentId: number) {
		const text = replyTexts[commentId];
		if (!text || !text.trim()) return;
		replySubmitting[commentId] = true;
		const ok = await requests.replyToComment(commentId, text.trim());
		replySubmitting[commentId] = false;
		if (ok) {
			replyTexts[commentId] = '';
		}
	}
</script>

<div class="pr-diff-pane">
	{#if error}
		<div class="pad note error">{error}</div>
	{:else if path === null}
		<div class="pad note">Select a file.</div>
	{:else if file === null}
		<div class="pad note">{loading ? 'Reading diff…' : ''}</div>
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
					{@const key = lineKey(line)}
					{@const composerId = key ? `${key.side}:${key.num}` : ''}
					{@const draft = getDraftComment(line)}
					{@const published = getPublishedComments(line)}
					{@const hasComments = draft !== null || published.length > 0 || activeComposer === composerId}

					<div class="line-row {line.origin}" class:has-comments={hasComments}>
						<div class="line-content">
							<span class="num">{line.old ?? ''}</span>
							<span class="num">{line.new ?? ''}</span>
							<span class="sign">{sign(line)}</span>
							<span class="text">{line.text}</span>

							<button
								class="comment-trigger"
								title="Add inline review comment"
								onclick={() => openComposer(line)}
							>
								<Icon name="plus" size="0.9em" />
							</button>
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

										{#if requests.role === 'developer' && !comment.resolved}
											<div class="developer-actions">
												<button
													class="resolve-btn"
													onclick={() => requests.resolveComment(comment.id)}
												>
													Mark as resolved
												</button>
											</div>
										{/if}

										<!-- Reply box for developer or reviewer -->
										<div class="reply-section">
											<input
												type="text"
												class="reply-input"
												placeholder="Reply to this thread…"
												bind:value={replyTexts[comment.id]}
												onkeydown={(e) => {
													if (e.key === 'Enter') submitReply(comment.id);
												}}
											/>
											<Btn
												disabled={!replyTexts[comment.id]?.trim() || replySubmitting[comment.id]}
												onclick={() => submitReply(comment.id)}
											>
												Reply
											</Btn>
										</div>
									</div>
								{/each}

								{#if draft}
									<div class="comment-card draft">
										<div class="comment-header">
											<span class="author-badge mono">Pending Review Draft</span>
											<button class="delete-draft-btn" onclick={() => removeDraft(line)}>
												Delete
											</button>
										</div>
										<div class="comment-body">{draft.body}</div>
									</div>
								{/if}

								{#if activeComposer === composerId}
									<div class="composer-card">
										<div class="composer-title note">Add inline review comment (Line {key?.num})</div>
										<textarea
											class="composer-input"
											rows="3"
											placeholder="Leave a comment or change request on this line…"
											bind:value={composerText}
										></textarea>
										<div class="composer-actions">
											<Btn primary disabled={!composerText.trim()} onclick={() => saveDraft(line)}>
												Add to review
											</Btn>
											<Btn onclick={closeComposer}>Cancel</Btn>
										</div>
									</div>
								{/if}
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
								<span class="text">{row.left.text}</span>
								<button
									class="comment-trigger"
									title="Add inline review comment"
									onclick={() => row.left && openComposer(row.left)}
								>
									<Icon name="plus" size="0.9em" />
								</button>
							{/if}
						</div>
						<div class="side {row.right?.origin ?? 'blank'}">
							{#if row.right}
								<span class="num">{row.right.new ?? ''}</span>
								<span class="text">{row.right.text}</span>
								<button
									class="comment-trigger"
									title="Add inline review comment"
									onclick={() => row.right && openComposer(row.right)}
								>
									<Icon name="plus" size="0.9em" />
								</button>
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

	.hunk-head {
		color: var(--muted);
		background: var(--panel);
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

	.line-content:hover .comment-trigger {
		opacity: 1;
		pointer-events: auto;
	}

	.comment-trigger {
		position: absolute;
		left: 4px;
		top: 2px;
		width: 18px;
		height: 18px;
		border-radius: 3px;
		border: 1px solid var(--line);
		background: var(--surface);
		color: var(--ink);
		display: grid;
		place-items: center;
		opacity: 0;
		pointer-events: none;
		cursor: pointer;
		z-index: 3;
		padding: 0;
		box-shadow: none;
	}

	.comment-trigger:hover {
		background: var(--accent);
		color: var(--on-accent);
		border-color: var(--accent);
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

	.side:hover .comment-trigger {
		opacity: 1;
		pointer-events: auto;
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
	.composer-card {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--r-panel);
		padding: 8px 10px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		box-shadow: none;
	}

	.comment-card.draft {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 8%, var(--surface));
	}

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

	.delete-draft-btn {
		background: none;
		border: none;
		color: var(--danger);
		font-size: var(--fs-secondary);
		cursor: pointer;
		padding: 0;
	}

	.delete-draft-btn:hover {
		text-decoration: underline;
	}

	.comment-body {
		font-size: var(--fs-ui);
		white-space: pre-wrap;
		color: var(--ink);
	}

	.developer-actions {
		display: flex;
		gap: 6px;
		margin-top: 2px;
	}

	.resolve-btn {
		background: transparent;
		border: 1px solid var(--ok);
		color: var(--ok);
		border-radius: var(--r-button);
		padding: 2px 8px;
		font-size: var(--fs-secondary);
		cursor: pointer;
	}

	.resolve-btn:hover {
		background: var(--ok);
		color: var(--on-accent);
	}

	.reply-section {
		display: flex;
		gap: 6px;
		margin-top: 4px;
	}

	.reply-input {
		flex: 1;
		padding: 4px 8px;
		font: inherit;
		font-size: var(--fs-secondary);
		background: var(--bg);
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
	}

	.composer-input {
		width: 100%;
		padding: 6px 8px;
		font: inherit;
		font-size: var(--fs-ui);
		background: var(--bg);
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		resize: vertical;
		box-sizing: border-box;
	}

	.composer-actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}
</style>
