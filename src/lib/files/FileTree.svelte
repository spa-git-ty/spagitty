<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import type { FilesEntry } from '$lib/types';
	import { files } from './store.svelte';

	/**
	 * The repository's tree at the revision (FEAT-131).
	 *
	 * A folder is read when it is opened, so a repository of a hundred
	 * thousand files costs what the folders somebody looked into cost. Typing
	 * in the field looks through every path at the revision instead, and the
	 * tree comes back when the field is emptied.
	 */
	interface Row {
		entry: FilesEntry;
		depth: number;
	}

	/** The folders that are open, flattened to the rows they show. */
	const rows = $derived.by(() => {
		const out: Row[] = [];
		const walk = (dir: string, depth: number) => {
			const held = files.folder(dir);
			if (!Array.isArray(held)) return;
			for (const entry of held) {
				out.push({ entry, depth });
				if (entry.kind === 'folder' && files.open.includes(entry.path)) walk(entry.path, depth + 1);
			}
		};
		walk('', 0);
		return out;
	});

	const root = $derived(files.folder(''));
	const here = $derived(files.tab?.path ?? null);

	let timer: ReturnType<typeof setTimeout> | null = null;

	/** Wait for a pause in the typing: every letter is a walk of the tree. */
	function type(text: string) {
		if (timer) clearTimeout(timer);
		timer = setTimeout(() => void files.findFiles(text), 140);
	}

	function press(entry: FilesEntry) {
		if (entry.kind === 'folder') files.toggleFolder(entry.path);
		else if (entry.kind !== 'submodule') void files.openFile(entry.path);
	}

	function nameOf(path: string): { dir: string; name: string } {
		const cut = path.lastIndexOf('/');
		return cut === -1 ? { dir: '', name: path } : { dir: path.slice(0, cut + 1), name: path.slice(cut + 1) };
	}
</script>

<aside class="tree" aria-label="Files at {files.rev}">
	<label class="find">
		<Icon name="search" size={14} weight={2} />
		<input
			type="text"
			aria-label="Find a file"
			placeholder="Find a file"
			spellcheck="false"
			autocomplete="off"
			value={files.finding}
			oninput={(event) => type(event.currentTarget.value)}
		/>
	</label>

	<div class="rows">
		{#if files.found}
			{#each files.found.paths as path (path)}
				{@const parts = nameOf(path)}
				<button class="row found" class:here={path === here} title={path} onclick={() => files.openFile(path)}>
					<span class="name">{parts.name}</span>
					<span class="dir note">{parts.dir}</span>
				</button>
			{:else}
				<span class="note said">No file by that name</span>
			{/each}
			{#if files.found.more}<span class="note said">More match. Type more to narrow them.</span>{/if}
		{:else if root === 'loading' || root === undefined}
			<Loader size="inline" label="Reading…" />
		{:else if !Array.isArray(root)}
			<span class="note said error">{root.error}</span>
		{:else}
			{#each rows as { entry, depth } (entry.path)}
				{@const opened = entry.kind === 'folder' && files.open.includes(entry.path)}
				<button
					class="row"
					class:folder={entry.kind === 'folder'}
					class:here={entry.path === here}
					data-path={entry.path}
					style:padding-left="{6 + depth * 14}px"
					aria-expanded={entry.kind === 'folder' ? opened : undefined}
					disabled={entry.kind === 'submodule'}
					title={entry.kind === 'submodule' ? `${entry.path} — a submodule` : entry.path}
					onclick={() => press(entry)}
				>
					<span class="arrow" aria-hidden="true"
						>{entry.kind === 'folder' ? (opened ? '▾' : '▸') : ''}</span
					>
					<span class="name">{entry.name}</span>
					{#if entry.kind === 'folder' && opened && files.folder(entry.path) === 'loading'}
						<Loader size="inline" label="" />
					{/if}
				</button>
			{/each}
		{/if}
	</div>
</aside>

<style>
	.tree {
		flex: 1 1 220px;
		max-width: 250px;
		min-width: 180px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.find {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		height: 32px;
		padding: 0 10px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-button);
		background: var(--sunken);
		color: var(--muted);
	}

	.find:focus-within {
		border-color: var(--accent);
	}

	.find input {
		flex: 1;
		min-width: 0;
		padding: 0;
		border: none;
		background: none;
		box-shadow: none;
		outline: none;
		font-size: var(--fs-secondary);
	}

	.rows {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding-top: 4px;
	}

	.row {
		flex: none;
		display: flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 8px 0 6px;
		border-radius: var(--r-field);
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		text-align: left;
		white-space: nowrap;
	}

	.row:hover:not(:disabled) {
		background: var(--hover);
	}

	.row.folder .name {
		color: var(--muted);
	}

	/* The file that is open: the same tint the rail gives where you are. */
	.row.here {
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
		font-weight: 600;
	}

	.row:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.arrow {
		flex: none;
		width: 12px;
		color: var(--muted);
	}

	.name {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.found {
		justify-content: space-between;
	}

	.found .name {
		flex: none;
		max-width: 70%;
	}

	.dir {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		direction: rtl;
		font-size: var(--fs-mono);
	}

	.said {
		padding: 6px;
		font-size: var(--fs-mono);
	}

	.error {
		color: var(--danger);
	}
</style>
