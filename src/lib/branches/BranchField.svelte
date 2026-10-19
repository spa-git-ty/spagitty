<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import BranchMenu from './BranchMenu.svelte';
	import type { BranchOption } from './pick';

	/**
	 * A form's branch field, in a dialog (TASK-069): it shows the branch chosen
	 * and opens the list every other branch is chosen from (FEAT-122), which can
	 * be searched. It replaced a native `<select>` of every ref in one run.
	 */
	interface Props {
		/** For the form's `<label for>`. */
		id: string;
		value: string;
		options: BranchOption[];
		/** The menu's name, for screen readers. */
		label: string;
		/** What the field says while nothing is chosen. */
		placeholder?: string;
		/** The branch checked out here. Marked in the list. */
		current?: string | null;
		disabled?: boolean;
		onpick: (name: string) => void;
	}

	let {
		id,
		value,
		options,
		label,
		placeholder = 'Choose a branch',
		current = null,
		disabled = false,
		onpick
	}: Props = $props();

	let open = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	function toggle(event: MouseEvent) {
		if (open) {
			open = null;
			return;
		}
		const field = event.currentTarget as HTMLElement;
		const box = field.getBoundingClientRect();
		open = { x: box.left, y: box.bottom + 4, anchor: field };
	}
</script>

<button
	{id}
	type="button"
	class="branch-field mono"
	class:empty={value === ''}
	aria-haspopup="listbox"
	aria-expanded={open !== null}
	{disabled}
	onclick={toggle}
>
	<span class="name">{value === '' ? placeholder : value}</span>
	<Icon name="chevron-down" size="0.85em" weight={2} />
</button>

{#if open}
	<BranchMenu
		x={open.x}
		y={open.y}
		anchor={open.anchor}
		layer="dialog"
		{label}
		{options}
		value={value === '' ? null : value}
		{current}
		{onpick}
		onclose={() => (open = null)}
	/>
{/if}

<style>
	.branch-field {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		min-width: 0;
		width: 100%;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 4px;
		padding: 6px 10px;
		font-size: var(--fs-secondary);
		color: var(--ink);
		cursor: pointer;
		text-align: left;
	}

	.branch-field:focus-visible,
	.branch-field[aria-expanded='true'] {
		border-color: var(--accent);
		outline: none;
	}

	.branch-field:disabled {
		cursor: default;
		opacity: 0.6;
	}

	.branch-field.empty .name {
		color: var(--muted);
	}

	.branch-field :global(svg) {
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
