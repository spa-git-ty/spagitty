<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { graph } from '$lib/graph/store.svelte';
	import LaneCanvas from '$lib/graph/LaneCanvas.svelte';
	import GraphHeader from '$lib/graph/GraphHeader.svelte';
	import { visibleRange } from '$lib/graph/lanes';
	import { branchOf, byAuthor } from '$lib/graph/highlight';
	import { createPeek } from '$lib/graph/peek.svelte';
	import * as api from '$lib/api';
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';
	import { columns } from '$lib/graph/columns.svelte';
	import { freezeAt, frozenLeft } from '$lib/graph/freeze';
	import { overlay } from '$lib/graph/overlay.svelte';
	import { visibility } from '$lib/graph/visibility.svelte';
	import { selection } from '$lib/graph/selection.svelte';
	import * as act from '$lib/graph/actions';
	import { clockTime, fullDate, isNotable, relativeTime } from '$lib/format';
	import {
		LANE_STROKE,
		laneColorVar,
		laneColumnWidth,
		laneNodeRadius,
		laneSpanFor,
		laneX
	} from '$lib/metrics';
	import { density } from './density.svelte';
	import { avatars } from '$lib/graph/avatars.svelte';
	import { scale } from '$lib/scale.svelte';
	import RefChip from '$lib/ui/RefChip.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import type { GraphRow, RefChip as Chip } from '$lib/types';

	/**
	 * The virtualized commit list, and everything you can do to it.
	 *
	 * One DOM node per *visible* row, positioned by transform inside a sizer
	 * that is `count × pitch` tall. The lane canvas is a sibling pinned over the
	 * lane column and redrawn from the same scroll offset, so rows and lanes
	 * cannot drift apart: both are `index × pitch`, from one value.
	 *
	 * The interactions the handoff asks for all live here because they are all
	 * about the same rows — clicking, multi-selecting, right-clicking a commit
	 * or a label, dragging one label onto another, hovering for a highlight.
	 * What they *do* lives in `actions.ts`; this file decides what is offered.
	 */

	interface Props {
		onopen?: (id: string) => void;
		/** Opening the working copy — what clicking the WIP node does. */
		onwip?: () => void;
	}

	let { onopen, onwip }: Props = $props();

	let scroller = $state<HTMLDivElement | null>(null);
	let scrollTop = $state(0);
	let viewportHeight = $state(0);

	/** The refs gutter shows at most this many chips before collapsing. */
	const MAX_CHIPS = 2;

	const pitch = $derived(scale.pitch);
	const range = $derived(visibleRange(scrollTop, viewportHeight, graph.count, 4, pitch));

	const rows = $derived.by(() => {
		// Read the version so new batches re-render.
		void graph.version;
		const out: GraphRow[] = [];
		for (let i = range.first; i <= range.last; i++) {
			const row = graph.row(i);
			if (row) out.push(row);
		}
		return out;
	});

	// Pull more history when the viewport approaches the end of what is loaded.
	$effect(() => {
		if (range.last >= 0) graph.ensure(range.last);
	});

	/*
	 * Lane column sized to the history, not to the viewport.
	 *
	 * `graph.lanes` is the true lane count over every loaded row and is clamped
	 * at neither end (FEAT-035, FEAT-046); `laneColumnWidth` clamps the width it
	 * produces, and past the cap it is the pitch that gives.
	 *
	 * It used to be measured over the rows on screen, with a delay before
	 * shrinking. That still re-spaced every lane and resized the column whenever
	 * a scroll crossed from shallow history into deep, so the graph shifted under
	 * the reader. A figure taken over the whole history only changes while the
	 * walk streams in.
	 */
	const laneCount = $derived(graph.lanes);

	/**
	 * The graph column's width, and the room the lanes get inside it.
	 *
	 * Until someone drags it the column sizes itself to the lanes on screen, and
	 * `laneColumnWidth` is that size. Once dragged, the chosen width wins
	 * (FEAT-039) — but it no longer squeezes the lanes. Lanes that fit keep their
	 * resting x, and the ones the edge reaches fold onto it, track and node
	 * together, until at the narrowest the whole graph shares lane 0 (FEAT-081).
	 * `laneSpan` is what tells the painter and the hover targets where that edge
	 * is.
	 */
	const laneWidth = $derived(
		columns.width('graph') || laneColumnWidth(laneCount, scale.zoom, density.current)
	);
	const laneSpan = $derived(laneSpanFor(laneWidth, scale.zoom, density.current));
	const shown = $derived(columns.shown);

	/**
	 * Total width of the columns once none of them fills, or null while one
	 * still does.
	 *
	 * Null is the ordinary case: the message column takes what is left and
	 * nothing scrolls sideways. Once it has been given a width of its own the
	 * columns can add up to more than the window, and the rows, the header and
	 * the lane layer all have to be that wide together — otherwise they scroll
	 * different amounts and the graph leaves its column again, which is BUG-003
	 * wearing a different hat.
	 */
	const tableWidth = $derived.by(() => {
		const total = columns.totalWidth;
		return total === null ? null : total - columns.width('graph') + laneWidth;
	});

	/** How far the rows are scrolled sideways; the header follows it. */
	let scrollLeft = $state(0);

	/**
	 * Whether there is more table off either side.
	 *
	 * A scrollbar is the honest answer to "can this scroll", and on every
	 * platform Spagitty runs on it is also an overlay that is invisible until
	 * you are already scrolling — which makes it no answer at all to somebody
	 * deciding whether there is anything over there. The edge shadow is: it
	 * appears exactly when content is hidden under it and goes when it is not.
	 *
	 * Measured rather than derived from the column widths. The table is wider
	 * than the window only sometimes, and by an amount that depends on a
	 * dragged width, a zoom and the window — the element already knows all
	 * three.
	 */
	let scrollWidth = $state(0);
	let scrollerWidth = $state(0);

	/**
	 * The commit-list pane starts at the message column.
	 *
	 * Columns before it — Branch/Tag and Graph — pan sideways. The pane stays
	 * put, the graph slides under it, and a shadow on the seam is what says
	 * so. That is GitKraken's model, and it is why sideways scrolling used to
	 * drag the subject line around under the eye.
	 */
	const freezeIndex = $derived(freezeAt(shown));

	function columnWidth(column: (typeof shown)[number]): number {
		return column.id === 'graph' ? laneWidth : column.width;
	}

	const scrollingWidth = $derived.by(() => {
		let width = 0;
		for (let i = 0; i < freezeIndex; i++) width += columnWidth(shown[i]);
		return width;
	});

	const frozenPackWidth = $derived.by(() => {
		if (tableWidth === null) return Math.max(0, scrollerWidth - scrollingWidth);
		let width = 0;
		for (let i = freezeIndex; i < shown.length; i++) width += columnWidth(shown[i]);
		return width;
	});

	const paneLeft = $derived(frozenLeft(scrollingWidth, frozenPackWidth, scrollLeft, scrollerWidth));

	/** Sticky `right` for each frozen column, last column at 0. */
	const frozenRight = $derived.by(() => {
		let fixed = 0;
		for (let i = freezeIndex; i < shown.length; i++) {
			if (!shown[i].fills) fixed += columnWidth(shown[i]);
		}
		const fillWidth = Math.max(0, frozenPackWidth - fixed);
		const rights: number[] = [];
		let acc = 0;
		for (let i = shown.length - 1; i >= freezeIndex; i--) {
			rights[i] = acc;
			acc += shown[i].fills ? fillWidth : columnWidth(shown[i]);
		}
		return rights;
	});

	function freezeStyle(index: number): string {
		if (index < freezeIndex) return '';
		return `position: sticky; right: ${frozenRight[index] ?? 0}px; z-index: 3;`;
	}

	const moreLeft = $derived(scrollLeft > 0);
	// A pixel of slack: fractional scroll positions at non-integer zoom levels
	// otherwise leave the shadow up forever at the far right.
	const moreRight = $derived(scrollLeft + scrollerWidth < scrollWidth - 1);

	function measure(element: HTMLElement) {
		scrollWidth = element.scrollWidth;
		scrollerWidth = element.clientWidth;
	}

	// Measured when the table's width changes as well as on scroll, or the
	// right edge would stay dark until the first sideways scroll — which is
	// exactly the moment it is no longer needed.
	$effect(() => {
		void tableWidth;
		void laneWidth;
		if (scroller) measure(scroller);
	});


	// --- Dimming ----------------------------------------------------------

	/**
	 * Which rows stay bright. Null means nothing is dimmed, which is not the
	 * same as an empty set — an empty set dims everything.
	 *
	 * The author filter is the only thing that dims now. Hovering a branch label
	 * used to grey out every commit outside that branch, and hovering a row drew
	 * a dashed ghost line up to its nearest reference; both came out in
	 * FEAT-023. A hover is the pointer resting somewhere on its way elsewhere,
	 * and answering it by draining the colour out of most of the screen makes
	 * the graph flicker as the mouse crosses it. The filter is a standing
	 * question the user typed, which is a different thing and keeps its answer.
	 */
	const highlight = $derived.by(() => {
		void graph.version;
		return byAuthor(columns.author, (i) => graph.row(i), range.first, range.last + 1);
	});

	// --- Hover (FEAT-081) -------------------------------------------------

	/**
	 * The row under the pointer, for the one thing hovering says in the gutter.
	 *
	 * `:hover` already tints the row; this is state rather than CSS only because
	 * the contextual branch name has to be *computed* for the hovered row, and
	 * computing it for every visible row to reveal one would read the history
	 * forty times on each scroll.
	 */
	let hovered = $state<number | null>(null);

	/**
	 * The branch a hovered bare commit is on, drawn faintly in its gutter.
	 *
	 * Not a chip. A chip is a ref that exists *at this commit*, and the gutter's
	 * chips are what a person reads to find where branches are; a hover that
	 * put a chip-shaped thing there would lie about that for as long as the
	 * pointer rested. So it is plain, muted text — a caption, not a label — on
	 * the hovered row only, and nothing else on screen changes. See `branchOf`
	 * for how the branch is chosen.
	 */
	const contextBranch = $derived.by(() => {
		if (hovered === null || !columns.isShown('refs')) return null;
		void graph.version;
		return branchOf(hovered, (i) => graph.row(i));
	});

	/** The full message, after the pointer rests on a subject. See `peek.svelte.ts`. */
	const peek = createPeek({ lookup: api.commitDetail, loaded: () => graph.detail });

	$effect(() => () => peek.leave());

	/**
	 * Where the tooltip goes: below-right of the pointer, kept on screen.
	 *
	 * Measured after it renders, because a message's length decides its size
	 * and there is no knowing that from here without laying it out.
	 */
	let peekWidth = $state(0);
	let peekHeight = $state(0);
	const peekPosition = $derived.by(() => {
		const showing = peek.current;
		if (!showing || typeof window === 'undefined') return null;
		const gap = 14;
		const left = Math.max(8, Math.min(showing.x + gap, window.innerWidth - peekWidth - 8));
		const below = showing.y + gap + 6;
		const top =
			below + peekHeight + 8 > window.innerHeight
				? Math.max(8, showing.y - gap - peekHeight)
				: below;
		return { left, top };
	});

	// --- Who a node belongs to (FEAT-079) --------------------------------

	/**
	 * Ask for the real picture of everybody currently on screen.
	 *
	 * Here rather than in the canvas because the canvas paints inside a scroll
	 * frame: a fling across a large repository would ask once per author *per
	 * frame*, and the store would spend that fling filling and draining a
	 * queue. This runs when the visible range changes, which is the rate the
	 * question actually changes at.
	 *
	 * The store deduplicates by address and never asks twice, so the loop being
	 * a loop over rows rather than over authors costs a map lookup each.
	 */
	$effect(() => {
		void graph.version;
		void avatars.enabled;

		for (let i = range.first; i <= range.last; i++) {
			const row = graph.row(i);
			if (row) avatars.lookup(row.authorEmail ?? '', row.authorName, row.id);
		}
	});

	/**
	 * How far into the lane column a row's node sits, and how wide it is.
	 *
	 * The same two functions the canvas draws with, so the hover target and
	 * the circle it is over cannot land in different places — a target computed
	 * a second way would drift the first time a column was dragged.
	 */
	function nodeAt(row: GraphRow): { x: number; r: number } {
		return {
			x: laneX(row.lane, laneCount, scale.zoom, laneSpan, density.current),
			// The node's size is the density's alone — not the drag, not the
			// depth in view — so a hover target cannot change size as history
			// scrolls past it any more than the circle under it can (FEAT-081).
			r: laneNodeRadius(density.current) * scale.zoom
		};
	}

	/**
	 * What hovering a node says: the name, the address, and the handle when the
	 * address spelled one out.
	 *
	 * A merge node is excluded by the caller rather than here — it is drawn as
	 * a plain dot precisely because it is not one person's work, and naming its
	 * author over the branch it swallowed would be the claim the dot exists to
	 * avoid.
	 */
	function describeAuthor(row: GraphRow): string {
		const { handle } = avatars.lookup(row.authorEmail ?? '', row.authorName);
		const parts = [row.authorName, row.authorEmail].filter(Boolean);
		if (handle) parts.push(`@${handle}`);
		return parts.join(' · ');
	}

	/** Row index -> how many stashes hang off it. */
	const stashRows = $derived.by(() => {
		void graph.version;
		const map = new Map<number, number>();
		if (overlay.stashes.length === 0) return map;

		const wanted = new Map<string, number>();
		for (const entry of overlay.stashes) {
			wanted.set(entry.parent, (wanted.get(entry.parent) ?? 0) + 1);
		}
		for (let i = range.first; i <= range.last; i++) {
			const row = graph.row(i);
			if (!row) continue;
			const count = wanted.get(row.id);
			if (count) map.set(i, count);
		}
		return map;
	});

	// --- Menus ------------------------------------------------------------

	let menu = $state<{ x: number; y: number; items: MenuItem[]; label: string } | null>(null);

	function openMenu(event: MouseEvent, label: string, items: MenuItem[]) {
		event.preventDefault();
		event.stopPropagation();
		menu = { x: event.clientX, y: event.clientY, items, label };
	}

	/**
	 * What can be done to a commit.
	 *
	 * When several rows are selected the destructive-to-many operations act on
	 * the whole selection and say so; the single-commit ones still act on the
	 * row that was right-clicked, because that is the one under the pointer.
	 */
	function commitMenu(row: GraphRow): MenuItem[] {
		const picked = selection.ordered();
		const many = picked.length > 1 && picked.includes(row.index);
		const ids = many ? picked.map((i) => graph.row(i)?.id ?? '').filter(Boolean) : [row.id];
		const labels = many
			? picked.map((i) => graph.row(i)?.short ?? '')
			: [row.short];
		// Oldest first: that is the order they will be replayed in.
		const oldestFirst = [...ids].reverse();
		const oldest = many ? graph.row(picked[picked.length - 1]) : row;

		const items: MenuItem[] = [
			{ heading: many ? `${picked.length} commits selected` : row.short },
			{
				id: 'branch',
				label: 'Create branch here',
				run: () => act.createBranchAt(row.id, row.short)
			},
			{ id: 'tag', label: 'Create tag here', run: () => act.createTagAt(row.id, row.short) },
			{ separator: true },
			{
				id: 'cherry',
				label: many ? `Cherry pick ${picked.length} commits` : 'Cherry pick',
				run: () => act.cherryPick(oldestFirst, labels)
			},
			{ id: 'revert', label: 'Revert', run: () => act.revertCommit(row.id, row.short) },
			{ separator: true },
			{
				id: 'reset-soft',
				label: 'Reset here — keep and stage the changes',
				note: 'soft',
				run: () => act.resetTo(row.id, row.short, 'soft')
			},
			{
				id: 'reset-mixed',
				label: 'Reset here — keep the changes unstaged',
				note: 'mixed',
				run: () => act.resetTo(row.id, row.short, 'mixed')
			},
			{
				id: 'reset-hard',
				label: 'Reset here — discard the changes',
				note: 'hard',
				danger: true,
				run: () => act.resetTo(row.id, row.short, 'hard')
			},
			{ separator: true },
			{
				id: 'rebase-onto',
				label: 'Rebase onto this commit',
				danger: true,
				run: () => act.rebaseOntoCommit(row.id, row.short)
			}
		];

		// Moving a *run* of commits needs somewhere to move it to, and the only
		// unambiguous target on screen is the checked-out branch's position.
		if (many && oldest) {
			const target = visibility.branches.find((branch) => branch.current);
			items.push({
				id: 'rebase-range',
				label: target
					? `Rebase these ${picked.length} onto ${target.name}`
					: `Rebase these ${picked.length}`,
				disabled: !target,
				reason: target ? undefined : 'no branch checked out',
				danger: true,
				run: () =>
					target && act.rebaseRangeOnto(oldest, picked.length, target.name)
			});
		}

		items.push(
			{ separator: true },
			{
				id: 'checkout',
				label: 'Check out this commit',
				note: 'detached',
				run: () => act.checkoutCommit(row.id, row.short)
			},
			{ id: 'copy', label: 'Copy SHA', note: row.short, run: () => act.copyId(row.id, row.short) },
			{ id: 'diff', label: 'Open the diff', run: () => onopen?.(row.id) }
		);

		return items;
	}

	/** What can be done to a branch or tag label. */
	function refMenu(chip: Chip, row: GraphRow): MenuItem[] {
		const branch = visibility.branches.find(
			(candidate) => candidate.name === chip.name && candidate.kind === chip.kind
		);
		const fullName = branch?.fullName ?? chip.name;
		const current = visibility.branches.find((candidate) => candidate.current);
		const isCurrent = chip.current;

		if (chip.kind === 'tag') {
			return [
				{ heading: chip.name },
				{ id: 'copy', label: 'Copy SHA', note: row.short, run: () => act.copyId(row.id, row.short) },
				{ separator: true },
				{ id: 'delete', label: 'Delete tag', danger: true, run: () => act.deleteTag(chip.name) }
			];
		}

		const items: MenuItem[] = [
			{ heading: chip.name },
			{
				id: 'checkout',
				label: 'Check out',
				disabled: isCurrent,
				reason: isCurrent ? 'already on it' : undefined,
				run: () => act.checkoutBranch(chip.name)
			},
			{ separator: true }
		];

		// Integrating a branch into itself is not a thing, and neither is
		// integrating anything when nothing is checked out.
		for (const entry of act.INTEGRATIONS) {
			items.push({
				id: entry.how,
				label: `${entry.label} into ${current?.name ?? '…'}`,
				disabled: isCurrent || !current,
				reason: isCurrent ? 'that is the current branch' : !current ? 'nothing checked out' : undefined,
				danger: entry.how === 'rebase',
				run: () => current && act.integrate(chip.name, current.name, entry.how)
			});
		}

		items.push(
			{ separator: true },
			{
				id: 'pin',
				label: visibility.isPinned(fullName) ? 'Unpin from the left' : 'Pin to the left',
				run: () => visibility.togglePin(fullName)
			},
			{ id: 'solo', label: 'Show only this branch', run: () => visibility.solo(fullName) },
			{ id: 'hide', label: 'Hide this branch', run: () => visibility.hide(fullName) }
		);

		if (chip.kind === 'branch') {
			items.push(
				{ separator: true },
				{ id: 'rename', label: 'Rename', run: () => act.renameBranch(chip.name) },
				{
					id: 'delete',
					label: 'Delete',
					disabled: isCurrent,
					reason: isCurrent ? 'checked out' : undefined,
					danger: !(branch?.merged ?? false),
					run: () => act.deleteBranch(chip.name, branch?.merged ?? false)
				}
			);
		}

		return items;
	}

	// --- Dragging one label onto another ----------------------------------

	/**
	 * The signature interaction: drag a branch label onto another and choose
	 * what that means. It is the same four operations the right-click menu
	 * offers, asked the other way round — by pointing at the pair rather than by
	 * naming the target.
	 */
	let dragged = $state<Chip | null>(null);
	let dropTarget = $state<string | null>(null);

	function dropOnRef(event: DragEvent, target: Chip) {
		event.preventDefault();
		const source = dragged;
		dragged = null;
		dropTarget = null;
		if (!source || source.name === target.name) return;

		menu = {
			x: event.clientX,
			y: event.clientY,
			label: `${source.name} onto ${target.name}`,
			items: [
				{ heading: `${source.name} → ${target.name}` },
				...act.INTEGRATIONS.map((entry) => ({
					id: entry.how,
					label: entry.label,
					danger: entry.how === 'rebase',
					// Every one of these acts on the checked-out branch, so the
					// target has to be checked out first. Saying so beats a
					// refusal from git a second later.
					disabled: !target.current,
					reason: target.current ? undefined : `check out ${target.name} first`,
					run: () => act.integrate(source.name, target.name, entry.how)
				}))
			]
		};
	}

	// --- Selection --------------------------------------------------------

	function click(event: MouseEvent, index: number) {
		if (event.shiftKey) {
			selection.extendTo(index);
		} else if (event.ctrlKey || event.metaKey) {
			selection.toggle(index);
		} else {
			selection.only(index);
			graph.select(index);
		}
	}

	function scrollIntoView(index: number) {
		if (!scroller) return;
		const top = index * pitch;
		const bottom = top + pitch;
		if (top < scroller.scrollTop) {
			scroller.scrollTop = top;
		} else if (bottom > scroller.scrollTop + viewportHeight) {
			scroller.scrollTop = bottom - viewportHeight;
		}
	}

	function onkeydown(event: KeyboardEvent) {
		const current = graph.selectedIndex ?? -1;
		let next: number | null = null;

		switch (event.key) {
			case 'ArrowDown':
				next = Math.min(graph.count - 1, current + 1);
				break;
			case 'ArrowUp':
				next = Math.max(0, current - 1);
				break;
			case 'Home':
				next = 0;
				break;
			case 'End':
				next = graph.count - 1;
				break;
			case 'Enter':
				if (current >= 0) {
					const row = graph.row(current);
					if (row) onopen?.(row.id);
				}
				return;
			case 'Escape':
				selection.clear();
				return;
			default:
				return;
		}

		if (next !== null && next >= 0) {
			event.preventDefault();
			// Shift extends the selection, matching what shift-click does.
			if (event.shiftKey) selection.extendTo(next);
			else selection.only(next);
			graph.select(next);
			scrollIntoView(next);
		}
	}

	/** Time is shown only on the first row of each day, as a landmark. */
	function notableTime(index: number, time: number): string | null {
		const previous = graph.row(index - 1);
		return isNotable(time, previous?.time) ? relativeTime(time) : null;
	}
