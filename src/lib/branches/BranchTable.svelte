<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { branches } from '$lib/branches/store.svelte';
	import { deleteBranch, renameBranch, undeletable } from '$lib/branches/actions';
	import { columns, type ColumnId } from '$lib/branches/columns.svelte';
	import DivergenceBar from '$lib/branches/DivergenceBar.svelte';
	import { widest } from '$lib/branches/divergence';
	import { relativeTime } from '$lib/format';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * Branch, drift, last change, actions.
	 *
	 * Merged branches render dashed: the same device the rest of the application
	 * uses for "not settled", here meaning "nothing here is only here". FEAT-013
	 * made that reading actionable: they can be deleted, one at a time from the
	 * row or all at once from the footer.
	 *
	 * A row that cannot be deleted keeps its button and says why in the title,
	 * rather than losing it. A control that disappears leaves the reader
	 * wondering whether they misremembered; one that explains itself does not.
	 *
	 * The columns were a hardcoded `grid-template-columns` until FEAT-047. They
	 * are the same store the graph uses now, so they drag and they come back
	 * that way. A divider sizes the column it sits on, which is the only model
	 * under which the boundary follows the pointer — the reasoning is written
	 * out once, in `GraphHeader.svelte`.
	 */

	const rows = $derived(branches.filtered);
	const shown = $derived(columns.shown);
	/** Both halves of every bar are scaled against this, so rows compare. */
	const max = $derived(widest(rows));

	/** The resize in progress: which column, and where it started. */
	let resizing: { id: ColumnId; startX: number; startWidth: number } | null = null;

	function startResize(event: PointerEvent, id: ColumnId) {
		event.preventDefault();
		event.stopPropagation();

		// Measured rather than read from the store: the filling column's stored
		// width is 0 until it is dragged, and starting from 0 would snap it to
		// its minimum before the pointer had moved.
		const handle = event.currentTarget as HTMLElement;
		const cell = handle.closest('.head')?.querySelector<HTMLElement>(`[data-column="${id}"]`);
		const startWidth = cell ? cell.getBoundingClientRect().width : columns.width(id);

		resizing = { id, startX: event.clientX, startWidth };
		handle.setPointerCapture(event.pointerId);
	}

	function moveResize(event: PointerEvent) {
		if (!resizing) return;
		columns.resize(resizing.id, resizing.startWidth + (event.clientX - resizing.startX));
	}

	function endResize(event: PointerEvent) {
		if (!resizing) return;
		(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
		resizing = null;
	}

	/** `width: Npx`, or nothing at all for the column that takes the rest. */
	function sizing(column: { width: number; fills?: boolean }): string {
		return column.fills ? '' : `width: ${column.width}px`;
	}
</script>

<div class="table" role="table" aria-label="Branches">
	<div class="head" role="row">
		{#each shown as column, index (column.id)}
			<span
				class="cell"
				class:fills={column.fills}
				class:drift={column.id === 'drift'}
				class:actions={column.id === 'actions'}
				data-column={column.id}
				style={sizing(column)}
				role="columnheader"
			>
				<span class="label">{column.label}</span>
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="divider"
					class:last={index === shown.length - 1}
					title={`Resize ${column.label} — double-click to reset`}
					onpointerdown={(event) => startResize(event, column.id)}
					onpointermove={moveResize}
					onpointerup={endResize}
					onpointercancel={endResize}
					ondblclick={() => columns.unsize(column.id)}
				></div>
			</span>
		{/each}
	</div>

	{#each rows as row (row.fullName)}
		<div
			class="row"
			class:merged={row.merged && !row.current}
			class:current={row.current}
			role="row"
		>
			{#each shown as column (column.id)}
				<div
					class="cell"
					class:fills={column.fills}
					class:drift={column.id === 'drift'}
					class:actions={column.id === 'actions'}
					style={sizing(column)}
					role="cell"
				>
					{#if column.id === 'name'}
						<span class="mono mark" aria-hidden="true">{row.current ? '✔' : ' '}</span>
						<span class="label" title={row.fullName}>{row.name}</span>
						{#if row.upstream}
							<span class="mono muted up" title="Tracks {row.upstream}">→ {row.upstream}</span>
						{/if}
					{:else if column.id === 'drift'}
						<DivergenceBar {row} {max} />
					{:else if column.id === 'when'}
						<span class="when note" title={row.summary}>
							{relativeTime(row.time)} · {row.authorName}
						</span>
					{:else if row.current}
						<span class="note">on this branch</span>
						<Chip title={undeletable(row) ?? undefined}>Delete</Chip>
					{:else if row.kind === 'branch'}
						<Btn disabled={branches.busy} onclick={() => branches.checkout(row.name)}>
							Check out
						</Btn>
						<Chip
							disabled={branches.busy}
							title="Rename {row.name}"
							onclick={() => renameBranch(row)}
						>
							Rename
						</Chip>
						<Chip
							disabled={branches.busy}
							danger={!row.merged}
							title={row.merged
								? `Delete ${row.name} — everything on it is already merged`
								: `Delete ${row.name} — it has commits that are on no other branch`}
							onclick={() => deleteBranch(row)}
						>
							Delete
						</Chip>
					{:else}
						<Btn
							disabled={branches.busy}
							title="Create a local branch that tracks {row.name}"
							onclick={() => {
								branches.setNewName(row.name.replace(/^[^/]+\//, ''));
								branches.setNewStart(row.name);
							}}
						>
							Branch from it
						</Btn>
						<Chip title={undeletable(row) ?? undefined}>Delete</Chip>
					{/if}
				</div>
			{/each}
		</div>
	{/each}

	{#if rows.length === 0}
		<p class="empty note">
			{branches.rows.length === 0
				? 'This repository has no branches yet.'
				: 'No branch matches those filters.'}
		</p>
	{/if}
</div>

<style>
	.table {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
	}

	.head,
	.row {
		display: flex;
		align-items: center;
		padding: 4px 12px;
	}

	.head {
		border-bottom: 1px solid var(--soft);
		font-size: var(--fs-secondary);
		color: var(--muted);
		position: sticky;
		top: 0;
		background: var(--bg);
		z-index: 1;
		user-select: none;
	}

	.cell {
		position: relative;
		display: flex;
		align-items: center;
		gap: 6px;
		padding-right: 10px;
		flex: none;
		min-width: 0;
	}

	.cell.fills {
		flex: 1;
		min-width: 0;
	}

	.cell.drift {
		gap: 8px;
	}

	.cell.actions {
		justify-content: flex-end;
		gap: 6px;
		padding-right: 0;
	}

	/*
	 * A row is a line, not a band (FEAT-084): the lists' type size, no rule
	 * under it, and a rounded highlight inset from the pane's edges — the same
	 * row the file lists and the graph draw. The inset comes out of the row's
	 * padding, so the cells stay under their headings.
	 */
	.row {
		min-height: 30px;
		margin: 0 6px;
		padding-inline: 6px;
		border-radius: var(--r-button);
		font-size: var(--fs-secondary);
		transition: background var(--t-fast) var(--ease);
	}

	/* Nothing on a merged branch is only there, so it reads as spent — dimmed
	   rather than drawn with a dashed rule, which said "unfinished". */
	.row.merged {
		color: var(--muted);
		opacity: 0.78;
	}

	/* The branch you are on: marked with flat selection surface. */
	.row.current {
		background: var(--selection);
	}

	.row:hover {
		background: var(--hover);
	}

	/*
	 * The actions show on the row being looked at — hovered or holding focus
	 * (FEAT-084). Two or three buttons on every one of a thousand rows was most
	 * of what made the table read as crowded. They stay in the row, so the
	 * keyboard reaches them and the row keeps its height.
	 */
	.row .cell.actions {
		opacity: 0;
		transition: opacity var(--t-fast) var(--ease);
	}

	.row:hover .cell.actions,
	.row:focus-within .cell.actions {
		opacity: 1;
	}

	.mark {
		flex: none;
		width: 10px;
		color: var(--accent);
	}

	.label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.up {
		flex: none;
	}

	.when {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/*
	 * The divider straddles the boundary, so it reads as belonging to both
	 * columns; the last one sits wholly inside its own, because half of it
	 * would otherwise be off the edge of the table.
	 */
	.divider {
		position: absolute;
		top: 0;
		right: -3px;
		width: 7px;
		height: 100%;
		cursor: col-resize;
		z-index: 1;
	}

	.divider.last {
		right: 0;
	}

	/* Drawn only while the header is pointed at (FEAT-084), as the graph's
	   are: at rest, space separates the columns. */
	.divider::after {
		content: '';
		position: absolute;
		left: 3px;
		top: 0;
		width: 1.5px;
		height: 100%;
		background: transparent;
		transition: background var(--t-fast) var(--ease);
	}

	.head:hover .divider::after {
		background: var(--soft);
	}

	.divider.last::after {
		left: auto;
		right: 0;
	}

	/* Same hint the panel splitters give — a hairline, never the accent. */
	.divider:hover::after {
		background: var(--line);
	}

	.empty {
		padding: 14px 12px;
	}
</style>
