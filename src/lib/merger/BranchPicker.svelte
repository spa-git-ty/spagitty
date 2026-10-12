<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import BranchMenu from '$lib/branches/BranchMenu.svelte';
	import type { BranchOption } from '$lib/branches/pick';
	import Icon from '$lib/ui/Icon.svelte';
	import type { Pickable } from './store.svelte';

	/**
	 * The name on a branch card, and the list that changes it (FEAT-100): any
	 * local branch, any remote-tracking branch, or a tag as something to merge
	 * from. The other card's branch is listed, with its reason, and cannot be
	 * picked. The list is the one every branch is chosen from (FEAT-122): it
	 * can be searched, and the branch checked out is marked in it.
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

	const choices = $derived(
		options.map(
			(option): BranchOption => ({
				name: option.name,
				group: option.group,
				reason: option.name === other ? `already ${which === 'A' ? 'B' : 'A'}` : null
			})
		)
	);

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
	aria-haspopup="listbox"
	aria-expanded={open !== null}
	aria-label="Branch {which}: {value ?? 'none'}"
	title="Choose branch {which}"
	onclick={toggle}
>
	<span class="name">{value ?? 'Choose a branch'}</span>
	<Icon name="chevron-down" size="0.85em" weight={2} />
</button>

{#if open}
	<BranchMenu
		x={open.x}
		y={open.y}
		anchor={open.anchor}
		label="Branch {which}"
		options={choices}
		{value}
		{current}
		{onpick}
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
