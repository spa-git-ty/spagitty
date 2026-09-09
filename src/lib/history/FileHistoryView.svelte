<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { fileHistory } from './store.svelte';
	import { graph } from '$lib/graph/store.svelte';
	import { relativeTime } from '$lib/format';
	import { goto } from '$app/navigation';
	import { notice } from '$lib/ui/notice.svelte';
	import { detectLanguage, highlightLine } from '$lib/diff/highlight';
	import Btn from '$lib/ui/Btn.svelte';

	let { onclose }: { onclose?: () => void } = $props();

	const path = $derived(fileHistory.path);
	const entries = $derived(fileHistory.entries);
	const blame = $derived(fileHistory.blame);
	const loading = $derived(fileHistory.loading);
	const language = $derived(detectLanguage(path));
	const error = $derived(fileHistory.error);
	const highlighted = $derived(fileHistory.highlightedCommit);

	async function jumpToGraph(commitSha: string): Promise<void> {
		graph.want(commitSha);
		await goto('/');
		if (onclose) onclose();
	}

	async function copyPath(): Promise<void> {
		if (!path) return;
		try {
			await navigator.clipboard.writeText(path);
			notice.ok('Path copied to clipboard');
		} catch (err) {
			notice.failed('Could not copy path', err);
		}
	}
</script>

