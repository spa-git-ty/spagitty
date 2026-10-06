<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { delight } from '$lib/delight/store.svelte';
	import { resolving } from '$lib/merger/resolve.svelte';
	import { isItemActive, navRows } from '$lib/nav';
	import { repo } from '$lib/repo.svelte';
	import { review } from '$lib/review/store.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import type { NavItem } from '$lib/nav';

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
		navRows(undefined, {
			pathname: page.url.pathname,
			conflicts: counts.conflicts,
			delight: delight.on
		})
	);

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
		// Review has no count to show: its dot is a pull request waiting on
		// your review (FEAT-087).
		if (item.code === '1R') return review.waiting;
		// Merger's dot is a merge started there with conflicts still to
		// resolve (FEAT-102). Conflicts keeps its own for what git stopped on.
		if (item.code === '1S') return resolving.unresolved > 0;
		const key = item.count;
		if (key !== 'working' && key !== 'conflicts') return false;
		return (counts[key] ?? 0) > 0;
	}
</script>

<div class="slot">
	<nav class="rail ornament" aria-label="Screens">
		<!--
			Opening a repository is the first thing a new user needs, so with
			nothing open it takes the top of the rail, filled with the accent. It
			goes once a repository is open: the tab strip's `+` and Ctrl+O still
			open one, and a bright button offering to replace the repository you
			are working in is the opposite of what the rail is for.
		-->
		{#if !repo.info}
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

		{#each rows as row (row.item.href)}
			{#if row.startsGroup && row.heading}
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
		{/each}
	</nav>
</div>

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

	.rail:hover,
	.rail:focus-within {
		width: var(--rail-open);
		transition-delay: 160ms;
	}

	.rail:not(:hover):not(:focus-within) {
		transition-delay: 0ms;
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

	.rail:hover .label,
	.rail:hover .count,
	.rail:focus-within .label,
	.rail:focus-within .count {
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
