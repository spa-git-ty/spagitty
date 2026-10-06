<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import RefTabs from '$lib/branches/RefTabs.svelte';
	import { untrack } from 'svelte';
	import BranchTable from '$lib/branches/BranchTable.svelte';
	import { branches, FILTERS } from '$lib/branches/store.svelte';
	import { columns } from '$lib/branches/columns.svelte';
	import { deleteMerged, mergedBranches } from '$lib/branches/actions';
	import { relativeTime } from '$lib/format';
	import { network } from '$lib/network/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * Every branch, how far it has drifted, and what is safe to forget.
	 *
	 * Since FEAT-013 this screen does remove things. Everything destructive here
	 * asks first, and the bulk cleanup asks with the whole list in the question:
	 * it is the one operation on the screen that touches refs the user did not
	 * point at individually, and a count is a thing people agree to where a list
	 * of names is a thing people read.
	 */

	let generation: number | null = null;
	$effect(() => {
		const current = repo.generation;
		if (repo.info === null) return;
		if (generation === current) return;

		generation = current;
		untrack(() => {
			// The column layout is per repository, so it is pointed at the new
			// one before the table paints rather than after.
			columns.open(repo.info?.path ?? null);
			branches.clear();
			branches.load();
		});
	});

	const counts = $derived.by(() => {
		const rows = branches.rows;
		const local = rows.filter((r) => r.kind === 'branch').length;
		const remote = rows.length - local;
		return remote === 0
			? `${local} ${local === 1 ? 'branch' : 'branches'}`
			: `${local} local · ${remote} remote-tracking`;
	});

	/** Whether any counts on screen could be out of date. */
	const anyUpstream = $derived(branches.rows.some((row) => row.upstream !== null));

	/**
	 * How old the ahead/behind numbers are (FEAT-018).
	 *
	 * They are computed against remote-tracking refs, which only move when
	 * something fetches — so a divergence bar on a repository nobody has
	 * fetched for a week is a week out of date and looks exactly as confident
	 * as one from a minute ago. Only said when a branch actually tracks
	 * something: with no upstreams there is nothing here that a fetch changes.
	 */
	const staleness = $derived.by(() => {
		if (!anyUpstream || repo.info === null) return null;
		const at = repo.info.lastFetched;
		return at === null ? 'never fetched' : `as of ${relativeTime(at)}`;
	});

	/** Local, merged, and not the one you are standing on. */
	const merged = $derived(mergedBranches(branches.rows));
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<RefTabs />
			{#if branches.loaded}<span class="note">{counts}</span>{/if}
			{#if staleness}
				<span
					class="note"
					title="Ahead and behind are counted against what the last fetch brought down"
				>
					· drift {staleness}
				</span>
			{/if}
		</div>
		<div class="right">
			{#if network.running}<span class="note">{network.label}</span>{/if}
			{#if branches.loading}<Loader size="inline" label="Reading…" />{/if}
			<Btn
				disabled={branches.busy || network.running}
				title="Bring the remote-tracking refs up to date, so the drift is current"
				onclick={() => network.fetch()}
			>
				Fetch
			</Btn>
			<Btn disabled={branches.busy} onclick={() => branches.load()}>Refresh</Btn>
		</div>
	</header>

	{#if repo.info === null}
		<div class="empty"><p class="note">No repository open.</p></div>
	{:else if branches.error}
		<div class="empty"><p class="note error">{branches.error}</p></div>
	{:else if !branches.loaded}
		<div class="empty"><Loader label="Reading the branches…" /></div>
	{:else}
		<div class="filters">
			<input
				class="field"
				type="text"
				placeholder="Filter by name"
				value={branches.query}
				oninput={(event) => branches.setQuery(event.currentTarget.value)}
				aria-label="Filter branches by name"
			/>
			{#each FILTERS as filter (filter)}
				<Chip
					active={branches.active.includes(filter)}
					onclick={() => branches.toggle(filter)}
					title={filter === 'mine'
						? 'Local branches — not "authored by me", which Spagitty cannot know yet'
						: undefined}
				>
					{filter}
				</Chip>
			{/each}
			{#if branches.hidden > 0}
				<span class="note">{branches.hidden} hidden</span>
				<Chip onclick={() => branches.clearFilters()}>clear</Chip>
			{/if}
		</div>

		<BranchTable />

		{#if merged.length > 0}
			<div class="cleanup">
				<span class="note">
					{merged.length} merged {merged.length === 1 ? 'branch' : 'branches'} — nothing on
					{merged.length === 1 ? 'it is' : 'them is'} only there
				</span>
				<Btn
					disabled={branches.busy}
					title="Delete every merged branch, after showing you the list"
					onclick={() => deleteMerged(branches.rows)}
				>
					Delete merged
				</Btn>
			</div>
		{/if}

		<div class="create">
			<span class="note">New branch</span>
			<input
				class="field name"
				type="text"
				placeholder="name"
				value={branches.newName}
				oninput={(event) => branches.setNewName(event.currentTarget.value)}
				aria-label="New branch name"
			/>
			<span class="note">from</span>
			<input
				class="field start"
				type="text"
				placeholder={branches.current?.name ?? 'HEAD'}
				value={branches.newStart}
				oninput={(event) => branches.setNewStart(event.currentTarget.value)}
				aria-label="Start point for the new branch"
			/>
			<Chip
				active={branches.newCheckout}
				onclick={() => branches.setNewCheckout(!branches.newCheckout)}
			>
				check it out
			</Chip>
			<Btn
				primary
				disabled={branches.busy || branches.newName.trim() === ''}
				onclick={() => branches.create()}
			>
				Create
			</Btn>
		</div>
	{/if}

	<!--
		Only a failure gets a footer, the way Stash and Settings now work.

		The sentence that used to sit here — "Nothing here deletes a branch." —
		was true, and still went: it announced that the screen does not do
		something, which is precisely the copy TASK-007 removed everywhere else.
		It was also saying it in the wrong place. The Delete chip carries its own
		reason in its title, at the control it is about, where someone wondering
		will actually look; a strip along the bottom of the screen is where you
		put it if you want it read by nobody.
	-->
	{#if branches.writeError}
		<footer class="foot">
			<span class="note error">{branches.writeError}</span>
		</footer>
	{/if}
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
	.cleanup,
	.filters,
	.create {
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

	.filters {
		padding: 8px 12px;
		flex-wrap: wrap;
	}

	.create {
		padding: 8px 12px;
		border-top: 1px solid var(--soft);
	}

	/* Above the create row, because it is about what is already there rather
	   than about what comes next, and it only exists when there is something
	   to clean up. */
	.cleanup {
		padding: 8px 12px;
		border-top: 1px solid var(--soft);
		justify-content: space-between;
	}

	.foot {
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-top: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}


	.field {
		background: transparent;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		padding: 3px 6px;
	}

	.field:focus {
		outline: none;
		border-color: var(--accent);
	}

	.field::placeholder {
		color: var(--placeholder);
	}

	.filters .field {
		width: 220px;
	}

	.name {
		width: 200px;
	}

	.start {
		width: 160px;
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 0 20px;
		text-align: center;
	}

	.error {
		color: var(--danger);
	}
</style>
