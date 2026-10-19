<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { delight } from '$lib/delight/store.svelte';
	import { resolving } from '$lib/merger/resolve.svelte';
	import { isItemActive, NAV_ITEMS, railRows } from '$lib/nav';
	import { repo } from '$lib/repo.svelte';
	import { review } from '$lib/review/store.svelte';
	import { agents } from '$lib/agents/store.svelte';
	import { farmStore } from '$lib/farm/store.svelte';
	import { settings } from '$lib/settings/store.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import type { NavItem, RailSettings } from '$lib/nav';

	interface Props {
		/**
		 * What to draw instead of the stored choice (FEAT-134).
		 *
		 * Settings › Rail shows this component, not a drawing of it, so what a
		 * person sees there while choosing is what the rail will be.
		 */
		rail?: RailSettings;
		/**
		 * Drawn to be looked at: in the flow of the page, closed, and inert.
		 * A preview that navigated would take the person off the screen they
		 * are previewing from.
		 */
		preview?: boolean;
	}

	let { rail, preview = false }: Props = $props();

	/**
	 * The rail is an ornament (FEAT-082): a floating pill of icons beside the
	 * pane, which shows its labels and counts when it is hovered or focused.
	 *
	 * It was a sidebar — a full-height strip with a collapse button and a
	 * splitter — and both of those existed to answer "how much room should the
	 * rail take". An ornament takes the room of its icons and borrows more,
	 * over the pane, only while somebody is reading it. There is nothing to
	 * collapse and nothing to drag.
	 */
	const counts = $derived(repo.counts);

	/**
	 * The rows that are on the rail right now (TASK-045): the everyday five and
	 * Settings, Conflicts while there is something to resolve, and whichever
	 * other screen is open, for as long as it is.
	 */
	const rows = $derived(
		railRows(NAV_ITEMS, {
			pathname: page.url.pathname,
			conflicts: counts.conflicts,
			delight: delight.on,
			rail: rail ?? settings.rail
		})
	);

	/**
	 * The More button's menu (FEAT-134): the screens the person hid, each with
	 * its code. Opened beside the button rather than at the pointer, so it is
	 * in the same place from the keyboard.
	 */
	let more = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	function toggleMore(event: MouseEvent) {
		const anchor = event.currentTarget as HTMLElement;
		if (more) {
			more = null;
			return;
		}
		const box = anchor.getBoundingClientRect();
		more = { x: box.right + 8, y: box.top, anchor };
	}

	const hidden = $derived(rows.flatMap((row) => (row.kind === 'more' ? row.items : [])));

	function under(items: NavItem[]): MenuItem[] {
		return items.map((item) => ({
			id: item.code,
			label: item.label,
			note: item.code,
			run: () => void goto(item.href)
		}));
	}

	/**
	 * A count of `null` means "not computed yet" and renders as a dot. Only the
	 * screens that exist report real numbers; inventing the rest would make the
	 * rail lie about how much work is waiting.
	 */
	function countLabel(key: keyof typeof counts): string {
		const value = counts[key];
		return value === null ? '·' : String(value);
	}

	/**
	 * Work waiting, as a dot on the icon while the labels are hidden. Only the
	 * counts that ask for something — changed files, conflicts — get one; how
	 * many commits or branches there are is inventory, and a dot on every icon
	 * would say nothing.
	 */
	function waiting(item: NavItem): boolean {
		// Farm's dot: a task waits on you, in this repository's farm (FEAT-109).
		if (item.code === '1Q') {
			return farmStore.path === repo.info?.path && farmStore.needsYou.length > 0;
		}
		// Review has no count to show: its dot is a pull request waiting on
		// your review (FEAT-087).
		// An agent waiting for you there draws the same dot (2.0).
		if (item.code === '1R') return review.waiting || agents.waiting('review');
		// Merger's dot is a merge started there with conflicts still to
		// resolve (FEAT-102). Conflicts keeps its own for what git stopped on.
		if (item.code === '1S') return resolving.unresolved > 0 || agents.waiting('merge');
		const key = item.count;
		if (key !== 'working' && key !== 'conflicts') return false;
		return (counts[key] ?? 0) > 0;
	}
</script>