</script>

<div class="body">
	<GraphHeader
		{laneWidth}
		{scrollLeft}
		freezeAt={freezeIndex}
		frozenLeft={paneLeft}
		{scrollingWidth}
	/>

	{#if overlay.wip}
		<!--
			The working copy, above the newest commit — which is where it belongs
			in time, not a compromise for being unable to give it a row index.
		-->
		<button class="wip" title="Open the working copy" onclick={() => onwip?.()}>
			<span class="wip-node" aria-hidden="true"></span>
			<span class="wip-text">
				Uncommitted changes
				<span class="note">
					{overlay.wip.staged} staged · {overlay.wip.unstaged} unstaged
				</span>
			</span>
		</button>
	{/if}

	<div class="rows">
		<!--
			The bed the columns stand on (BUG-016).

			The graph's band used to be painted by `.lane-space`, which is a
			cell inside a row — so on a repository with fewer commits than the
			window is tall, the columns stopped where the commits stopped and
			the table read as though it had been cut off halfway down.

			This layer is the same column arithmetic as a row and as the canvas
			above it, laid out once at the full height of the scroller. It is
			built from `columns.shown`, like the other two, so a resize, a
			reorder or a hidden column moves all three together or none of them.

			It paints only where a row does not cover it, which is exactly the
			empty space under the last commit.
		-->
		<div
			class="bed"
			style={tableWidth === null ? '' : `width: ${tableWidth}px`}
			aria-hidden="true"
		>
			<div class="bed-scroll" style="width: {scrollingWidth}px; transform: translateX({-scrollLeft}px)">
				{#each shown.slice(0, freezeIndex) as column (column.id)}
					{#if column.id === 'graph'}
						<div class="bed-slot lane-band" style="width: {laneWidth}px"></div>
					{:else}
						<div class="bed-slot" style="width: {column.width}px"></div>
					{/if}
				{/each}
			</div>
			<div class="bed-frozen" style="left: {paneLeft}px"></div>
		</div>

		<div
			class="scroller"
			bind:this={scroller}
			bind:clientHeight={viewportHeight}
			bind:clientWidth={scrollerWidth}
			onscroll={(event) => {
				// Scrolling moves a different row under a pointer that has not
				// moved, so whatever the tooltip was describing has gone.
				peek.leave();
				scrollTop = event.currentTarget.scrollTop;
				scrollLeft = event.currentTarget.scrollLeft;
				measure(event.currentTarget);
			}}
			{onkeydown}
			role="listbox"
			aria-label="Commits"
			aria-multiselectable="true"
			aria-activedescendant={graph.selectedIndex === null
				? undefined
				: `commit-${graph.selectedIndex}`}
			tabindex="0"
		>
			<div
				class="sizer"
				style="height: {graph.count * pitch}px; {tableWidth === null
					? ''
					: `width: ${tableWidth}px`}"
			>
				{#each rows as row (row.index)}
					{@const time = notableTime(row.index, row.time)}
					{@const extra = row.refs.length - MAX_CHIPS}
					{@const dim = highlight !== null && !highlight.has(row.index)}
					{@const node = nodeAt(row)}
					{@const laneColor = `var(${laneColorVar(row.color)})`}
					{@const lead = LANE_STROKE * scale.zoom}
					{@const leadTop = (pitch - lead) / 2}
					<!--
						Keyboard handling lives on the listbox, not on each option —
						that is the ARIA pattern, and it is also the only workable
						one here: options are virtualized, so most of them do not
						exist as DOM nodes to receive a key event.
					-->
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<div
						id="commit-{row.index}"
						class="row"
						class:selected={selection.has(row.index)}
						class:focused={graph.selectedIndex === row.index}
						class:dim
						style="transform: translateY({row.index * pitch}px)"
						role="option"
						aria-selected={selection.has(row.index)}
						tabindex="-1"
						onclick={(event) => click(event, row.index)}
						ondblclick={() => onopen?.(row.id)}
						oncontextmenu={(event) => openMenu(event, 'Commit', commitMenu(row))}
						onpointerenter={() => (hovered = row.index)}
						onpointerleave={() => {
							if (hovered === row.index) hovered = null;
						}}
					>
						{#each shown as column, index (column.id)}
							{#if column.id === 'refs'}
								<div class="cell refs" style="width: {column.width}px">
									{#each row.refs.slice(0, MAX_CHIPS) as chip (chip.kind + chip.name)}
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<span
											class="chip-slot"
											class:target={dropTarget === chip.name}
											draggable={chip.kind !== 'tag'}
											role="button"
											tabindex="-1"
											ondragstart={() => (dragged = chip)}
											ondragend={() => {
												dragged = null;
												dropTarget = null;
											}}
											ondragover={(event) => {
												if (!dragged || dragged.name === chip.name) return;
												event.preventDefault();
												dropTarget = chip.name;
											}}
											ondragleave={() => {
												if (dropTarget === chip.name) dropTarget = null;
											}}
											ondrop={(event) => dropOnRef(event, chip)}
											ondblclick={(event) => {
												event.stopPropagation();
												if (chip.kind !== 'tag') act.checkoutBranch(chip.name);
											}}
											oncontextmenu={(event) => openMenu(event, 'Reference', refMenu(chip, row))}
										>
											<RefChip {chip} />
										</span>
									{/each}
									{#if extra > 0}
										<span class="more" title={row.refs.map((r) => r.name).join(', ')}>
											+{extra}
										</span>
									{/if}
									{#if row.refs.length === 0 && hovered === row.index && contextBranch}
										<span class="context-branch" title={`On ${contextBranch}`}>
											{contextBranch}
										</span>
									{/if}
									{#if row.refs.length > 0}
										<!--
											Grows from the last chip to the graph column,
											where `.graph-lead` continues it into the node.
										-->
										<span
											class="ref-lead"
											style="height: {lead}px; margin-top: {leadTop}px; background: {laneColor}"
											aria-hidden="true"
										></span>
									{/if}
								</div>
							{:else if column.id === 'graph'}
								<!--
									Reserves the lane column; the canvas overlays exactly
									this.

									It also carries the node's hover target (FEAT-079). The
									canvas above cannot: it is one element for the whole
									column and takes no pointer events, by design, so that
									the rows underneath keep their clicks. This is that row,
									at that row's height already, so the target only has to
									find the node's x — and it finds it with the same
									functions the canvas drew it with.
								-->
								<div class="cell lane-space lane-band" style="width: {laneWidth}px">
									{#if row.refs.length > 0}
										<span
											class="ref-lead graph-lead"
											style="width: {Math.max(0, node.x)}px; height: {lead}px; top: {leadTop}px; background: {laneColor}"
											aria-hidden="true"
										></span>
									{/if}
									{#if row.parents.length <= 1}
										<span
											class="node-hit"
											style="left: {node.x - node.r}px; width: {node.r * 2}px; height: {node.r *
												2}px"
											title={describeAuthor(row)}
										></span>
									{/if}
								</div>
							{:else if column.id === 'message'}
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="cell message frozen"
									style={freezeStyle(index)}
									onpointerenter={(event) =>
										peek.enter(row.index, row.id, event.clientX, event.clientY)}
									onpointermove={(event) => peek.move(event.clientX, event.clientY)}
									onpointerleave={() => peek.leave()}
									onpointerdown={() => peek.leave()}
								>
									{#if row.signed}
										<!--
											FEAT-019. `S`, not a tick: the tick already means
											"this is the branch you are on" on a RefChip, and
											two meanings for one mark on one screen is worse
											than a letter that has to be hovered once.

											It says signed. It does not say verified — the
											title spells that out, because the difference is
											the whole reason this is cheap enough to show on
											every row.
										-->
										<span
											class="mono signed"
											title="Signed. Spagitty reads the signature header; it does not verify the signature."
											aria-label="signed">S</span
										>
									{/if}
									<!--
										No `title`: it repeated the subject, which is already on
										screen, and would stack a native tooltip on the full
										message (FEAT-081).
									-->
									<span class="summary">{row.summary}</span>
									{#if time}<span class="mono muted when">{time}</span>{/if}
								</div>
							{:else if column.id === 'author'}
								<div
									class="cell text author frozen"
									style="width: {column.width}px; {freezeStyle(index)}"
								>
									<!--
										The same mark the node carries, resolved the same way
										— one face per person on the screen, so the author
										column and the graph agree about who is who. The real
										picture when one has arrived, and initials on a
										stable colour until then (FEAT-079).
									-->
									<AuthorAvatar
										email={row.authorEmail}
										name={row.authorName}
										letters={row.initials}
										title={describeAuthor(row)}
									/>
									<span class="ellipsis" title={describeAuthor(row)}>{row.authorName}</span>
								</div>
							{:else if column.id === 'time'}
								<div
									class="cell text frozen"
									style="width: {column.width}px; {freezeStyle(index)}"
								>
									<span class="mono muted ellipsis">{fullDate(row.time)} {clockTime(row.time)}</span>
								</div>
							{:else if column.id === 'sha'}
								<div
									class="cell text frozen"
									style="width: {column.width}px; {freezeStyle(index)}"
								>
									<span class="mono muted">{row.short}</span>
								</div>
							{/if}
						{/each}
					</div>
				{/each}
			</div>
		</div>

		<!--
			The canvas is laid out by the same rules as a row, rather than being
			placed at a computed x.

			It used to sit at `--refs-gutter-w` — the design's 186px — which is
			the right answer only while Branch/Tag is at its default width and
			first in the order. Dragging that column, reordering, or hiding it
			left the canvas behind and the lanes landed on the messages (BUG-003).

			So this layer mirrors the row: one spacer per column ahead of the
			graph, with the same widths the cells use, and the canvas in the
			graph's slot. The browser does the arithmetic, which means the two
			cannot disagree — whatever moves a cell moves the canvas with it.
		-->
		<div
			class="lane-layer"
			style="width: {scrollingWidth}px; transform: translateX({-scrollLeft}px)"
			aria-hidden="true"
		>
			{#each shown.slice(0, freezeIndex) as column (column.id)}
				{#if column.id === 'graph'}
					<div class="lane-slot" style="width: {laneWidth}px">
						<LaneCanvas
							span={laneSpan}
							{scrollTop}
							first={range.first}
							last={range.last}
							width={laneWidth}
							height={viewportHeight}
							columns={laneCount}
							{highlight}
							stashes={stashRows}
						/>
					</div>
				{:else}
					<div class="lane-gap" style="width: {column.width}px"></div>
				{/if}
			{/each}
		</div>

		<!--
			The list pane's shadow. One strip for the whole height, not per row,
			or every commit would cast its own — a stack of seams. The pane
			itself is the sticky cells; this only says the graph continues
			underneath.
		-->
		<div class="list-shadow" style="left: {paneLeft}px" aria-hidden="true"></div>

		<!--
			The edges. Purely an affordance: they say there is table under them
			and take no clicks, so anything they cover stays reachable.
		-->
		<div class="edge left" class:showing={moreLeft} aria-hidden="true"></div>
		<div class="edge right" class:showing={moreRight} aria-hidden="true"></div>
	</div>
</div>

{#if peek.current && peekPosition}
	<!--
		Fixed to the viewport rather than placed inside the scroller, so the frozen
		pane, the edge shadows and the detail panel cannot clip it. It takes no
		pointer events: it describes the row under the pointer and must never
		become the thing under the pointer.
	-->
	<div
		class="peek"
		role="tooltip"
		bind:clientWidth={peekWidth}
		bind:clientHeight={peekHeight}
		style="left: {peekPosition.left}px; top: {peekPosition.top}px"
	>{peek.current.text}</div>
{/if}

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		items={menu.items}
		label={menu.label}
		onclose={() => (menu = null)}
	/>
{/if}

<style>
	.body {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	.rows {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	/*
	 * Three layers over the same columns, stacked explicitly rather than by
	 * document order: the bed underneath, the commits over it, the lane canvas
	 * over both. `.edge` is above all three at 4.
	 */
	.bed {
		position: absolute;
		inset: 0;
		pointer-events: none;
		z-index: 0;
	}

	.bed-scroll {
		display: flex;
		height: 100%;
	}

	.bed-slot {
		flex: none;
		height: 100%;
	}

	.bed-frozen {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		background: var(--bg);
		pointer-events: none;
	}

	/*
	   A gradient rather than a hard line, and over the content rather than
	   beside it: what it means is "this carries on underneath", and a rule
	   would say "this stops here" — the opposite.
	*/
	.edge {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 24px;
		pointer-events: none;
		opacity: 0;
		transition: opacity 0.12s ease;
		z-index: 4;
	}

	.edge.showing {
		opacity: 1;
	}

	.edge.left {
		left: 0;
		border-right: 1px solid var(--line);
		background: none;
	}

	.list-shadow {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 1px;
		pointer-events: none;
		z-index: 4;
		box-shadow: -8px 0 14px -4px color-mix(in srgb, var(--umbra) 32%, transparent);
	}

	.scroller {
		position: relative;
		z-index: 1;
		height: 100%;
		overflow-y: auto;
		/*
		 * Sideways scrolling exists only once the columns are wider than the
		 * window, which is possible now that the message column can be given a
		 * width of its own. Until then `.sizer` has no width of its own and
		 * there is nothing to scroll.
		 */
		overflow-x: auto;
		outline: none;
	}

	.sizer {
		position: relative;
		width: 100%;
	}

	.row {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: var(--row-pitch);
		display: flex;
		align-items: center;
		cursor: pointer;
		contain: style;
	}

	/*
	 * The graph is a surface of its own, not a gap between two columns.
	 *
	 * Each row paints its own slice of it, which is what keeps the fill aligned
	 * with the rows as they are translated during a scroll — a single element
	 * behind the scroller would have to be positioned against `scrollTop` by
	 * hand and would lag it by a frame. The canvas draws on top of these slices.
	 */
	/*
	 * The canvas layer: the same flex row as a commit row, over the top of them
	 * all, transparent to the pointer. `overflow: hidden` on the slot is the
	 * belt to the layout's braces — a canvas that is somehow the wrong size gets
	 * cut off at its column's edge instead of painting over a neighbour.
	 */
	.lane-layer {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		display: flex;
		pointer-events: none;
		z-index: 2;
	}

	.lane-gap {
		flex: none;
	}

	.lane-slot {
		flex: none;
		position: relative;
		height: 100%;
		overflow: hidden;
	}

	/*
	   The seam where the lanes end and the messages begin.

	   The lane column clips its canvas, so on a history deeper than the column
	   is wide the lanes stop at a hard vertical edge — which reads as "the
	   graph ends here" when what is true is "there is more of it than fits".
	   A short shadow at the edge says the second thing: the lanes pass under
	   the messages rather than stopping against them.

	   Always on, unlike the scroll edges either side of the table, because
	   what it marks is always true — this is a boundary between two columns,
	   not a scroll position.
	*/
	.lane-slot::after {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		right: 0;
		width: 14px;
		pointer-events: none;
		background: none;
	}

	/*
	 * The graph's surface.
	 *
	 * One declaration, worn by both the per-row cell and the bed beneath it, so
	 * the band cannot come out one colour where there are commits and another
	 * where there are none. No vertical rules: a line down each side of the
	 * column boxed the history into a table. The labels join their nodes with a
	 * lead instead, which is what says they belong together.
	 */
	.lane-band {
		background: var(--graph-bg);
	}

	/*
	 * The node's hover target (FEAT-079).
	 *
	 * A cell that used to be a spacer now positions one thing, so it needs a
	 * containing block. Nothing else about it changes: the canvas still paints
	 * over it, and the target is transparent.
	 */
	.lane-space {
		position: relative;
	}

	/*
	 * Invisible, and deliberately so — the circle a person is hovering is
	 * already drawn, on the canvas above this. Duplicating it here would mean
	 * two circles to keep in step, and the canvas is the one that can draw a
	 * picture.
	 *
	 * Sized to the node exactly rather than generously. A larger target would
	 * be easier to hit and would answer "who is this" for a pointer that is
	 * over the lane *beside* the node, which is a different row's line — and a
	 * tooltip naming the wrong person is worse than one that takes a second
	 * attempt to summon.
	 */
	.node-hit {
		position: absolute;
		top: 50%;
		transform: translateY(-50%);
		border-radius: 50%;
		/* The row owns the click; this only wants the pointer. */
		cursor: default;
	}

	.row:hover {
		background: var(--hover);
	}

	/*
	 * A selected row is tinted across its width rather than filled flat, so the
	 * lanes and the chips stay legible through it and the fill reads as a
	 * highlight rather than as a coloured block laid over the history.
	 */
	.row.selected {
		background: var(--selection);
	}

	/*
	 * The row the detail panel is showing, which is not the same as the set of
	 * rows a cherry-pick would act on.
	 *
	 * A bar down the leading edge rather than the inset accent glow this used
	 * to be (TASK-026): the glow was the last soft shadow on the graph, and a
	 * flat interface marks a row with a line. It still has to be tellable from
	 * `.selected`, which tints the whole width — so this one takes the edge.
	 */
	.row.focused {
		box-shadow: inset 2px 0 0 var(--accent);
	}

	.row.dim {
		opacity: 0.35;
	}

	.cell {
		flex: none;
		min-width: 0;
		height: 100%;
		display: flex;
		align-items: center;
	}

	/*
	   Chips start at the column's own left edge, so every row's first chip is at
	   the same x and the eye reads a column.

	   The lead after them takes whatever is left, so a label on the left is
	   still joined to the node it names. Tucking the chips against the graph
	   would shorten that line and scatter the names; the names stay a column.
	*/
	.refs {
		justify-content: flex-start;
		gap: 4px;
		padding: 0 0 0 8px;
		overflow: hidden;
	}

	/*
	 * The line from a label to its node.
	 *
	 * In the refs cell it grows from the last chip to the graph. In the graph
	 * cell it runs from the column's left edge to the node's centre — the
	 * canvas paints the node on top, so the lead disappears into the head
	 * rather than stopping short of it.
	 *
	 * Both halves take the same explicit offset from the row's top. They used
	 * to be centred two ways — flex centring on one side, `top: 50%` and a
	 * `translateY(-50%)` on the other — and with a 2.5px stroke or a zoomed
	 * pitch the centre falls on a half pixel, which the two rounded to
	 * different rows: a one-pixel step in the line where the columns meet.
	 */
	.ref-lead {
		flex: 1;
		min-width: 8px;
		align-self: flex-start;
		pointer-events: none;
	}

	.graph-lead {
		position: absolute;
		flex: none;
		left: 0;
	}

	/*
	 * The hovered bare commit's branch (FEAT-081). Text at the chip's size and
	 * nothing else — no border, no fill, no lead — and muted, so it cannot be
	 * mistaken for a ref that is actually at this commit.
	 */
	.context-branch {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
		color: var(--muted);
		opacity: 0.8;
		pointer-events: none;
	}

	.peek {
		position: fixed;
		z-index: 60;
		max-width: min(520px, calc(100vw - 16px));
		max-height: min(360px, calc(100vh - 16px));
		overflow: hidden;
		padding: 8px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-floating);
		background: var(--surface);
		color: var(--ink);
		box-shadow: 0 6px 18px color-mix(in srgb, var(--umbra) 28%, transparent);
		font-size: var(--fs-secondary);
		line-height: 1.45;
		/* Paragraph breaks and trailers are part of the message. */
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		pointer-events: none;
	}

	.chip-slot {
		display: inline-flex;
		border-radius: var(--r-pill);
	}

	.chip-slot.target {
		outline: 2px solid var(--accent);
		outline-offset: 1px;
	}

	.more {
		border: 1px solid var(--soft);
		border-radius: var(--r-field);
		padding: 0 5px;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		color: var(--muted);
		background: var(--surface);
		box-shadow: var(--sheen);
		flex: none;
	}

	.cell.frozen {
		background-color: var(--bg);
	}

	.row:hover .cell.frozen {
		background-color: color-mix(in srgb, var(--ink) 7%, var(--bg));
	}

	.row.selected .cell.frozen {
		/* `--selection` is a tint, not a fill — paint it over the pane or the
		   graph shows through the subject line. */
		background-color: var(--bg);
		background-image: linear-gradient(var(--selection), var(--selection));
	}

	.message {
		flex: 1;
		gap: 8px;
		padding: 0 10px;
		border-left: 1px solid var(--soft);
	}

	.text {
		gap: 6px;
		padding: 0 8px;
		border-left: 1px solid var(--soft);
	}

	.summary,
	.ellipsis {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.when {
		flex: none;
	}

	.author {
		gap: 6px;
	}

	.wip {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 6px 10px;
		border-bottom: 1px solid var(--soft);
		text-align: left;
		flex: none;
		/* It opens the working copy, so it has to look like it does something. */
		cursor: pointer;
	}

	.wip:hover {
		background: var(--hover);
	}

	/* Hollow, so it does not read as a commit that has happened. */
	.wip-node {
		width: 12px;
		height: 12px;
		flex: none;
		border: 2px dashed var(--accent);
		border-radius: 50%;
	}

	.wip-text {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}
</style>
