<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import * as api from '$lib/api';
	import { mod } from '$lib/platform';
	import Icon from '$lib/ui/Icon.svelte';
	import { speedSearch } from '$lib/ui/speedsearch.svelte';
	import FileGlyph from './FileGlyph.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { notice } from '$lib/ui/notice.svelte';
	import type { FilesEntry, FilesHit } from '$lib/types';
	import { optionKey } from './keys';
	import SearchToggles from './SearchToggles.svelte';
	import { files } from './store.svelte';
	import UsageList from './UsageList.svelte';

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

	/**
	 * Wait for a pause in the typing: every letter is a walk of the tree, and
	 * a search of the text reads every file in it, so it waits longer.
	 */
	function type(text: string) {
		if (timer) clearTimeout(timer);
		timer =
			files.lookFor === 'text'
				? setTimeout(() => void files.searchFiles(text), 350)
				: setTimeout(() => void files.findFiles(text), 140);
	}

	function fieldKey(event: KeyboardEvent) {
		if (files.lookFor !== 'text') return;
		const option = optionKey(event);
		if (option) {
			event.preventDefault();
			files.toggleOption(option);
		} else if (event.key === 'Enter') {
			if (timer) clearTimeout(timer);
			void files.searchFiles(event.currentTarget instanceof HTMLInputElement ? event.currentTarget.value : '');
		}
	}

	/** The lines found, by file, in the order they came. */
	const groups = $derived.by(() => {
		const out: { path: string; hits: FilesHit[] }[] = [];
		for (const hit of files.searched?.hits ?? []) {
			const last = out.at(-1);
			if (last?.path === hit.path) last.hits.push(hit);
			else out.push({ path: hit.path, hits: [hit] });
		}
		return out;
	});

	function press(entry: FilesEntry) {
		if (entry.kind === 'folder') files.toggleFolder(entry.path);
		else if (entry.kind !== 'submodule') void files.openFile(entry.path);
	}

	function nameOf(path: string): { dir: string; name: string } {
		const cut = path.lastIndexOf('/');
		return cut === -1 ? { dir: '', name: path } : { dir: path.slice(0, cut + 1), name: path.slice(cut + 1) };
	}

	/**
	 * Right-click a row (FEAT-152): its name, its path from the top of the
	 * repository, or a file's text at the revision, onto the clipboard.
	 */
	let menu = $state<{ x: number; y: number; anchor: HTMLElement; path: string; folder: boolean } | null>(null);

	function context(event: MouseEvent, path: string, folder = false) {
		event.preventDefault();
		menu = { x: event.clientX, y: event.clientY, anchor: event.currentTarget as HTMLElement, path, folder };
	}

	async function copy(what: string, shown: string, text: () => string | Promise<string>) {
		try {
			await navigator.clipboard.writeText(await text());
			notice.ok(`Copied the ${what}`, shown);
		} catch (error) {
			notice.failed(`Could not copy the ${what}`, error);
		}
	}

	/** The tree holds no text, so the file is read, at the revision the tree shows. */
	async function textOf(path: string): Promise<string> {
		const read = await api.filesRead(files.resolved?.id ?? files.rev, path);
		if (read.content.kind !== 'text') throw new Error(`${path} is not text`);
		return read.content.text;
	}

	const items = $derived.by((): MenuItem[] => {
		if (!menu) return [];
		const { path, folder } = menu;
		const name = nameOf(path).name;
		const out: MenuItem[] = [
			{
				id: 'copy-name',
				label: folder ? 'Copy folder name' : 'Copy file name',
				note: name,
				run: () => void copy('name', name, () => name)
			},
			{
				id: 'copy-path',
				label: 'Copy relative path',
				note: path,
				run: () => void copy('path', path, () => path)
			}
		];
		if (!folder) {
			out.push({
				id: 'copy-content',
				label: 'Copy file content',
				run: () => void copy('content', path, () => textOf(path))
			});
		}
		return out;
	});
</script>

<!-- Typing anywhere on the screen types into the find field (FEAT-158). -->
<aside
	class="tree"
	class:wide={files.lookFor !== 'names'}
	aria-label="Files at {files.rev}"
	use:speedSearch={{ field: () => document.querySelector<HTMLInputElement>('.tree .find input') }}
