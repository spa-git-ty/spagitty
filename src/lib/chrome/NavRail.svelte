<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { isActive, navRows } from '$lib/nav';
	import { panels } from '$lib/panels.svelte';
	import { repo } from '$lib/repo.svelte';
	import Icon from '$lib/ui/Icon.svelte';

	/**
	 * Collapsed, the rail is a strip of glyphs. Everything stays where it was —
	 * same items, same order, same routes — so the muscle memory of which
	 * position is which screen survives the collapse.
	 */
	const collapsed = $derived(panels.railCollapsed);

	const counts = $derived(repo.counts);

	/**
	 * A count of `null` means "not computed yet" and renders as a dot. Only the
	 * screens that exist report real numbers; inventing the rest would make the
	 * rail lie about how much work is waiting.
	 */
	function countLabel(key: keyof typeof counts): string {
		const value = counts[key];
		return value === null ? '·' : String(value);
	}

	/*
	 * The foot is gone (it was FEAT-040's).
	 *
	 * It stacked four lines under the screens — the working-copy count, how
	 * fresh the walk and the remote were, tags and submodules — and two of them
	 * repeated counts the rows above already carry as badges. They are one line
	 * in the status strip now, which spans the window and is where an
	 * application says what is true of what it has open. The rail is navigation.
	 */
</script>

