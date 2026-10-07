<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import BlameStrip from './BlameStrip.svelte';
	import QueryBar from './QueryBar.svelte';
	import ResultDetail from './ResultDetail.svelte';
	import ResultRows from './ResultRows.svelte';
	import { search } from './store.svelte';

	/**
	 * Log (1I), in the house style (TASK-057): the head every designed screen
	 * has, the question on a card of its own, the results as a list of commits
	 * you can read, and the opened commit and its blame beside them, each on a
	 * card. It used to be three bars and two bordered columns that looked
	 * borrowed from somewhere else.
	 */
	interface Props {
		/** Focus the first field, when the screen was reached by its shortcut. */
		autofocus?: boolean;
		/** `Alt+Enter` — the commit's hunks, on the Diff screen. */
		ondiff?: (id: string) => void;
	}

	let { autofocus = false, ondiff }: Props = $props();

	const status = $derived.by((): { tone: string; text: string } | null => {
		if (search.running) return { tone: 'busy', text: `Walking · ${search.count} found` };
		if (search.error) return { tone: 'bad', text: 'The walk failed' };
		if (!search.ran) return null;
		return { tone: 'ok', text: `${search.count} ${search.count === 1 ? 'result' : 'results'}` };
	});
</script>

<div class="screen">
	<div class="page">
		<header class="head">
			<span class="tile" aria-hidden="true"><Icon name="search" size="1.25em" weight={1.9} /></span>
			<div class="titles">
				<h1 class="title">Log</h1>
				<span class="note">Find commits by who, what, where and when.</span>
			</div>
			{#if status}
				<span class="status tone-{status.tone}"><span class="pip" aria-hidden="true"></span>{status.text}</span>
			{/if}
		</header>

		<section class="card query" aria-label="Search">
			<QueryBar {autofocus} />
		</section>

		<div class="body">
			<section class="card results" aria-label="Results">
				<div class="card-head">
					<span class="label">Results</span>
					{#if search.ran && search.count > 0}
						<span class="note small">↵ to read · Alt+↵ for the diff</span>
					{/if}
				</div>
				{#if search.error}
					<p class="note error">{search.error}</p>
				{:else if !search.ran}
					<div class="empty">
						<span class="empty-mark" aria-hidden="true"><Icon name="history" size="1.6em" weight={1.6} /></span>
						<span class="note">Ask above. Matches appear as history is walked.</span>
					</div>
				{:else if search.count === 0 && !search.running}
					<div class="empty">
						<span class="empty-mark" aria-hidden="true"><Icon name="search" size="1.6em" weight={1.6} /></span>
						<span class="note">
							Nothing matched{#if search.narrowestApplied}, and
								<span class="mono">{search.narrowestApplied}</span> is the narrowest filter applied{/if}.
						</span>
					</div>
				{:else}
					<ResultRows onopen={(id) => search.select(id)} {ondiff} />
					{#if search.running}
						<Loader size="inline" label="Still walking…" />
					{/if}
				{/if}
			</section>

			<aside class="side">
				<section class="card detail-card" aria-label="Commit">
					<span class="label">Commit</span>
					<ResultDetail {ondiff} />
				</section>
				<section class="card blame-card" aria-label="Blame">
					<span class="label">Blame</span>
					<BlameStrip />
				</section>
			</aside>
		</div>
	</div>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.page {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 16px 20px 88px;
	}

	.head,
	.query {
		flex: none;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}

	.tile {
		width: 38px;
		height: 38px;
		border-radius: 12px;
		display: grid;
		place-items: center;
		background: color-mix(in srgb, var(--accent) 16%, transparent);
		color: var(--accent);
		flex: none;
	}

	.titles {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1 1 320px;
		min-width: 0;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.status {
		--tone: var(--ok);
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--tone) 45%, transparent);
		color: var(--tone);
		font-size: var(--fs-secondary);
		background-color: var(--surface-veil);
		white-space: nowrap;
	}

	.status.tone-busy {
		--tone: var(--accent);
	}

	.status.tone-bad {
		--tone: var(--danger);
	}

	.pip {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--tone);
	}

	.query {
		padding: 14px 16px;
		border-radius: 18px;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 14px;
	}

	.results {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 14px;
		border-radius: 16px;
		overflow: auto;
	}

	.card-head {
		display: flex;
		align-items: baseline;
		gap: 10px;
		padding: 0 4px;
	}

	.label {
		font-size: var(--fs-mono);
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 600;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 32px 16px;
		text-align: center;
	}

	.empty-mark {
		width: 56px;
		height: 56px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		color: var(--muted);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
	}

	.side {
		width: var(--search-side-w);
		flex: none;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.detail-card,
	.blame-card {
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 14px;
		border-radius: 16px;
	}

	.detail-card {
		flex: 0 1 auto;
		max-height: 62%;
	}

	.blame-card {
		flex: 1 1 0;
	}

	.error {
		color: var(--danger);
	}
</style>
