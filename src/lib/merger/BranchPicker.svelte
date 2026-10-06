<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import type { Pickable } from './store.svelte';

	/**
	 * The name on a branch card, and the menu that changes it (FEAT-100): any
	 * local branch, any remote-tracking branch, or a tag as something to merge
	 * from. The other card's branch is listed, disabled, with its reason.
	 */
	interface Props {
		value: string | null;
		/** What the card is: `A` or `B`, for the menu's name. */
		which: string;
		options: Pickable[];
		/** The other card's branch. */
		other: string | null;
		/** The branch checked out here. */
		current: string | null;
		onpick: (name: string) => void;
	}

	let { value, which, options, other, current, onpick }: Props = $props();

	let open = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	/** Past this many tags only the newest are offered; Tags lists them all. */
	const TAGS = 30;

	const GROUPS: { group: Pickable['group']; heading: string }[] = [
		{ group: 'local', heading: 'Branches' },
		{ group: 'remote', heading: 'Remote branches' },
		{ group: 'tag', heading: 'Tags' }
	];

	const items = $derived.by((): MenuItem[] => {
		const list: MenuItem[] = [];
		for (const { group, heading } of GROUPS) {
			const rows = options.filter((option) => option.group === group).slice(0, group === 'tag' ? TAGS : undefined);
			if (rows.length === 0) continue;
			list.push({ heading });
			for (const row of rows) {
				list.push({
					id: `${group}:${row.name}`,
					label: row.name,
					note: row.name === current ? 'checked out' : undefined,
					disabled: row.name === other || row.name === value,
					reason: row.name === other ? `already ${which === 'A' ? 'B' : 'A'}` : undefined,
					run: () => onpick(row.name)
				});
			}
		}
		if (list.length === 0) list.push({ id: 'none', label: 'no branches', disabled: true, run: () => {} });
		return list;
	});

	function toggle(event: MouseEvent) {
		const button = event.currentTarget as HTMLElement;
		if (open) {
			open = null;
			return;
		}
		const box = button.getBoundingClientRect();
		open = { x: box.left, y: box.bottom + 4, anchor: button };
	}
</script>

<button
	class="picker mono"
	aria-haspopup="menu"
	aria-expanded={open !== null}
	aria-label="Branch {which}: {value ?? 'none'}"
	title="Choose branch {which}"
	onclick={toggle}
>
	<span class="name">{value ?? 'Choose a branch'}</span>
	<Icon name="chevron-down" size="0.85em" weight={2} />
</button>

{#if open}
	<Menu
		x={open.x}
		y={open.y}
		anchor={open.anchor}
		label="Branch {which}"
		{items}
		onclose={() => (open = null)}
	/>
{/if}

<style>
	.picker {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 3px 8px;
		border-radius: 8px;
		font-size: var(--fs-ui);
		font-weight: 600;
		color: var(--ink);
	}

	.picker:hover {
		background: var(--hover);
	}

	.picker :global(svg) {
		color: var(--muted);
		flex: none;
	}

	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
