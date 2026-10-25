<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { isMac } from '$lib/platform';
	import { callsIn, filterItems, isFolder, methodColour, methodOf } from './model';
	import type { Collection, Item, Opened, Stored } from './types';

	/**
	 * Collections, on the left of Calls (FEAT-160, proposal §5.1): a filter,
	 * each collection with its file and count, its folders, and its calls as
	 * method and name. A collapsed collection shows its kind and count. A
	 * file that could not be read is listed with why, and cannot be opened.
	 * *History* is at the foot.
	 */

	interface Props {
		collections: Stored<Collection>[];
		filter: string;
		collapsed: string[];
		current: Opened | null;
		today: number;
		historyOpen: boolean;
		onfilter: (filter: string) => void;
		ontoggle: (id: string) => void;
		onopen: (opened: Opened) => void;
		oncreate: (stem: string | null, folder: string | null) => void;
		onduplicate: (opened: Opened) => void;
		onremove: (opened: Opened) => void;
		onhistory: () => void;
		search?: HTMLInputElement | null;
	}

	let {
		collections,
		filter,
		collapsed,
		current,
		today,
		historyOpen,
		onfilter,
		ontoggle,
		onopen,
		oncreate,
		onduplicate,
		onremove,
		onhistory,
		search = $bindable(null)
	}: Props = $props();

	let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
	const searchKeys = isMac() ? '⌘ K' : 'Ctrl K';

	/** What a collection is, for when it is collapsed: REST, SOAP or both. */
	function kindOf(collection: Collection): string {
		const kinds = new Set(callsIn(collection.items).map((call) => call.kind));
		if (kinds.size === 2) return 'REST · SOAP';
		return kinds.has('soap') ? 'SOAP' : 'REST';
	}

	const isOpen = (stem: string, id: string) => current?.stem === stem && current.id === id;

	function callMenu(event: MouseEvent, opened: Opened) {
		event.preventDefault();
		menu = {
			x: event.clientX,
			y: event.clientY,
			items: [
				{ id: 'open', label: 'Open', run: () => onopen(opened) },
				{ id: 'duplicate', label: 'Duplicate', run: () => onduplicate(opened) },
				{ separator: true },
				{ id: 'delete', label: 'Delete', danger: true, run: () => onremove(opened) }
			]
		};
	}

	function folderMenu(event: MouseEvent, stem: string, folder: string | null) {
		event.preventDefault();
		menu = {
			x: event.clientX,
			y: event.clientY,
			items: [{ id: 'new', label: folder ? `New call in ${folder}` : 'New call', run: () => oncreate(stem, folder) }]
		};
	}
</script>

