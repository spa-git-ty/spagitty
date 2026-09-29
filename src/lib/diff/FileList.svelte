<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { FileChange } from '$lib/types';
	import FileName from '$lib/ui/FileName.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import * as api from '$lib/api';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * The files a commit touched, with what each one cost in lines.
	 *
	 * A file with no line counts says why — binary, or too large — rather than
	 * showing `+0 −0`, which would read as "nothing changed".
	 *
	 * Given its files rather than reading a store, so the Stash screen can show
	 * the contents of an entry with this component instead of a second one that
	 * drifts from it (FEAT-034). A stash *is* a commit, so the two lists are the
	 * same list.
	 */

	interface Props {
		files: FileChange[];
		/** Path of the selected file, or null when nothing is selected. */
		selected: string | null;
		onselect: (path: string) => void;
		/**
		 * Move `delta` files through the list. Given, the list handles the
		 * arrow keys itself; withheld, it is click-only.
		 */
		onstep?: (delta: number) => void;
		/** What this is a list of. Read out, so it says which screen it is on. */
		label?: string;
		/** Shown when there are no files at all. */
		empty?: string;
	}

	let {
		files,
		selected,
		onselect,
		onstep,
		label = 'Files in this commit',
		empty = 'No file changes.'
	}: Props = $props();

	let menu = $state<{ x: number; y: number; path: string; anchor: HTMLElement } | null>(null);

	function oncontextmenu(event: MouseEvent, filePath: string) {
		event.preventDefault();
		menu = {
			x: event.clientX,
			y: event.clientY,
			path: filePath,
			anchor: event.currentTarget as HTMLElement
		};
	}

	async function openExternal(filePath: string) {
		try {
			await api.launchExternalDiff(filePath);
			notice.ok('Launched external diff tool', filePath);
		} catch (err) {
			notice.failed('Could not launch external tool', err);
		}
	}

	const menuItems = $derived.by((): MenuItem[] => {
		if (!menu) return [];
		const filePath = menu.path;
		return [
			{
				id: 'ext-diff',
				label: 'Open in External Diff Tool',
				run: () => void openExternal(filePath)
			}
		];
	});

	/**
	 * Arrow keys walk the list, Home and End reach its ends.
	 *
	 * On the list rather than the window: a file list is one of several things
	 * on screen that answer to an arrow key, and the one with focus is the one
	 * that should.
	 */
	function onkeydown(event: KeyboardEvent) {
		if (!onstep || event.metaKey || event.ctrlKey || event.altKey) return;

		if (event.key === 'ArrowDown') onstep(1);
		else if (event.key === 'ArrowUp') onstep(-1);
		else if (event.key === 'Home') onstep(-files.length);
		else if (event.key === 'End') onstep(files.length);
		else return;

		event.preventDefault();
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<nav class="files" aria-label={label} {onkeydown}>
	{#each files as file (file.path)}
		<button
			class="file"
			class:selected={file.path === selected}
			onclick={() => onselect(file.path)}
			oncontextmenu={(e) => oncontextmenu(e, file.path)}
			title={file.path}
		>
			<FileName path={file.path} status={file.status} />
			{#if file.binary}
				<span class="mono muted counts">bin</span>
			{:else if file.tooLarge}
				<span class="mono muted counts">big</span>
			{:else}
				<span class="mono counts">
					<span class="plus">+{file.added}</span>
					<span class="minus">−{file.removed}</span>
				</span>
			{/if}
		</button>
	{/each}

	{#if files.length === 0}
		<div class="empty note">{empty}</div>
	{/if}
</nav>

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		anchor={menu.anchor}
		items={menuItems}
		label="File actions"
		onclose={() => (menu = null)}
	/>
{/if}

<style>
	/*
	 * On the pane, not a panel beside it (TASK-046): no fill and no rule, and
	 * each file is a line with a rounded highlight rather than a band.
	 */
	.files {
		width: var(--diff-files-w);
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 1px;
		overflow-y: auto;
		padding: 8px 6px;
	}

	.file {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 34px;
		padding: 0 10px 0 8px;
		border-radius: var(--r-button);
		text-align: left;
		width: 100%;
		min-width: 0;
		flex: none;
		transition: background var(--t-fast) var(--ease);
	}

	.file:hover {
		background: var(--hover);
	}

	.file.selected {
		background: var(--selection);
	}

	.counts {
		flex: none;
		display: flex;
		gap: 4px;
	}

	.plus {
		color: var(--ok);
	}

	.minus {
		color: var(--danger);
	}

	.empty {
		padding: 8px;
	}
</style>
