<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { changes } from '$lib/changes/store.svelte';
	import { discardAll, discardPaths } from '$lib/changes/discard';
	import FileName from '$lib/ui/FileName.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import type { DiffSide, StatusEntry } from '$lib/types';

	/**
	 * What is staged and what is not, in one column.
	 *
	 * A path can appear in both sections at once, because a file can be staged
	 * in part; collapsing that into one row is what makes people commit
	 * something they did not mean to. The classes `solid` and `dashed` still
	 * name the two sides, which is what the tests and the markup call them.
	 *
	 * Discard is offered on the unstaged side only (FEAT-048). That is the side
	 * where the change has not been kept yet, and it is the side whose contents
	 * `git restore --worktree` is defined against.
	 *
	 * **Rows you can read** (TASK-046). Each row is the file's name and then its
	 * folder, with a lettered badge — no box round it, and its actions shown
	 * while the row is hovered, focused or selected. Fourteen files used to be
	 * fourteen bordered boxes, each with a `+` and a `✕` beside a path cut from
	 * its start.
	 */

	const work = $derived(changes.work);

	function selected(entry: StatusEntry, side: DiffSide): boolean {
		return changes.selection?.path === entry.path && changes.selection?.side === side;
	}

	function paths(entries: StatusEntry[]): string[] {
		return entries.map((entry) => entry.path);
	}
</script>