<div class="history-view" aria-label="File History and Blame">
	<header class="header">
		<div class="header-left">
			<span class="path mono" title={path ?? ''}>{path ?? 'No file selected'}</span>
			{#if path}
				<button class="copy-btn" title="Copy path" onclick={copyPath}>📋</button>
			{/if}
			{#if entries.length > 0}
				<span class="count-badge">{entries.length} commits</span>
			{/if}
		</div>
		<div class="header-right">
			{#if onclose}
				<Btn onclick={onclose}>Close</Btn>
			{/if}
		</div>
	</header>

	{#if loading}
		<div class="loading-state">Loading file history and blame…</div>
	{:else if error}
		<div class="error-state" role="alert">{error}</div>
	{:else if !path}
		<div class="empty-state">Select a file to inspect its history and blame.</div>
	{:else}
		<div class="content-split">
			<!-- Left: Commit Timeline -->
			<aside class="timeline-pane" aria-label="Commit timeline">
				<div class="pane-header">Commits ({entries.length})</div>
				<div class="timeline-list">
					{#each entries as entry (entry.commit)}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="timeline-card"
							class:highlighted={highlighted === entry.commit}
							onmouseenter={() => fileHistory.setHighlight(entry.commit)}
							onmouseleave={() => fileHistory.setHighlight(null)}
						>
							<div class="card-row-top">
								<span class="author">{entry.authorName}</span>
								<span class="time">{relativeTime(entry.time)}</span>
							</div>
							<div class="summary">{entry.summary}</div>
							<div class="card-row-bottom">
								<span class="sha mono">{entry.short}</span>
								<button
									type="button"
									class="graph-jump"
									onclick={() => void jumpToGraph(entry.commit)}
								>View on Graph →</button>
							</div>
						</div>
					{/each}
				</div>
			</aside>

			<!-- Right: Blame Gutter + File Content -->
			<main class="blame-pane" aria-label="Line attribution">
				{#if blame?.refused}
					<div class="refused-message">
						{#if blame.refused === 'binary'}
							Binary file cannot be blamed.
						{:else if blame.refused === 'tooLarge'}
							File is too large to blame.
						{:else if blame.refused === 'notAFile'}
							Path is not a regular file in this revision.
						{:else if blame.refused === 'empty'}
							File is empty.
						{/if}
					</div>
				{:else if blame && blame.lines.length > 0}
					<div class="blame-table mono">
						{#each blame.lines as line (line.line)}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="blame-row"
								class:row-highlight={highlighted === line.commit}
								onmouseenter={() => fileHistory.setHighlight(line.commit)}
								onmouseleave={() => fileHistory.setHighlight(null)}
							>
								<div class="gutter-num">{line.line}</div>
								<div class="gutter-meta" title="{line.authorName} • {line.summary}">
									<span class="meta-author">{line.authorName}</span>
									<span class="meta-sha">{line.short}</span>
									<span class="meta-time">{relativeTime(line.time)}</span>
								</div>
								<div class="code-line">{@html highlightLine(line.text, language)}</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="empty-state">No line attribution available.</div>
				{/if}
			</main>
		</div>
	{/if}
</div>

<style>
	.history-view {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg);
		color: var(--ink);
		overflow: hidden;
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 16px;
		background: var(--surface);
		border-bottom: 1px solid var(--line);
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 8px;
		overflow: hidden;
	}

	.path {
		font-size: var(--fs-secondary);
		font-weight: 500;
		color: var(--ink);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 450px;
	}

	.copy-btn {
		background: transparent;
		border: none;
		cursor: pointer;
		font-size: var(--fs-secondary);
		padding: 2px 4px;
	}

	.count-badge {
		background: var(--soft);
		color: var(--muted);
		font-size: var(--fs-mono);
		padding: 2px 8px;
		border-radius: 10px;
	}

	.content-split {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.timeline-pane {
		width: 320px;
		min-width: 260px;
		background: var(--surface);
		border-right: 1px solid var(--line);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.pane-header {
		padding: 8px 12px;
		font-size: var(--fs-mono);
		font-weight: 600;
		text-transform: uppercase;
		color: var(--muted);
		border-bottom: 1px solid var(--line);
	}

	.timeline-list {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px;
	}

	.timeline-card {
		background: var(--surface-2);
		border: 1px solid var(--line);
		border-radius: 6px;
		padding: 8px 10px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		cursor: default;
		transition: border-color 0.15s, background 0.15s;
	}

	.timeline-card:hover,
	.timeline-card.highlighted {
		border-color: var(--accent);
		background: var(--hover);
	}

	.card-row-top {
		display: flex;
		justify-content: space-between;
		font-size: var(--fs-secondary);
	}

	.author {
		font-weight: 500;
		color: var(--ink);
	}

	.time {
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.summary {
		font-size: var(--fs-secondary);
		color: var(--muted);
		line-height: 1.3;
	}

	.card-row-bottom {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 2px;
	}

	.sha {
		font-size: var(--fs-mono);
		color: var(--accent);
	}

	.graph-jump {
		background: transparent;
		border: none;
		font-size: var(--fs-mono);
		color: var(--muted);
		cursor: pointer;
		padding: 0;
	}

	.graph-jump:hover {
		color: var(--accent);
	}

	.blame-pane {
		flex: 1;
		overflow: auto;
		background: var(--bg);
	}

	.blame-table {
		display: flex;
		flex-direction: column;
		font-size: var(--fs-secondary);
		line-height: 20px;
	}

	.blame-row {
		display: flex;
		border-bottom: 1px solid var(--soft);
		transition: background 0.1s;
	}

	.blame-row:hover,
	.blame-row.row-highlight {
		background: var(--accent-soft);
	}

	.gutter-num {
		width: 45px;
		text-align: right;
		padding-right: 8px;
		color: var(--muted);
		user-select: none;
		flex-shrink: 0;
	}

	.gutter-meta {
		width: 220px;
		padding: 0 8px;
		color: var(--muted);
		background: var(--sunken);
		border-right: 1px solid var(--line);
		display: flex;
		gap: 6px;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		flex-shrink: 0;
		font-size: var(--fs-mono);
	}

	.meta-author {
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.meta-sha {
		color: var(--accent);
	}

	.meta-time {
		color: var(--muted);
	}

	.code-line {
		padding: 0 12px;
		white-space: pre;
		color: var(--ink);
		flex: 1;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.loading-state,
	.empty-state,
	.refused-message {
		padding: 48px;
		text-align: center;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.error-state {
		padding: 16px;
		color: var(--danger);
		background: var(--danger-soft);
		margin: 16px;
		border-radius: 6px;
		font-size: var(--fs-secondary);
	}
</style>
