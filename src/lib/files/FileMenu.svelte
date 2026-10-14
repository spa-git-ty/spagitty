<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { openInFiles } from './open';
	import { revisionLabel } from './store.svelte';

	/**
	 * Right-click any file, on any screen, and open it in Files.
	 *
	 * One listener on the window, not one per list: a row only has to say it
	 * is a file (`data-file-path`, see `open.ts`). A row with a menu of its
	 * own handles the right-click first and this one leaves it alone.
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

<svelte:window {oncontextmenu} />

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