>
	<div class="look" role="group" aria-label="Look for">
		<button
			type="button"
			class:on={files.lookFor === 'names'}
			aria-pressed={files.lookFor === 'names'}
			onclick={() => files.setLookFor('names')}>Names</button
		>
		<button
			type="button"
			class:on={files.lookFor === 'text'}
			aria-pressed={files.lookFor === 'text'}
			title="Search the text of every file at this revision"
			onclick={() => files.setLookFor('text')}>Text</button
		>
		<button
			type="button"
			class:on={files.lookFor === 'usages'}
			aria-pressed={files.lookFor === 'usages'}
			title="Where a name is defined and used. {mod()}click a name in the file"
			onclick={() => files.setLookFor('usages')}>Usages</button
		>
	</div>
	{#if files.lookFor !== 'usages'}
		<label class="find">
			<Icon name="search" size={14} weight={2} />
			{#if files.lookFor === 'text'}
				<input
					type="text"
					aria-label="Search in files"
					placeholder="Search in files"
					spellcheck="false"
					autocomplete="off"
					value={files.seeking}
					oninput={(event) => type(event.currentTarget.value)}
					onkeydown={fieldKey}
				/>
				<SearchToggles />
			{:else}
				<input
					type="text"
					aria-label="Find a file"
					placeholder="Find a file"
					spellcheck="false"
					autocomplete="off"
					value={files.finding}
					oninput={(event) => type(event.currentTarget.value)}
				/>
			{/if}
		</label>
	{/if}

	<div class="rows">
		{#if files.lookFor === 'usages'}
			<UsageList oncontext={(event, path) => context(event, path)} />
		{:else if files.lookFor === 'text'}
			{#if files.searchedError}
				<span class="note said error" role="alert">{files.searchedError}</span>
			{:else if files.seekingNow && !files.searched}
				<Loader size="inline" label="Searching…" />
			{:else if files.searched}
				<span class="note said" aria-live="polite">
					{files.searched.hits.length}
					{files.searched.hits.length === 1 ? 'line' : 'lines'} in {files.searched.files}
					{files.searched.files === 1 ? 'file' : 'files'}{#if files.searched.more}, and more: type more to narrow them{/if}
				</span>
				{#each groups as group (group.path)}
					{@const parts = nameOf(group.path)}
					<div
						class="row found file"
						title={group.path}
						role="presentation"
						oncontextmenu={(event) => context(event, group.path)}
					>
						<FileGlyph path={group.path} />
						<span class="name">{parts.name}</span>
						<span class="dir note">{parts.dir}</span>
						<span class="count note">{group.hits.length}</span>
					</div>
					{#each group.hits as hit (hit.line)}
						<button
							class="row hit mono"
							class:here={group.path === here && files.match?.line === hit.line - 1}
							title="{group.path}:{hit.line}"
							onclick={() => files.openHit(hit)}
						>
							<span class="at note">{hit.line}</span>
							<span class="text"
								>{hit.text.slice(0, hit.start)}<mark>{hit.text.slice(hit.start, hit.end)}</mark>{hit.text.slice(hit.end)}</span
							>
						</button>
					{/each}
				{/each}
			{/if}
		{:else if files.found}
			{#each files.found.paths as path (path)}
				{@const parts = nameOf(path)}
				<button
					class="row found"
					class:here={path === here}
					title={path}
					onclick={() => files.openFile(path)}
					oncontextmenu={(event) => context(event, path)}
				>
					<FileGlyph {path} />
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
					oncontextmenu={(event) => context(event, entry.path, entry.kind !== 'file')}
				>
					<span class="arrow" aria-hidden="true"
						>{entry.kind === 'folder' ? (opened ? '▾' : '▸') : ''}</span
					>
					<FileGlyph path={entry.path} folder={entry.kind === 'folder' || entry.kind === 'submodule'} open={opened} />
					<span class="name">{entry.name}</span>
					{#if entry.kind === 'folder' && opened && files.folder(entry.path) === 'loading'}
						<Loader size="inline" label="" />
					{/if}
				</button>
			{/each}
		{/if}
	</div>
</aside>

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		anchor={menu.anchor}
		{items}
		label="Copy"
		onclose={() => (menu = null)}
	/>
{/if}

<style>
	/* As wide as it was dragged (FEAT-152); the file gives way. */
	.tree {
		flex: none;
		width: var(--files-tree-w);
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

	.found .name {
		flex: none;
		max-width: 64%;
	}

	.found .dir {
		margin-left: auto;
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

	/* Lines of text need more room than names do. */
	.tree.wide {
		width: max(var(--files-tree-w), 340px);
	}

	.look {
		flex: none;
		display: flex;
		gap: 2px;
		padding: 2px;
		border-radius: var(--r-button);
		background: var(--sunken);
	}

	.look button {
		flex: 1;
		height: 22px;
		border-radius: var(--r-field);
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.look button.on {
		background: var(--surface);
		color: var(--ink);
		font-weight: 600;
	}

	.row.file {
		margin-top: 6px;
	}

	.row.file .count {
		flex: none;
		margin-left: 6px;
	}

	.row.hit {
		gap: 8px;
		padding-left: 12px;
	}

	.hit .at {
		flex: none;
		min-width: 3ch;
		text-align: right;
	}

	.hit .text {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: pre;
	}

	.hit mark {
		border-radius: 3px;
		background: color-mix(in srgb, var(--accent) 28%, transparent);
		color: inherit;
	}
</style>