<aside class="tree" aria-label="Collections">
	<label class="filter">
		<Icon name="search" size={14} />
		<input
			bind:this={search}
			type="text"
			placeholder="Filter calls"
			aria-label="Filter calls"
			spellcheck="false"
			value={filter}
			oninput={(event) => onfilter(event.currentTarget.value)}
			onkeydown={(event) => {
				if (event.key === 'Escape' && filter) {
					event.stopPropagation();
					onfilter('');
				}
			}}
		/>
		<kbd class="mono">{searchKeys}</kbd>
	</label>

	<div class="list">
		{#each collections as stored (stored.stem)}
			{@const id = `c:${stored.stem}`}
			{@const shut = collapsed.includes(id) && !filter.trim()}
			{#if stored.value}
				{@const items = filterItems(stored.value.items, filter)}
				{@const count = callsIn(stored.value.items).length}
				{#if !filter.trim() || items.length}
					<section class="collection">
						<button
							type="button"
							class="head"
							aria-expanded={!shut}
							title={stored.file}
							onclick={() => ontoggle(id)}
							oncontextmenu={(event) => folderMenu(event, stored.stem, null)}
						>
							{#if shut}<span class="arrow" aria-hidden="true">▸</span>{/if}
							<span class="name">{stored.value.name}</span>
							<span class="gap"></span>
							{#if shut}
								<span class="kind mono">{kindOf(stored.value)}</span>
							{/if}
							<span class="count">{count} {shut ? '' : count === 1 ? 'call' : 'calls'}</span>
						</button>
						{#if !shut}
							<p class="file mono">{stored.file}</p>
							{@render tree(stored.stem, items, 0)}
							{#if count === 0}
								<button type="button" class="empty" onclick={() => oncreate(stored.stem, null)}>+ New call</button>
							{/if}
						{/if}
					</section>
				{/if}
			{:else}
				<section class="collection broken" title={stored.error ?? ''}>
					<div class="head"><span class="name">{stored.stem}</span></div>
					<p class="file mono">{stored.file}</p>
					<p class="why">Not read: {stored.error}</p>
				</section>
			{/if}
		{:else}
			<div class="none">
				<p>No collections yet. They are kept in this repository, as JSON, so they can be committed.</p>
				<button type="button" class="empty" onclick={() => oncreate(null, null)}>+ New call</button>
			</div>
		{/each}
	</div>

	<button type="button" class="history" class:on={historyOpen} aria-pressed={historyOpen} onclick={onhistory}>
		<Icon name="history" size={15} />
		<span>History</span>
		<span class="gap"></span>
		<span class="count">{today} today</span>
	</button>
</aside>

{#snippet tree(stem: string, items: Item[], depth: number)}
	{#each items as item, index (isFolder(item) ? `f:${item.folder}:${index}` : item.id)}
		{#if isFolder(item)}
			{@const id = `f:${stem}/${item.folder}`}
			{@const shut = collapsed.includes(id) && !filter.trim()}
			<button
				type="button"
				class="folder"
				style:padding-left={`${8 + depth * 14}px`}
				aria-expanded={!shut}
				onclick={() => ontoggle(id)}
				oncontextmenu={(event) => folderMenu(event, stem, item.folder)}
				><span class="arrow" aria-hidden="true">{shut ? '▸' : '▾'}</span>{item.folder}</button
			>
			{#if !shut}
				{@render tree(stem, item.items, depth + 1)}
			{/if}
		{:else}
			{@const method = methodOf(item)}
			<button
				type="button"
				class="call"
				class:on={isOpen(stem, item.id)}
				aria-current={isOpen(stem, item.id) ? 'true' : undefined}
				style:padding-left={`${8 + depth * 14}px`}
				title={item.url}
				onclick={() => onopen({ stem, id: item.id })}
				oncontextmenu={(event) => callMenu(event, { stem, id: item.id })}
				><span class="method mono" style:color={methodColour(method)}>{method}</span><span class="label"
					>{item.name}</span
				></button
			>
		{/if}
	{/each}
{/snippet}

{#if menu}
	<Menu x={menu.x} y={menu.y} label="Call" items={menu.items} onclose={() => (menu = null)} />
{/if}

<style>
	.tree {
		flex: none;
		width: 260px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	.filter {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 8px;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		background: var(--sunken);
		color: var(--muted);
	}

	.filter input {
		flex: 1;
		min-width: 0;
		padding: 2px 0;
		border: 0;
		background: transparent;
		color: var(--ink);
	}

	.filter input:focus {
		outline: none;
	}

	.filter:focus-within {
		border-color: var(--ring);
	}

	kbd {
		padding: 0 5px;
		border: 1px solid var(--soft);
		border-radius: 4px;
		font-size: var(--fs-secondary);
	}

	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.collection {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.head {
		display: flex;
		align-items: baseline;
		gap: 6px;
		padding: 4px 8px;
		border-radius: var(--r-row);
		text-align: left;
	}

	.head:hover,
	.folder:hover,
	.call:hover {
		background: var(--hover);
	}

	.name {
		font-weight: 600;
	}

	.gap {
		flex: 1;
	}

	.count,
	.why {
		color: var(--muted);
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	.kind {
		color: var(--agent);
		font-size: var(--fs-secondary);
	}

	.file {
		margin: 0 0 4px;
		padding: 0 8px;
		overflow: hidden;
		color: var(--muted);
		font-size: var(--fs-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.broken .name {
		color: var(--danger);
	}

	.why {
		margin: 0;
		padding: 0 8px;
		white-space: normal;
	}

	.folder,
	.call {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 8px;
		border-radius: var(--r-row);
		text-align: left;
	}

	.folder {
		color: var(--muted);
	}

	.arrow {
		width: 10px;
		color: var(--muted);
		font-size: 0.8em;
	}

	.call.on {
		background: var(--surface-2);
	}

	.method {
		flex: none;
		width: 52px;
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.none p {
		margin: 4px 8px 8px;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.empty {
		align-self: flex-start;
		margin: 4px 8px;
		padding: 3px 10px;
		border: 1px dashed var(--soft);
		border-radius: var(--r-button);
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.empty:hover {
		color: var(--ink);
	}

	.history {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-top: 1px solid var(--line);
		border-radius: var(--r-row);
		color: var(--muted);
	}

	.history:hover,
	.history.on {
		background: var(--hover);
		color: var(--ink);
	}
</style>
