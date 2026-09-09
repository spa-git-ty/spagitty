<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { BinaryDiff } from '$lib/types';

	interface Props {
		diff: BinaryDiff;
	}

	let { diff }: Props = $props();

	function formatBytes(bytes: number | null): string {
		if (bytes === null) return '0 B';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
	}

	const delta = $derived.by(() => {
		if (diff.oldSize === null && diff.newSize !== null) {
			return { text: `+${formatBytes(diff.newSize)}`, sign: 'added' };
		}
		if (diff.oldSize !== null && diff.newSize === null) {
			return { text: `-${formatBytes(diff.oldSize)}`, sign: 'removed' };
		}
		if (diff.oldSize !== null && diff.newSize !== null) {
			const diffSize = diff.newSize - diff.oldSize;
			if (diffSize === 0) return { text: '0 B', sign: 'same' };
			if (diffSize > 0) return { text: `+${formatBytes(diffSize)}`, sign: 'added' };
			return { text: `-${formatBytes(Math.abs(diffSize))}`, sign: 'removed' };
		}
		return { text: '0 B', sign: 'same' };
	});
</script>

<div class="binary-diff" aria-label="Binary diff metadata">
	<div class="card">
		<div class="icon">📦</div>
		<div class="title">Binary File Changed</div>
		<div class="path mono">{diff.path}</div>

		<div class="stats-grid">
			<div class="stat-col">
				<span class="stat-label">Previous Size</span>
				<span class="stat-val mono">{formatBytes(diff.oldSize)}</span>
			</div>
			<div class="stat-arrow">→</div>
			<div class="stat-col">
				<span class="stat-label">New Size</span>
				<span class="stat-val mono">{formatBytes(diff.newSize)}</span>
			</div>
			<div class="stat-col delta" class:added={delta.sign === 'added'} class:removed={delta.sign === 'removed'}>
				<span class="stat-label">Delta</span>
				<span class="stat-val mono">{delta.text}</span>
			</div>
		</div>

		<div class="meta-row">
			<span class="meta-label">MIME Type:</span>
			<span class="mime-pill mono">{diff.mime}</span>
		</div>
	</div>
</div>

<style>
	.binary-diff {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1;
		height: 100%;
		padding: 32px;
		background: var(--bg);
	}

	.card {
		background: var(--panel);
		border: 1px solid var(--line);
		border-radius: var(--r-field, 6px);
		padding: 24px 32px;
		max-width: 480px;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 12px;
	}

	.icon {
		font-size: 2em;
	}

	.title {
		font-size: var(--fs-ui);
		font-weight: 600;
		color: var(--ink);
	}

	.path {
		font-size: var(--fs-secondary);
		color: var(--muted);
		word-break: break-all;
	}

	.stats-grid {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 16px;
		padding: 16px;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 6px;
		width: 100%;
		margin-top: 6px;
	}

	.stat-col {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.stat-arrow {
		color: var(--muted);
		font-size: var(--fs-ui);
	}

	.stat-label {
		font-size: var(--fs-mono);
		color: var(--muted);
		text-transform: uppercase;
		font-weight: 500;
	}

	.stat-val {
		font-size: var(--fs-secondary);
		color: var(--ink);
	}

	.delta.added .stat-val {
		color: var(--accent);
	}

	.delta.removed .stat-val {
		color: var(--ok);
	}

	.meta-row {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: var(--fs-secondary);
		color: var(--muted);
		margin-top: 4px;
	}

	.mime-pill {
		background: var(--soft);
		padding: 2px 8px;
		border-radius: 4px;
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.mono {
		font-family: var(--font-mono);
	}
</style>