<div class="column">
	{#if work.conflicted.length > 0}
		<section class="group">
			<header class="head">
				<h3 class="title">Conflicts</h3>
				<span class="count mono">{work.conflicted.length}</span>
			</header>
			<p class="note explain">Nothing can be committed until these are resolved.</p>
			{#each work.conflicted as entry (entry.path)}
				<div class="row conflicted" title={entry.path}>
					<span class="open">
						<FileName path={entry.path} status="conflicted" />
					</span>
				</div>
			{/each}
		</section>
	{/if}

	<section class="group">
		<header class="head">
			<h3 class="title">Staged</h3>
			<span class="count mono">{work.staged.length}</span>
			<span class="spacer"></span>
			{#if work.staged.length > 0}
				<button
					class="text-action"
					disabled={changes.busy}
					onclick={() => changes.unstage(paths(work.staged))}
				>
					Unstage all
				</button>
			{/if}
		</header>

		{#each work.staged as entry (entry.path)}
			<div class="row solid" class:selected={selected(entry, 'staged')}>
				<button
					class="open"
					onclick={() => changes.open({ path: entry.path, side: 'staged' })}
					title={entry.path}
				>
					<FileName path={entry.path} status={entry.status} />
				</button>
				<button
					class="act"
					disabled={changes.busy}
					title="Unstage {entry.path}"
					aria-label="Unstage {entry.path}"
					onclick={() => changes.unstage([entry.path])}
				>
					<Icon name="minimize" size="1em" weight={2.2} />
				</button>
			</div>
		{/each}

		{#if work.staged.length === 0}
			<p class="note empty">Nothing staged yet.</p>
		{/if}
	</section>

	<section class="group">
		<header class="head">
			<h3 class="title">Unstaged</h3>
			<span class="count mono">{work.unstaged.length}</span>
			<span class="spacer"></span>
			{#if work.unstaged.length > 0}
				<button
					class="text-action"
					disabled={changes.busy}
					onclick={() => changes.stage(paths(work.unstaged))}
				>
					Stage all
				</button>
				<button
					class="text-action danger"
					disabled={changes.busy}
					title="Throw away every unstaged change — this cannot be undone"
					onclick={() => discardAll()}
				>
					Discard all
				</button>
			{/if}
		</header>

		{#each work.unstaged as entry (entry.path)}
			<div class="row dashed" class:selected={selected(entry, 'unstaged')}>
				<button
					class="open"
					onclick={() => changes.open({ path: entry.path, side: 'unstaged' })}
					title={entry.path}
				>
					<FileName path={entry.path} status={entry.status} />
				</button>
				<button
					class="act stage"
					disabled={changes.busy}
					title="Stage {entry.path}"
					aria-label="Stage {entry.path}"
					onclick={() => changes.stage([entry.path])}
				>
					<Icon name="plus" size="1em" weight={2.2} />
				</button>
				<button
					class="act discard"
					disabled={changes.busy}
					title={entry.status === 'untracked'
						? `Delete ${entry.path} — this cannot be undone`
						: `Discard changes to ${entry.path} — this cannot be undone`}
					aria-label="Discard {entry.path}"
					onclick={() => discardPaths([entry])}
				>
					<Icon name="undo" size="1em" weight={2} />
				</button>
			</div>
		{/each}

		{#if work.unstaged.length === 0}
			<p class="note empty">Nothing unstaged.</p>
		{/if}
	</section>
</div>

<style>
	/*
	 * The column sits on the pane (TASK-046): no fill of its own and no rule
	 * down its edge — the diff beside it is a card, and space is what
	 * separates them.
	 */
	.column {
		width: var(--changes-files-w);
		flex: none;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 10px 6px 12px 8px;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	/* A section's heading: its name, how many, and what can be done to all of
	   them, as quiet text buttons. */
	.head {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 6px 6px 8px;
	}

	.title {
		margin: 0;
		font-size: var(--fs-secondary);
		font-weight: 650;
		color: var(--ink);
	}

	.count {
		min-width: 22px;
		padding: 1px 7px;
		border-radius: var(--r-pill);
		text-align: center;
		color: var(--muted);
		background: var(--soft);
	}

	.spacer {
		flex: 1;
	}

	.text-action {
		padding: 3px 8px;
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		font-weight: 600;
		color: var(--accent);
		transition: background var(--t-fast) var(--ease);
	}

	.text-action:hover:not(:disabled) {
		background: var(--accent-soft);
	}

	.text-action.danger {
		color: var(--muted);
	}

	.text-action.danger:hover:not(:disabled) {
		color: var(--danger);
		background: var(--danger-soft);
	}

	.text-action:disabled {
		opacity: 0.45;
	}

	.explain,
	.empty {
		margin: 0;
		padding: 2px 8px 4px;
	}

	/* A row is a line, not a box: a rounded highlight under the pointer, a
	   tinted one when it is the file on show. */
	.row {
		display: flex;
		align-items: center;
		gap: 2px;
		min-height: 34px;
		padding: 0 4px 0 0;
		border-radius: var(--r-button);
		min-width: 0;
		transition: background var(--t-fast) var(--ease);
	}

	.row:hover {
		background: var(--hover);
	}

	.row.selected {
		background: var(--selection);
	}

	.row.conflicted {
		background: var(--danger-soft);
	}

	.open {
		display: flex;
		align-items: center;
		flex: 1;
		min-width: 0;
		align-self: stretch;
		padding: 0 6px 0 8px;
		text-align: left;
	}

	/*
	 * The actions: round, and only while the row is being looked at — hovered,
	 * holding focus, or on show. A column of permanent glyphs beside every file
	 * was most of what made the list read as crowded.
	 */
	.act {
		flex: none;
		display: inline-grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border-radius: 50%;
		color: var(--muted);
		opacity: 0;
		transition:
			opacity var(--t-fast) var(--ease),
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	.row:hover .act,
	.row:focus-within .act,
	.row.selected .act {
		opacity: 1;
	}

	.act:hover:not(:disabled) {
		color: var(--accent);
		background: var(--accent-soft);
	}

	/* Red under the pointer only: a column of red would read as a list of
	   errors rather than a column of available actions. */
	.act.discard:hover:not(:disabled) {
		color: var(--danger);
		background: var(--danger-soft);
	}

	.act:disabled {
		opacity: 0.3;
	}
</style>