<nav class="rail" class:collapsed aria-label="Screens">
	<div class="head">
		<button
			class="collapse"
			title={collapsed ? 'Expand the sidebar' : 'Collapse the sidebar'}
			aria-label={collapsed ? 'Expand the sidebar' : 'Collapse the sidebar'}
			aria-expanded={!collapsed}
			onclick={() => panels.toggleRail()}
		>
			<Icon name={collapsed ? 'chevron-right' : 'chevron-left'} size="1em" />
		</button>
	</div>

	<!--
		Opening a repository is the first thing a new user needs and the least
		discoverable place in the rail is below a spacer, which is where it used
		to sit. It takes the top slot instead — the one the log filter had, which
		only duplicated the Log screen's own query bar and the Ctrl+F shortcut.

		And it goes away once there is a repository open, because at that point
		it is the loudest thing in the rail and it is offering to do something
		nobody wants: a filled accent button, above every screen, whose job is
		to replace the repository the person is working in. It is not lost —
		the tab strip's `+`, the repository menu and Ctrl+O all still open one,
		and All repositories is a row down the same rail. The rail's brightest
		pixel now belongs to whichever screen you are on.
	-->
	{#if !repo.info}
		<div class="open">
			<button
				class="item open-repository primary"
				title="Open repository…"
				aria-label="Open repository…"
				onclick={() => repo.choose()}
			>
				<!--
					The icon and the label in one `.name`, exactly as every other
					row builds itself.

					They were direct children, and `.item` lays its children out
					with `space-between` so that a screen's count can sit at the
					far right. With nothing to push apart but an icon and a word,
					that put the icon against the left edge and the text against
					the right, with the whole rail's width between them. Grouping
					them is what every other row already does, and it is why every
					other row reads.
				-->
				<span class="name">
					<Icon name="folder" size="1.2em" />
					{#if !collapsed}<span class="responsive-label">Open repository…</span>{/if}
				</span>
			</button>
		</div>
	{/if}

	<!--
		Three groups rather than fourteen equal rows (TASK-041).

		Expanded, each group after the first carries a heading; collapsed, the
		same boundary is a divider, because a heading needs a word and 48px of
		rail does not have room for one. The order of the rows is unchanged, so
		nothing anybody's hand has learned has moved.
	-->
	{#each navRows() as row (row.item.href)}
		{#if row.startsGroup && row.heading}
			{#if collapsed}
				<div class="hr"></div>
			{:else}
				<h2 class="group">{row.heading}</h2>
			{/if}
		{/if}
		<button
			class="item"
			data-active={isActive(row.item.href, page.url.pathname)}
			title={row.item.label}
			aria-label={row.item.label}
			onclick={() => goto(row.item.href)}
		>
			{#if collapsed}
				<Icon name={row.item.icon} size="1.2em" />
			{:else}
				<span class="name">
					<Icon name={row.item.icon} size="1.15em" />
					<span>{row.item.label}</span>
				</span>
				<span class="count mono">
					{row.item.count ? countLabel(row.item.count) : ''}
				</span>
			{/if}
		</button>
	{/each}

	<div class="spacer"></div>
</nav>

<style>
	/*
	 * The rail is chrome, so it takes the chrome gradient — but vertically,
	 * falling away from the screen it sits beside, and it casts a short shadow
	 * over that screen. That shadow is the whole reason the rail reads as a
	 * *sidebar* rather than as the left-hand third of one flat window.
	 */
	.rail {
		width: var(--rail-w);
		flex: none;
		display: flex;
		flex-direction: column;
		background-color: var(--chrome-veil);
		border-right: 1px solid color-mix(in srgb, var(--line) 60%, transparent);
		box-shadow: none;
		position: relative;
		z-index: 1;
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		padding: 8px;
		border-bottom: 1px solid var(--soft);
	}

	.rail.collapsed .head {
		justify-content: center;
		padding: 8px 4px;
	}

	.collapse {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--muted);
		font-size: var(--fs-secondary);
		line-height: 1;
		padding: 4px;
		border-radius: var(--r-field);
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease),
			transform var(--t-fast) var(--spring);
	}

	.collapse:active {
		transform: scale(0.9);
	}

	.collapse:hover {
		color: var(--accent);
		background: var(--accent-soft);
	}

	/* Collapsed, an item is a glyph in a square: same order, same routes, no
	   labels. The title attribute carries the name for a pointer, and
	   `aria-label` carries it for everything else. */
	.rail.collapsed .item {
		justify-content: center;
		padding-left: 0;
		padding-right: 0;
		width: calc(100% - 8px);
		margin-inline: 4px;
	}

	.rail.collapsed .open {
		padding: 8px 4px;
	}

	/* Collapsed, the group holds one glyph and must not claim the row's width,
	   or the icon centres inside a stretched box instead of inside the pill. */
	.rail.collapsed .open-repository .name {
		justify-content: center;
		width: 100%;
	}

	/* Label and icon travel together; the count is what the row's spare width
	   belongs to. */
	.name {
		display: flex;
		align-items: center;
		gap: 9px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/*
	 * The count is a number in a soft capsule rather than loose grey text, so a
	 * rail of twelve items reads as twelve rows with badges instead of
	 * twenty-four pieces of text.
	 */
	.count {
		flex: none;
		min-width: 20px;
		padding: 0 6px;
		border-radius: var(--r-pill);
		text-align: center;
		color: var(--muted);
		background: var(--soft);
		font-variant-numeric: tabular-nums;
	}

	.item[data-active='true'] .count {
		color: var(--accent);
		background: color-mix(in srgb, var(--accent) 18%, transparent);
	}

	/* The primary action, so it gets the width of the rail rather than sitting
	   in it at its own size. */
	.open {
		padding: 8px;
	}

	/* With a repository open there is no `.open` block, so the first screen
	   would butt straight up against the head's rule. It takes the padding the
	   button used to provide. */
	.head + .item {
		margin-top: 7px;
	}

	.open :global(.btn) {
		width: 100%;
		justify-content: center;
	}

	/*
	 * The one item that is a button rather than a destination.
	 *
	 * No `justify-content` of its own: it inherits `.item`'s, and its icon and
	 * label are grouped in a `.name` like every other row's, so the group sits
	 * at the leading edge and the collapsed rule still centres it.
	 */
	.open-repository {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 650;
	}

	.open-repository:hover {
		background: var(--accent-lift);
		color: var(--on-accent);
		transform: none;
	}

	/*
	 * A group's name (TASK-041).
	 *
	 * Quiet on purpose. It is a label for the run of rows below it, not a row
	 * of its own, so it sits at the secondary size in the muted colour with a
	 * wide letter-spacing — the treatment that reads as "this is a heading"
	 * without reading as "this is something to click". A heading as loud as the
	 * items under it would have made the rail busier rather than clearer, which
	 * is the opposite of the point.
	 *
	 * `h2` rather than a `div`: a screen reader user navigating by heading gets
	 * the same structure the eye does, and the rail already carries an
	 * `aria-label` naming it.
	 */
	.group {
		margin: 12px 6px 3px;
		padding: 0 10px;
		font-size: var(--fs-mono);
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--muted);
		user-select: none;
	}

	/* The first heading has the head's rule above it already. */
	.head + .group,
	.open + .group {
		margin-top: 7px;
	}

	/* Collapsed, the boundary is the divider instead — a heading needs a word
	   and 48px of rail has no room for one. */
	.rail.collapsed .hr {
		margin: 7px 10px;
	}

	/*
	 * An item is a pill, not a full-width strip with a bar stuck on its left.
	 *
	 * The strip ran edge to edge, so the only thing that could mark the active
	 * one was a border on the window's own edge. A pill sits *inside* the rail
	 * with room around it, which means the active one can be a raised object —
	 * and that is a much louder answer to "where am I" than three pixels of
	 * accent at the far left.
	 */
	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		width: calc(100% - 12px);
		margin: 1px 6px;
		padding: 6px 10px;
		border-radius: var(--r-button);
		font-size: var(--fs-secondary);
		text-align: left;
		transition:
			background var(--t-fast) var(--ease),
			box-shadow var(--t-fast) var(--ease),
			transform var(--t-fast) var(--spring),
			color var(--t-fast) var(--ease);
	}

	/*
	 * Hover changes the colour and nothing else (TASK-042).
	 *
	 * It used to slide the row two pixels right, which is a target moving out
	 * from under a pointer that is on its way to it — on a list of fourteen rows
	 * where the pointer crosses several to reach one, that is fourteen small
	 * shifts on the way down. The fill already says "this one", and it says it
	 * without moving anything.
	 *
	 * The **press** keeps its motion: a press is a deliberate act and the
	 * feedback is a key going down, which is a state change worth showing. That
	 * is the line this task draws — motion for what somebody did, not for where
	 * their pointer happens to be.
	 */
	.item:hover {
		background: var(--hover);
	}

	.item:active {
		transform: scale(0.99);
	}

	/*
	 * The active item is a flat accent-tinted pill and nothing else.
	 *
	 * It had a bar down its leading edge and a halo under it as well, which
	 * together read as a raised blue-edged tab rather than as "you are here" —
	 * the extra depth was the loudest thing in the rail and it was saying
	 * nothing the tint and the accent label do not already say. No inset bar,
	 * no glow, no rim.
	 */
	.item[data-active='true'] {
		background: color-mix(in srgb, var(--accent) 16%, transparent);
		color: var(--accent);
		font-weight: 600;
	}

	/* The active row does not press either: it is already where you are. */
	.item[data-active='true']:active {
		transform: none;
	}

	.spacer {
		flex: 1;
	}

	/* A tiling compositor can make a window narrower than the application's
	   requested minimum. Compact the chrome instead of squeezing the workspace. */
	@media (max-width: 900px) {
		.rail {
			width: 48px;
		}

		.head {
			justify-content: center;
			padding: 8px 4px;
		}

		.group {
			display: none;
		}

		.count,
		.name > span,
		.responsive-label {
			display: none;
		}

		.open {
			padding: 8px 4px;
		}

		.item,
		.open-repository {
			justify-content: center;
			width: calc(100% - 8px);
			margin-inline: 4px;
			padding-inline: 0;
		}
	}
</style>
