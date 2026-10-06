<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Chip from '$lib/ui/Chip.svelte';
	import { room } from './room.svelte';

	/**
	 * The files a pull request touches, each with its viewed tick
	 * (FEAT-091), and which of them hold code written while fixing a merge
	 * conflict (FEAT-092).
	 *
	 * A tick is kept against the file's content, not its name: when the author
	 * changes a ticked file, it comes back unticked, and a file still bold is
	 * one still to read.
	 */

	function name(path: string): string {
		return path.slice(path.lastIndexOf('/') + 1);
	}

	// Conflict fixes are found in the fetched head; the host's patch has no
	// merges to re-do.
	const local = $derived(room.head !== null);
</script>

<aside class="files" aria-label="Touched files">
	{#if local}
		<div class="filters">
			<Chip active={room.filter === 'all'} onclick={() => room.setFilter('all')}>All {room.files.length}</Chip>
			<Chip active={room.filter === 'author'} onclick={() => room.setFilter('author')}>Author</Chip>
			<Chip active={room.filter === 'conflict'} onclick={() => room.setFilter('conflict')}>Conflict fixes</Chip>
		</div>
	{/if}
	<ul>
		{#each room.visible as file (file.path)}
			{@const viewed = room.isViewed(file.path)}
			{@const threads = room.openThreadsOn(file.path)}
			<li class:on={room.layout === 'one' && room.selected === file.path}>
				<input
					type="checkbox"
					aria-label="Viewed {name(file.path)}"
					checked={viewed}
					onchange={() => room.toggleViewed(file.path)}
				/>
				<button class="pick" title={file.path} onclick={() => room.select(file.path)}>
					<span class="name mono" class:viewed>{name(file.path)}</span>
					<span class="facts">
						<span class="mono"><span class="add">+{file.added}</span> <span class="del">−{file.removed}</span></span>
						{#if room.hasFix(file.path)}<span class="fix"><span class="dot"></span>conflict fix</span>{/if}
						{#if threads > 0}<span class="threads">{threads} {threads === 1 ? 'thread' : 'threads'}</span>{/if}
					</span>
				</button>
			</li>
		{:else}
			<li class="none note">
				{room.filter === 'conflict' ? 'No conflict fixes in this pull request.' : 'No files.'}
			</li>
		{/each}
	</ul>
	{#if local}
		<div class="legend note">
			<span><span class="dot author"></span>Author's own commits</span>
			<span><span class="dot"></span>Written while fixing a merge conflict</span>
			{#if room.conflictsError}
				<span class="error" title={room.conflictsError}>Conflict fixes could not be looked for.</span>
			{:else if room.conflicts?.truncated}
				<span>Only the newest 20 merges were looked at.</span>
			{/if}
		</div>
	{/if}
</aside>

<style>
	.files {
		width: 262px;
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		padding: 4px 8px 10px;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 4px 0 12px;
		overflow: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	li {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--r-panel);
	}

	li:hover:not(.none) {
		background: var(--hover);
	}

	li.on {
		background: color-mix(in srgb, var(--accent) 14%, transparent);
	}

	input {
		width: 16px;
		height: 16px;
		margin: 0;
		flex: none;
		accent-color: var(--ok);
	}

	.pick {
		flex: 1;
		min-width: 0;
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.name {
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--ink);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Read: it steps back, so what is left to read stands out. */
	.name.viewed {
		font-weight: 400;
		color: var(--muted);
	}

	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 0 8px;
		align-items: center;
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.add {
		color: var(--ok);
	}

	.del {
		color: var(--danger);
	}

	.fix {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--resolve);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--resolve);
		flex: none;
	}

	.dot.author {
		background: var(--muted);
	}

	.threads {
		color: var(--accent);
	}

	.legend {
		display: flex;
		flex-direction: column;
		gap: 5px;
		padding: 10px 10px 14px;
		font-size: var(--fs-mono);
	}

	.legend > span {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.error {
		color: var(--danger);
	}
</style>
