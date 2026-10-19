<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { openInFiles, targetOf } from './open';
	import { revisionLabel } from './store.svelte';

	/**
	 * Right-click any file, on any screen, and open it in Files.
	 *
	 * One listener on the window, not one per list: a row only has to say it
	 * is a file (`data-file-path`, see `open.ts`). A row with a menu of its
	 * own handles the right-click first and this one leaves it alone.
	 *
	 * Opening does not need the menu (FEAT-150): a double-click, or a click
	 * with Ctrl or ⌘, opens the file from any row, and a row marked
	 * `data-file-open="click"` opens on a plain click.
	 */
	let menu = $state<{
		x: number;
		y: number;
		anchor: HTMLElement;
		path: string;
		rev: string | undefined;
		absent: string | undefined;
	} | null>(null);

	function oncontextmenu(event: MouseEvent) {
		if (event.defaultPrevented) return;
		const row = (event.target as Element | null)?.closest?.<HTMLElement>('[data-file-path]');
		const path = row?.dataset.filePath;
		if (!row || !path) return;
		event.preventDefault();
		menu = {
			x: event.clientX,
			y: event.clientY,
			anchor: row,
			path,
			rev: row.dataset.fileRev || undefined,
			absent: row.dataset.fileAbsent || undefined
		};
	}

	function rowOf(event: MouseEvent): HTMLElement | null {
		const target = event.target as Element | null;
		// Typing in a field is not asking for a file.
		if (target?.closest?.('input, textarea, select, [contenteditable="true"]')) return null;
		return target?.closest?.<HTMLElement>('[data-file-path]') ?? null;
	}

	function onclick(event: MouseEvent) {
		if (event.defaultPrevented || event.button !== 0) return;
		const row = rowOf(event);
		if (!row) return;
		const modified = event.ctrlKey || event.metaKey;
		if (!modified && row.dataset.fileOpen !== 'click') return;
		// A control inside a plain-click row keeps its own click.
		const inner = (event.target as Element).closest('button, a, [role="button"]');
		if (!modified && inner && inner !== row && row.contains(inner)) return;
		const target = targetOf(row);
		if (target) void openInFiles(target);
	}

	function ondblclick(event: MouseEvent) {
		if (event.defaultPrevented) return;
		const row = rowOf(event);
		const target = row ? targetOf(row) : null;
		if (!target) return;
		event.preventDefault();
		void openInFiles(target);
	}

	const items = $derived.by((): MenuItem[] => {
		if (!menu) return [];
		const { path, rev, absent } = menu;
		return [
			{
				id: 'open-in-files',
				label: 'Open in Files',
				note: rev ? revisionLabel(rev) : undefined,
				disabled: !!absent,
				reason: absent,
				run: () => void openInFiles({ path, rev })
			}
		];
	});
</script>

<svelte:window {oncontextmenu} {onclick} {ondblclick} />

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		anchor={menu.anchor}
		{items}
		label="File actions"
		onclose={() => (menu = null)}
	/>
{/if}