<div class="slot" class:preview>
	<nav class="rail ornament" aria-label={preview ? 'Preview of the rail' : 'Screens'} inert={preview}>
		<!--
			Opening a repository is the first thing a new user needs, so with
			nothing open it takes the top of the rail, filled with the accent. It
			goes once a repository is open: the tab strip's `+` and Ctrl+O still
			open one, and a bright button offering to replace the repository you
			are working in is the opposite of what the rail is for.
		-->
		{#if !repo.info && !preview}
			<div class="open">
				<button
					class="item open-repository primary"
					title="Open repository…"
					aria-label="Open repository…"
					onclick={() => repo.choose()}
				>
					<span class="glyph"><Icon name="folder" size="1.2em" /></span>
					<span class="label">Open repository…</span>
				</button>
			</div>
		{/if}

		{#each rows as row (row.kind === 'more' ? 'more' : row.item.href)}
			{#if row.kind === 'more'}
				{#if row.startsGroup}
					<div class="hr" aria-hidden="true"></div>
				{/if}
				<button
					class="item more"
					title="More"
					aria-label="More"
					aria-haspopup="menu"
					aria-expanded={more !== null}
					onclick={toggleMore}
				>
					<span class="glyph"><Icon name="more" size="1.2em" /></span>
					<span class="label">More</span>
				</button>
			{:else}
				{#if row.startsGroup && (row.heading || row.item.dividerBefore)}
					<div class="hr" aria-hidden="true"></div>
				{/if}
				<button
					class="item"
					data-active={isItemActive(row.item, page.url.pathname)}
					title={row.item.label}
					aria-label={row.item.label}
					onclick={() => goto(row.item.href)}
				>
					<span class="glyph">
						<Icon name={row.item.icon} size="1.2em" />
						{#if waiting(row.item)}<span class="dot" aria-hidden="true"></span>{/if}
					</span>
					<span class="label">{row.item.label}</span>
					<span class="count mono">
						{row.item.count ? countLabel(row.item.count) : ''}
					</span>
				</button>
			{/if}
		{/each}
	</nav>
</div>

<!--
	Outside the pill: an ornament's blur makes it the containing block of
	anything `position: fixed` inside it, and the pill clips what overflows it,
	so a menu mounted in there would be drawn inside the rail and cut off.
-->
{#if more && hidden.length > 0}
	<Menu
		x={more.x}
		y={more.y}
		anchor={more.anchor}
		label="More screens"
		items={under(hidden)}
		onclose={() => (more = null)}
	/>
{/if}

<style>
	/*
	 * The room the rail takes in the stage: its closed width, always. The rail
	 * itself is positioned inside it, so opening it widens the ornament over
	 * the pane instead of pushing the pane over.
	 */
	.slot {
		--rail-closed: 52px;
		--rail-open: 216px;
		flex: none;
		width: var(--rail-closed);
		position: relative;
		z-index: 3;
	}

	/*
	 * The pill. It sizes to its rows rather than filling the height: an
	 * ornament is an object beside the pane, not a column of the window.
	 *
	 * Opening waits a moment and closing does not. A pointer on its way from
	 * the tab strip to the pane crosses the rail, and a rail that sprang open
	 * under every crossing would be the loudest thing in the window.
	 */
	.rail {
		position: absolute;
		top: 0;
		left: 0;
		width: var(--rail-closed);
		max-height: 100%;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 6px;
		border-radius: var(--r-ornament);
		overflow: hidden auto;
		scrollbar-width: none;
		transition: width var(--t-slow) var(--ease);
	}

	.slot:not(.preview) .rail:hover,
	.slot:not(.preview) .rail:focus-within {
		width: var(--rail-open);
		transition-delay: 160ms;
	}

	.rail:not(:hover):not(:focus-within) {
		transition-delay: 0ms;
	}

	/*
	 * The preview in Settings › Rail (FEAT-134): the same pill, standing in
	 * the page instead of floating beside the pane, and closed — it is there
	 * to show which rows there are, and its labels are in the list beside it.
	 */
	.preview {
		z-index: auto;
	}

	.preview .rail {
		position: static;
		max-height: none;
	}

	.item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		height: 40px;
		padding: 0;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		text-align: left;
		color: var(--ink);
		white-space: nowrap;
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	/* Hover changes the colour and nothing else (TASK-042): no target moves
	   out from under a pointer on its way to it. */
	.item:hover {
		background: var(--hover);
	}

	.item:active {
		background: var(--press);
	}

	/*
	 * The active row is an accent-tinted capsule. The icon is what shows while
	 * the rail is closed, so it carries the accent too.
	 */
	.item[data-active='true'] {
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
		font-weight: 600;
	}

	/* A square the width of the closed rail's inside, so the icon sits in the
	   same place open or closed. */
	.glyph {
		position: relative;
		flex: none;
		width: 40px;
		height: 40px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	.dot {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--accent);
		box-shadow: 0 0 0 2px var(--glass-thick);
	}

	/*
	 * The label and the count are always in the row — for a screen reader,
	 * and so nothing reflows when the rail opens — and only drawn while it is
	 * open.
	 */
	.label,
	.count {
		opacity: 0;
		transition: opacity var(--t-fast) var(--ease);
	}

	.label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.slot:not(.preview) .rail:hover .label,
	.slot:not(.preview) .rail:hover .count,
	.slot:not(.preview) .rail:focus-within .label,
	.slot:not(.preview) .rail:focus-within .count {
		opacity: 1;
		transition-delay: 160ms;
	}

	.count {
		flex: none;
		min-width: 22px;
		margin-right: 8px;
		padding: 0 7px;
		border-radius: var(--r-pill);
		text-align: center;
		color: var(--muted);
		background: var(--soft);
		font-variant-numeric: tabular-nums;
	}

	.count:empty {
		display: none;
	}

	.item[data-active='true'] .count {
		color: var(--accent);
		background: color-mix(in srgb, var(--accent) 18%, transparent);
	}

	.hr {
		margin: 5px 8px;
	}

	.open {
		padding-bottom: 4px;
	}

	.open-repository {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 650;
	}

	.open-repository:hover {
		background: var(--accent-lift);
	}
</style>
