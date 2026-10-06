<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { goto } from '$app/navigation';
	import { clockTime } from '$lib/format';
	import { stash } from '$lib/stash/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * What the selected entry is, and what could be done with it.
	 *
	 * Pop, Apply and Drop each hand off to `stash.restore`, which puts the
	 * confirmation up through `graph/actions.ts` and re-reads the list. What
	 * each one does is said in that confirmation rather than here, so the Stash
	 * screen and the graph's own stash menu cannot describe the same operation
	 * two different ways.
	 *
	 * The files themselves left this panel in FEAT-034: they are a column of
	 * their own now, beside the pane that shows one of them. What stays here is
	 * everything about the entry that is not a file.
	 */

	const entry = $derived(stash.selected);
	const contents = $derived(stash.contents);

	const counts = $derived.by(() => {
		if (contents === null) return '';
		const files = contents.files.length === 1 ? '1 file' : `${contents.files.length} files`;
		return `${files} · +${contents.added} −${contents.removed}`;
	});

	const position = $derived(
		stash.fileCount === 0 ? '' : `file ${stash.fileIndex + 1} of ${stash.fileCount}`
	);
</script>

<aside class="detail">
	{#if entry === null}
		<div class="empty note">Select an entry to see what is in it.</div>
	{:else}
		<header class="head">
			<span class="mono">{entry.name}</span>
			<span class="mono muted">{entry.short}</span>
		</header>

		<div class="pad column">
			<div class="message">{entry.message}</div>
			<div class="mono muted">
				{entry.authorName} · {clockTime(entry.time)}
			</div>
			<div class="mono muted" title={entry.parentSummary}>
				made on {entry.parentShort}
			</div>

			<div class="hr"></div>

			{#if stash.contentsError}
				<p class="note error">{stash.contentsError}</p>
			{:else if contents === null}
				<p class="note">{#if stash.contentsLoading}<Loader label="Reading…" />{:else}{/if}</p>
			{:else}
				<div class="files-head">
					<span class="note">{counts}</span>
					<Btn onclick={() => goto(`/diff?commit=${entry.id}`)}>Open full diff →</Btn>
				</div>
				{#if position}<span class="note">{position}</span>{/if}
			{/if}

			<div class="hr"></div>

			<div class="actions">
				<span class="note">Restoring this entry</span>
				<div class="chips">
					<Chip
						title="Restore this entry and remove it from the stash"
						onclick={() => stash.restore('pop')}>Pop</Chip
					>
					<Chip
						title="Restore this entry and keep it in the stash"
						onclick={() => stash.restore('apply')}>Apply — keep in stash</Chip
					>
					<Chip title="Delete this entry without restoring it" onclick={() => stash.restore('drop')}
						>Drop</Chip
					>
				</div>
			</div>
		</div>
	{/if}
</aside>

<style>
	.detail {
		width: var(--detail-w);
		flex: none;
		border-left: 1px solid var(--line);
		background: var(--panel);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 8px 10px;
		border-bottom: 1px solid var(--soft);
		flex: none;
	}

	.pad {
		padding: 10px;
		overflow-y: auto;
	}

	.column {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.empty {
		padding: 14px 10px;
	}

	.files-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.actions {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.error {
		color: var(--danger);
	}
</style>
