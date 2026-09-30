<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	/**
	 * A changed file, named the way a person looks for it (TASK-046).
	 *
	 * The name first, at full strength, then the folder it lives in, quieter.
	 * When the row runs out of room the folder gives way, from its end, and the
	 * name keeps its place — so every row is readable from its first letter.
	 *
	 * The lists used to show the whole path with `direction: rtl`, which cut the
	 * path's head: a long path read `…s/services/debit-card-pair-cache.service.ts`,
	 * starting mid-word, and a dotfile needed an invisible mark to stay
	 * `.gitignore` rather than `gitignore.`.
	 *
	 * The change is a small lettered badge in its own colour instead of `~`, `?`
	 * and `+`: M modified, A added, D deleted, R renamed, U untracked, ! in
	 * conflict. The letter is the one git itself prints in `status --short`.
	 */
	interface Props {
		path: string;
		status: string;
	}

	let { path, status }: Props = $props();

	const slash = $derived(path.lastIndexOf('/'));
	const name = $derived(path.slice(slash + 1));
	const folder = $derived(slash > 0 ? path.slice(0, slash) : '');

	const BADGES: Record<string, { letter: string; label: string }> = {
		modified: { letter: 'M', label: 'Modified' },
		added: { letter: 'A', label: 'Added' },
		deleted: { letter: 'D', label: 'Deleted' },
		renamed: { letter: 'R', label: 'Renamed' },
		untracked: { letter: 'U', label: 'Untracked' },
		conflicted: { letter: '!', label: 'In conflict' }
	};

	const badge = $derived(BADGES[status] ?? { letter: '?', label: status });
</script>

<span class="file-name">
	<span class="glyph {status}" title={badge.label} aria-label={badge.label}>{badge.letter}</span>
	<span class="path">{name}</span>
	{#if folder}<span class="folder">{folder}</span>{/if}
</span>

<style>
	.file-name {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
	}

	.glyph {
		flex: none;
		display: inline-grid;
		place-items: center;
		width: 18px;
		height: 18px;
		border-radius: 6px;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		font-weight: 700;
		line-height: 1;
		color: var(--warn);
		background: var(--warn-soft);
	}

	.glyph.added,
	.glyph.untracked {
		color: var(--ok);
		background: var(--ok-soft);
	}

	.glyph.deleted,
	.glyph.conflicted {
		color: var(--danger);
		background: var(--danger-soft);
	}

	.glyph.renamed {
		color: var(--lane-1);
		background: color-mix(in srgb, var(--lane-1) 16%, transparent);
	}

	/* The name keeps its room; it shrinks only once the folder has gone. */
	.path {
		flex: 0 1 auto;
		min-width: 4ch;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
		color: var(--ink);
	}

	.folder {
		flex: 1 1 0;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-mono);
		color: var(--muted);
	}
</style>
