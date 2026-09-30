<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import RefTabs from '$lib/branches/RefTabs.svelte';
	import { onMount, untrack } from 'svelte';
	import * as api from '$lib/api';
	import { relativeTime } from '$lib/format';
	import { branchHere, checkoutHere, resetHere } from '$lib/reflog/actions';
	import { reflog } from '$lib/reflog/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * Where a ref has been, and the ways back to it (FEAT-050).
	 *
	 * Every other screen answers what history looks like. This one answers what
	 * you just did to it, which is the only question that helps after a rewrite
	 * goes wrong — and since FEAT-015 and FEAT-016, rewriting is something
	 * Spagitty does rather than something you go to a terminal for.
	 *
	 * Each row carries the revision it would be typed as, `HEAD@{3}`, because
	 * half the value of a reflog screen is knowing what to type when you are
	 * somewhere it cannot reach.
	 */

	let generation: number | null = null;
	$effect(() => {
		const current = repo.generation;
		if (repo.info === null) return;
		if (generation === current) return;

		generation = current;
		untrack(() => {
			reflog.clear();
			reflog.load();
		});
	});

	onMount(() => {
		if (api.inTauri() && repo.info) reflog.load();
		return () => reflog.clear();
	});

	const entries = $derived(reflog.entries);
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<RefTabs />
			{#if reflog.log}
				<span class="note">
					{reflog.log.entries.length}
					{reflog.log.entries.length === 1 ? 'move' : 'moves'} of
					<span class="mono">{reflog.log.reference}</span>
				</span>
			{/if}
		</div>
		<div class="right">
			{#if reflog.loading}<span class="note">Reading…</span>{/if}
			<input
				class="field"
				type="text"
				placeholder="Filter by operation or message"
				spellcheck="false"
				aria-label="Filter the reflog"
				value={reflog.query}
				oninput={(event) => reflog.setQuery(event.currentTarget.value)}
			/>
			<Btn disabled={reflog.busy} onclick={() => reflog.load()}>Refresh</Btn>
		</div>
	</header>

	{#if reflog.refs.length > 1}
		<div class="refs">
			{#each reflog.refs as name (name)}
				<Chip
					active={name === 'HEAD' ? reflog.reference === '' : reflog.reference.endsWith(name)}
					disabled={reflog.busy}
					title={name === 'HEAD'
						? 'Everything that moved HEAD, checkouts included'
						: `What happened to ${name}`}
					onclick={() => reflog.show(name === 'HEAD' ? '' : `refs/heads/${name}`)}
				>
					{name}
				</Chip>
			{/each}
		</div>
	{/if}

	<div class="body">
		{#if repo.info === null}
			<div class="empty"><p class="note">No repository open.</p></div>
		{:else if reflog.error}
			<div class="empty"><p class="note error">{reflog.error}</p></div>
		{:else if !reflog.loaded}
			<div class="empty"><span class="note">Reading…</span></div>
		{:else if reflog.absent}
			<div class="empty">
				<p class="note">This ref has no reflog.</p>
				<p class="note">
					A repository with <span class="mono">core.logAllRefUpdates</span> turned off keeps
					none, and a ref that has never moved has nothing to record yet.
				</p>
			</div>
		{:else if entries.length === 0}
			<div class="empty">
				<p class="note">
					{reflog.hidden > 0 ? 'Nothing matches that filter.' : 'Nothing has moved this ref.'}
				</p>
			</div>
		{:else}
			<div class="table" role="table" aria-label="Reflog">
				{#each entries as entry (entry.revision)}
					<div class="row" role="row">
						<span class="mono revision" role="cell" title="Type this to reach it">
							{entry.revision}
						</span>
						<span class="op" role="cell">{entry.operation}</span>
						<span class="mono ids" role="cell">
							{#if entry.created}
								<span class="note">created at</span>
							{:else}
								{entry.beforeShort} →
							{/if}
							{entry.afterShort}
						</span>
						<span class="message" role="cell" title={entry.message}>{entry.message}</span>
						<span class="note when" role="cell" title={entry.authorName}>
							{relativeTime(entry.time)}
						</span>
						<span class="acts" role="cell">
							<Chip
								disabled={reflog.busy}
								title="Create a branch at {entry.afterShort} — nothing else moves"
								onclick={() => branchHere(entry)}
							>
								branch here
							</Chip>
							<Chip
								disabled={reflog.busy}
								title="Check out {entry.afterShort} with no branch attached"
								onclick={() => checkoutHere(entry)}
							>
								check out
							</Chip>
							<Chip
								danger
								disabled={reflog.busy}
								title="Move this branch to {entry.afterShort} and discard the working tree"
								onclick={() => resetHere(entry)}
							>
								reset here
							</Chip>
						</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<footer class="foot">
		{#if reflog.writeError}
			<span class="note error">{reflog.writeError}</span>
		{:else if reflog.log?.truncated}
			<span class="note">
				The newest {reflog.log.entries.length} moves. A reflog older than that is what
				<span class="mono">git reflog</span> is for.
			</span>
		{:else if reflog.hidden > 0}
			<span class="note">{reflog.hidden} hidden by the filter</span>
		{:else}
			<span class="note">
				Git expires unreachable entries after 30 days. Anything here is still reachable
				until then, whatever the graph shows.
			</span>
		{/if}
	</footer>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head,
	.foot,
	.refs {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.head {
		justify-content: space-between;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.refs {
		padding: 4px 12px 8px;
		flex-wrap: wrap;
	}

	.field {
		font: inherit;
		font-size: var(--fs-ui);
		width: 220px;
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}

	/*
	 * A line, not a band (FEAT-084): the lists' type size, no rule, and a
	 * rounded highlight inset from the pane's edges.
	 */
	.row {
		display: grid;
		grid-template-columns: 90px 90px 150px minmax(0, 1fr) 90px auto;
		align-items: center;
		gap: 10px;
		margin: 0 6px;
		padding: 4px 6px;
		border-radius: var(--r-button);
		min-height: 30px;
		font-size: var(--fs-secondary);
		transition: background var(--t-fast) var(--ease);
	}

	.row:hover {
		background: var(--hover);
	}

	/* The revision is the thing to copy out, so it leads the row rather than
	   sitting at the end of it. */
	.revision {
		color: var(--accent);
	}

	.op,
	.message,
	.when {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ids {
		color: var(--muted);
	}

	/* On the row being looked at (FEAT-084), as the file lists' are. */
	.acts {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
		opacity: 0;
		transition: opacity var(--t-fast) var(--ease);
	}

	.row:hover .acts,
	.row:focus-within .acts {
		opacity: 1;
	}

	.empty {
		padding: 14px 12px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.foot {
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-top: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	p {
		margin: 0;
	}
</style>
