<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { columns, type ColumnId } from '$lib/graph/columns.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { createResizeDrag } from '$lib/ui/resize-drag';
	import { scale } from '$lib/scale.svelte';
	import type { MenuItem } from '$lib/ui/menu';

	/**
	 * The graph's column header.
	 *
	 * Three things live here, and they are all direct manipulation of the same
	 * strip: right-click to choose which columns exist, drag a header to reorder
	 * them, drag a divider to resize. The alternative — a preferences page with
	 * a list of checkboxes — puts the control a long way from the thing it
	 * controls, and this is a header people are already pointing at.
	 *
	 * The Graph column's width is the exception: it is computed from how many
	 * lanes are on screen, so its divider is not draggable. That is stated in
	 * the divider's title rather than left as a divider that mysteriously does
	 * nothing.
	 */

	interface Props {
		/**
		 * How far the rows are scrolled sideways.
		 *
		 * The header is not inside the rows' scroller — it must stay visible
		 * while they scroll vertically — so the graph half is moved by the same
		 * amount. The commit-list half is pinned, so it does not come along.
		 */
		scrollLeft?: number;
		/** Width of the lane column right now, which the store cannot know. */
		laneWidth: number;
		/** First column that belongs to the pinned commit-list pane. */
		freezeAt: number;
		/** Left edge of that pane, matching the rows. */
		frozenLeft: number;
		/** Combined width of the columns that pan with the graph. */
		scrollingWidth: number;
	}

	let {
		laneWidth,
		scrollLeft = 0,
		freezeAt,
		frozenLeft,
		scrollingWidth
	}: Props = $props();

	const shown = $derived(columns.shown);

	let menu = $state<{ x: number; y: number } | null>(null);
	/** Index of the header being dragged, and the slot it would land in. */
	let dragging = $state<number | null>(null);
	let over = $state<number | null>(null);
	/** Which column the drag in progress is sizing. */
	let resizing: ColumnId | null = null;
	/** The divider holding pointer capture, so teardown can let go of it. */
	let captured: { handle: HTMLElement; pointerId: number } | null = null;

	/**
	 * One drag at a time, applied once per frame and saved once at the end
	 * (FEAT-081). See `resize-drag.ts` for why a move is not a write.
	 */
	const drag = createResizeDrag({
		apply: (width) => {
			if (resizing) columns.drag(resizing, width);
		},
		settle: () => columns.settle()
	});

	// Unmounting mid-drag — switching repository tab with the button still held
	// — must not leave a frame pending against a store nobody is looking at, or
	// a width that was on screen but never saved.
	$effect(() => () => finishResize());

	/**
	 * Below this the word does not fit and the header shows the graph's icon
	 * instead (FEAT-081). "Graph" at the secondary size plus the cell's padding
	 * is a little over fifty pixels; seventy-two leaves the ellipsis out of it,
	 * because "Gr…" is a worse name for the column than a picture of one.
	 */
	const GRAPH_LABEL_MIN = 72;
	let filtering = $state(false);

	function widthOf(id: ColumnId): number {
		return id === 'graph' ? laneWidth : columns.width(id);
	}

	function openMenu(event: MouseEvent) {
		event.preventDefault();
		menu = { x: event.clientX, y: event.clientY };
	}

	const menuItems = $derived<MenuItem[]>([
		{ heading: 'Columns' },
		...columns.catalogue.map(({ column, shown: on }) => ({
			id: column.id,
			label: `${on ? '✓ ' : '   '}${column.label}`,
			disabled: column.required && on,
			reason: column.required && on ? 'always shown' : undefined,
			run: () => columns.toggle(column.id)
		})),
		{ separator: true as const },
		{ id: 'reset', label: 'Reset columns', run: () => columns.reset() }
	]);

	/**
	 * A divider sizes the column on its **left**.
	 *
	 * That is the only model under which the boundary goes where the pointer
	 * goes: everything left of it grows, everything right of it shifts along,
	 * and the filling column takes up whatever is left.
	 *
	 * Two earlier attempts got this wrong in ways only a person dragging it
	 * could see. Sizing the column *after* the divider changed that column's
	 * width while its left edge stayed pinned, so the commit message column
	 * shrank from its **right** edge and left a growing gap before the detail
	 * panel — the boundary never moved. Skipping backwards past the graph column
	 * moved the boundary correctly but left the graph itself unresizable, so
	 * narrowing the table could not reclaim the empty half of a wide graph
	 * column.
	 *
	 * Every column is now sizable, including the graph, whose lanes compress
	 * into whatever width they are given (FEAT-039). So the answer is the
	 * simplest one: the column the divider sits on.
	 */
	function resizeTarget(index: number): ColumnId {
		return shown[index].id;
	}

	function startResize(event: PointerEvent, index: number) {
		const id = resizeTarget(index);

		event.preventDefault();
		event.stopPropagation();

		// Measured, not read from the store: the filling column's stored width
		// is 0 until it is dragged, and starting a drag from 0 would snap it to
		// its minimum before the pointer had moved a pixel.
		const handle = event.currentTarget as HTMLElement;
		const cell = handle.closest('.header-clip')?.querySelector<HTMLElement>(`[data-column="${id}"]`);
		const startWidth = cell ? cell.getBoundingClientRect().width : columns.width(id);

		resizing = id;
		drag.start(event.clientX, startWidth);
		captured = { handle, pointerId: event.pointerId };
		try {
			handle.setPointerCapture(event.pointerId);
		} catch {
			// A pointer that is already gone cannot be captured. The drag still
			// ends on the next `up` that reaches this handle.
		}
	}

	function moveResize(event: PointerEvent) {
		if (!drag.active) return;
		drag.move(event.clientX);
	}

	/**
	 * Every way a drag stops comes through here: release, cancellation, lost
	 * capture and unmount. `clientX` is absent for the last three, which keep
	 * the boundary where the last frame drew it.
	 */
	function finishResize(clientX?: number) {
		drag.end(clientX);
		resizing = null;
		const held = captured;
		captured = null;
		if (held && held.handle.hasPointerCapture?.(held.pointerId)) {
			held.handle.releasePointerCapture(held.pointerId);
		}
	}

	function drop(to: number) {
		if (dragging !== null) columns.reorder(dragging, to);
		dragging = null;
		over = null;
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="header-clip"
	oncontextmenu={openMenu}
	role="row"
	tabindex="-1"
	aria-label="Graph columns"
>
	<div
		class="header-scroll"
		style="width: {scrollingWidth}px; transform: translateX({-scrollLeft}px)"
	>
		{#each shown.slice(0, freezeAt) as column, index (column.id)}
			{@render heading(column, index)}
		{/each}
	</div>
	<div class="header-frozen" style="left: {frozenLeft}px">
		{#if freezeAt > 0}
			<!--
				The seam handle lives on the pane, not on the graph column: the
				graph slides under this edge, and a divider that travelled with
				it would disappear the moment somebody panned.
			-->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="divider seam"
				title={`Resize ${shown[freezeAt - 1]?.label} — double-click to reset`}
				onpointerdown={(event) => startResize(event, freezeAt - 1)}
				onpointermove={moveResize}
				onpointerup={(event) => finishResize(event.clientX)}
				onpointercancel={() => finishResize()}
				onlostpointercapture={() => finishResize()}
				ondblclick={() => columns.unsize(resizeTarget(freezeAt - 1))}
			></div>
		{/if}
		{#each shown.slice(freezeAt) as column, offset (column.id)}
			{@render heading(column, freezeAt + offset)}
		{/each}
	</div>
</div>

{#snippet heading(column: (typeof shown)[0], index: number)}
	{@const target = resizeTarget(index)}
	{@const sized = shown.find((c) => c.id === target)?.label}
	{@const compact = column.id === 'graph' && widthOf(column.id) < GRAPH_LABEL_MIN * scale.zoom}
	<div
		class="cell"
		data-column={column.id}
		class:fills={column.fills}
		class:compact
		class:dragging={dragging === index}
		class:over={over === index && dragging !== index}
		style={column.fills ? '' : `width: ${widthOf(column.id)}px`}
		role="columnheader"
		aria-label={column.label}
		title={compact ? column.label : undefined}
		tabindex="-1"
		draggable="true"
		ondragstart={() => (dragging = index)}
		ondragend={() => {
			dragging = null;
			over = null;
		}}
		ondragover={(event) => {
			event.preventDefault();
			over = index;
		}}
		ondrop={(event) => {
			event.preventDefault();
			drop(index);
		}}
	>
		{#if compact}
			<!--
				The column is too narrow for its name, so it shows what it is
				instead (FEAT-081). Only a picture: it does not collapse or expand
				anything, and the reference never shows it doing so. The header
				keeps its accessible name, and the divider below is unchanged, so
				the column can still be dragged back open from here.
			-->
			<span class="label icon-label" aria-hidden="true"><Icon name="graph" size="1.2em" /></span>
		{:else}
			<span class="label note">{column.label}</span>
		{/if}

		{#if column.id === 'author'}
			<!--
				The filter lives in the Author column because that is what it
				filters, and it opens on click rather than always showing a
				field: a permanently open input in a header reads as a search
				box for the whole screen.
			-->
			{#if filtering || columns.author !== ''}
				<input
					class="filter"
					type="text"
					placeholder="filter…"
					spellcheck="false"
					aria-label="Filter by author"
					value={columns.author}
					oninput={(event) => columns.setAuthor(event.currentTarget.value)}
					onblur={() => (filtering = false)}
				/>
			{:else}
				<button
					class="filter-open"
					title="Filter by author"
					aria-label="Filter by author"
					onclick={() => (filtering = true)}
				>
					⌕
				</button>
			{/if}
		{/if}

		<!--
			Every column gets a divider, including the one that fills.
			Dragging the filling column is how it stops filling — before this
			it was the one column with no handle at all, which read as "this
			one is not resizable" rather than "this one takes what is left".
			Double-click hands the fill back.
		-->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="divider"
			class:last={index === shown.length - 1}
			class:silent={column.id === 'refs'}
			title={`Resize ${sized} — double-click to reset`}
			onpointerdown={(event) => startResize(event, index)}
			onpointermove={moveResize}
			onpointerup={(event) => finishResize(event.clientX)}
			onpointercancel={() => finishResize()}
			onlostpointercapture={() => finishResize()}
			ondblclick={() => columns.unsize(target)}
		></div>
	</div>
{/snippet}

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		items={menuItems}
		label="Columns"
		onclose={() => (menu = null)}
	/>
{/if}

<style>
	.header-clip {
		position: relative;
		overflow: hidden;
		flex: none;
		height: calc(var(--row-pitch) + 2px);
		border-bottom: 1px solid var(--line);
		background-color: var(--chrome-veil);
	}

	.header-scroll,
	.header-frozen {
		display: flex;
		align-items: stretch;
		height: 100%;
		font-size: var(--fs-secondary);
		font-weight: 550;
		letter-spacing: 0.02em;
		user-select: none;
	}

	.header-scroll {
		flex: none;
	}

	.header-frozen {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		z-index: 3;
		background-color: var(--chrome-veil);
		box-shadow: -8px 0 14px -4px color-mix(in srgb, var(--umbra) 32%, transparent);
	}

	.divider.seam {
		left: -3px;
		right: auto;
		z-index: 4;
	}

	.cell {
		position: relative;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 0 8px;
		min-width: 0;
		flex: none;
		cursor: grab;
	}

	.cell.compact {
		justify-content: center;
		padding: 0;
	}

	.icon-label {
		display: inline-flex;
		color: var(--muted);
	}

	.cell.fills {
		flex: 1;
		min-width: 0;
	}

	.cell.dragging {
		opacity: 0.4;
	}

	.cell.over {
		box-shadow: inset 0 -2px 0 var(--accent);
	}

	.label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.divider {
		position: absolute;
		top: 0;
		right: -3px;
		width: 7px;
		height: 100%;
		cursor: col-resize;
		z-index: 1;
	}

	/*
	 * The last column's divider sits wholly inside it.
	 *
	 * Every other divider straddles the boundary between two columns, which is
	 * what makes it feel like it belongs to both. The last one has nothing on
	 * its right but the window edge, so the same -3px put a third of the grab
	 * area off-screen and the rest against the frame — the message column, which
	 * is last by default, could not be resized at all.
	 */
	.divider.last {
		right: 0;
	}

	.divider::after {
		content: '';
		position: absolute;
		left: 3px;
		top: 0;
		width: 1px;
		height: 100%;
		background: var(--soft);
	}

	/* Its line stays on the column's own edge rather than moving in with it. */
	.divider.last::after {
		left: auto;
		right: 0;
	}

	/* A hover hint, not an announcement: the divider is a handle, and lighting
	   it in the accent read as something breaking along the header's edge. */
	.divider:hover::after {
		background: var(--line);
	}

	/*
	 * Between Branch/Tag and Graph there is no rule. The labels join their
	 * nodes with a lead; a column line there would cut that join at the header.
	 * The handle still exists — hover lights the line so the resize is not lost.
	 */
	.divider.silent::after {
		background: none;
	}

	.divider.silent:hover::after {
		background: var(--line);
	}



	.filter {
		font: inherit;
		font-size: var(--fs-secondary);
		color: inherit;
		background: var(--bg);
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		padding: 1px 5px;
		min-width: 0;
		width: 100%;
		outline: none;
	}

	.filter:focus {
		border-color: var(--accent);
	}

	.filter-open {
		font-size: var(--fs-ui);
		color: var(--muted);
		line-height: 1;
	}

	.filter-open:hover {
		color: var(--accent);
	}
</style>
