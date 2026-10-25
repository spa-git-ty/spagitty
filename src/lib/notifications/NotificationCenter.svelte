<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { repo } from '$lib/repo.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { speedSearch } from '$lib/ui/speedsearch.svelte';
	import { act, follow } from './act';
	import { center } from './center.svelte';
	import {
		FILTERS,
		actionOf,
		groups,
		timeLabel,
		unread,
		unreadCount,
		visible,
		type Entry,
		type Filter
	} from './center';
	import { styleOf } from './kinds';

	/**
	 * The notification center (FEAT-142): a drawer from the right, over any
	 * screen, that keeps everything Spagitty has said.
	 *
	 * Mounted once, in the layout, so it opens over whatever screen is up. The
	 * bell in the title bar opens it; Escape, the close button and the dimmed
	 * window around it close it.
	 */

	let filter = $state<Filter>('all');
	let onlyHere = $state(false);
	let search = $state('');
	let now = $state(Date.now());
	let panel = $state<HTMLElement | null>(null);

	const here = $derived(repo.info?.path ?? null);
	const shown = $derived(
		visible(center.entries, {
			filter,
			repository: onlyHere ? here : null,
			search
		})
	);
	const days = $derived(groups(shown, now));
	const fresh = $derived(unreadCount(center.entries, onlyHere ? here : null));

	// The times on the rows are relative; they move on while the drawer is open.
	$effect(() => {
		if (!center.open) return;
		now = Date.now();
		const clock = setInterval(() => (now = Date.now()), 30_000);
		void tick().then(() => panel?.focus());
		return () => clearInterval(clock);
	});

	function onkey(event: KeyboardEvent) {
		if (center.open && event.key === 'Escape') {
			event.preventDefault();
			center.hide();
		}
	}

	/** Opening an item marks it read and goes where it is about. */
	function open(entry: Entry) {
		void center.read([entry.id]);
		center.hide();
		follow(entry.target);
	}

	async function run(entry: Entry) {
		void center.read([entry.id]);
		center.hide();
		await act(entry.kind, entry.target);
	}

	function settings() {
		center.hide();
		void goto('/settings#notifications');
	}
</script>

<svelte:window onkeydown={onkey} />

{#if center.open}
	<div class="scrim" aria-hidden="true" onclick={() => center.hide()}></div>
	<div
		class="drawer ornament"
		role="dialog"
		aria-modal="true"
		aria-labelledby="notifications-title"
		tabindex="-1"
		bind:this={panel}
		use:speedSearch={{ field: () => panel?.querySelector<HTMLInputElement>('.search input') }}
	>
		<header class="head">
			<div class="title-row">
				<div class="title">
					<h2 id="notifications-title">Notifications</h2>
					<span class="fresh">{fresh ? `${fresh} new` : 'all read'}</span>
				</div>
				<div class="tools">
					<button
						class="text-button"
						disabled={fresh === 0}
						onclick={() => center.readAll(onlyHere ? here : null)}>Mark all read</button
					>
					<button class="round" aria-label="Notification settings" title="Notification settings" onclick={settings}>
						<Icon name="settings" size="1.05em" />
					</button>
					<button class="round" aria-label="Close" title="Close" onclick={() => center.hide()}>
						<Icon name="close" size="1.05em" />
					</button>
				</div>
			</div>

			<div class="find-row">
				<label class="search">
					<Icon name="search" size="0.95em" />
					<input type="search" aria-label="Search notifications" placeholder="Search notifications" bind:value={search} />
				</label>
				<div class="scope" role="radiogroup" aria-label="Which repositories">
					<button role="radio" aria-checked={!onlyHere} class:on={!onlyHere} onclick={() => (onlyHere = false)}>All repos</button>
					<button
						role="radio"
						aria-checked={onlyHere}
						class:on={onlyHere}
						disabled={!here}
						title={here ?? 'No repository is open'}
						onclick={() => (onlyHere = true)}>{repo.info?.name ?? 'This repo'}</button
					>
				</div>
			</div>

			<div class="chips" role="group" aria-label="Kind">
				{#each FILTERS as chip (chip.id)}
					<Chip active={filter === chip.id} onclick={() => (filter = chip.id)}>
						{chip.label}
						{#if chip.id === 'unread' && fresh > 0}<span class="count">{fresh}</span>{/if}
					</Chip>
				{/each}
			</div>
		</header>

		<div class="list">
			{#each days as day (day.label)}
				<div class="day">
					<div class="day-label">{day.label}</div>
					{#each day.entries as entry (entry.id)}
						{@const style = styleOf(entry.kind)}
						<div class="row" class:unread={unread(entry)} style:--kind={style.colour}>
							<span class="tile"><Icon name={style.icon} size="1.05em" /></span>
							<div class="text">
								<div class="line">
									<button class="open" onclick={() => open(entry)}>
										<span class="what">{entry.title}</span>
									</button>
									<span class="time">{timeLabel(entry.createdMs, now)}</span>
								</div>
								{#if entry.body}<span class="body">{entry.body}</span>{/if}
								<div class="foot">
									{#if entry.repositoryName}<span class="repo mono">{entry.repositoryName}</span>{:else}<span></span>{/if}
									<div class="actions">
										{#if unread(entry)}
											<button class="text-button small" onclick={() => center.read([entry.id])}>Mark read</button>
										{/if}
										<button class="act" onclick={() => run(entry)}>{actionOf(entry)}</button>
									</div>
								</div>
							</div>
							<span class="dot" aria-label={unread(entry) ? 'Unread' : undefined}></span>
						</div>
					{/each}
				</div>
			{:else}
				<p class="empty">Nothing here. You’re caught up.</p>
			{/each}
			{#if days.length}
				<p class="kept">Every notification is kept · scroll for older</p>
			{/if}
		</div>
	</div>
{/if}

<style>
	/* The window dims under the title bar, which stays usable: the bell is the
	   way back out. */
	.scrim {
		position: fixed;
		inset: var(--titlebar-h) 0 0 0;
		z-index: 40;
		background: color-mix(in srgb, var(--bg) 55%, transparent);
	}

	.drawer {
		position: fixed;
		top: calc(var(--titlebar-h) + 2px);
		right: 16px;
		bottom: 16px;
		z-index: 41;
		width: min(470px, calc(100% - 32px));
		display: flex;
		flex-direction: column;
		border-radius: var(--r-ornament);
		outline: none;
		overflow: hidden;
	}

	.head {
		padding: 18px 18px 12px 22px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.title-row,
	.find-row,
	.title,
	.tools {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.title-row {
		justify-content: space-between;
	}

	.title {
		align-items: baseline;
		gap: 10px;
	}

	h2 {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.fresh {
		color: var(--accent);
		font-size: var(--fs-secondary);
	}

	.text-button {
		height: 32px;
		padding: 0 12px;
		border: 0;
		border-radius: var(--r-pill);
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: var(--fs-secondary);
		cursor: pointer;
	}

	.text-button.small {
		height: 28px;
		padding: 0 10px;
	}

	.text-button:hover:not(:disabled),
	.round:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.text-button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.round {
		width: 32px;
		height: 32px;
		display: grid;
		place-items: center;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: var(--muted);
		cursor: pointer;
	}

	.search {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		height: 36px;
		padding: 0 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		background: var(--sunken);
		color: var(--muted);
	}

	.search input {
		flex: 1;
		min-width: 0;
		border: 0;
		padding: 0;
		background: transparent;
		color: var(--ink);
		outline: none;
		font-size: var(--fs-secondary);
	}

	.scope {
		display: flex;
		padding: 3px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		background: var(--sunken);
	}

	.scope button {
		height: 28px;
		max-width: 130px;
		padding: 0 11px;
		border: 0;
		border-radius: var(--r-pill);
		background: transparent;
		color: var(--muted);
		font: inherit;
		font-size: var(--fs-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}

	.scope button.on {
		background: var(--hover);
		color: var(--ink);
		font-weight: 600;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.count {
		margin-left: 4px;
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		display: inline-grid;
		place-items: center;
		border-radius: var(--r-pill);
		background: var(--accent);
		color: var(--on-accent);
		font-size: calc(var(--fs-mono) * 0.92);
		font-weight: 700;
	}

	.list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 0 10px 10px;
	}

	.day {
		display: flex;
		flex-direction: column;
	}

	/* Sticky over the rows it heads; the drawer's own colour behind it so the
	   rows scrolling under it do not show through. */
	.day-label {
		position: sticky;
		top: 0;
		z-index: 1;
		padding: 12px 12px 6px;
		color: var(--muted);
		font-size: var(--fs-mono);
		font-weight: 600;
		letter-spacing: 0.02em;
		background: color-mix(in srgb, var(--surface-2) 96%, transparent);
	}

	.row {
		display: grid;
		grid-template-columns: 36px minmax(0, 1fr) 8px;
		gap: 12px;
		align-items: start;
		padding: 12px;
		border-radius: var(--r-floating);
	}

	.row.unread {
		background: color-mix(in srgb, var(--accent) 5%, transparent);
	}

	.row:hover,
	.row:focus-within {
		background: var(--hover);
	}

	.tile {
		width: 36px;
		height: 36px;
		display: grid;
		place-items: center;
		border-radius: 11px;
		color: var(--kind);
		background: color-mix(in srgb, var(--kind) 14%, transparent);
	}

	.text {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.line {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 10px;
	}

	.open {
		min-width: 0;
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.what {
		display: block;
		font-size: var(--fs-ui);
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		color: color-mix(in srgb, var(--ink) 88%, transparent);
	}

	.unread .what {
		font-weight: 600;
		color: var(--ink);
	}

	.open:hover .what {
		text-decoration: underline;
	}

	.time {
		flex: none;
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.body {
		color: var(--muted);
		font-size: var(--fs-secondary);
		line-height: 1.4;
	}

	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		min-height: 28px;
		margin-top: 3px;
	}

	.repo {
		padding: 2px 8px;
		border-radius: 6px;
		background: var(--soft);
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	/* On hover or focus only: a column of buttons down the drawer is louder
	   than the notifications they act on. */
	.actions {
		display: flex;
		gap: 4px;
		opacity: 0;
		transition: opacity var(--t-fast) var(--ease);
	}

	.row:hover .actions,
	.row:focus-within .actions {
		opacity: 1;
	}

	.act {
		height: 28px;
		padding: 0 12px;
		border: 0;
		border-radius: var(--r-pill);
		background: var(--accent);
		color: var(--on-accent);
		font: inherit;
		font-size: var(--fs-secondary);
		font-weight: 600;
		cursor: pointer;
	}

	.dot {
		width: 8px;
		height: 8px;
		margin-top: 6px;
		border-radius: 50%;
	}

	.unread .dot {
		background: var(--accent);
	}

	.empty {
		padding: 48px 20px;
		text-align: center;
		color: var(--muted);
	}

	.kept {
		padding: 18px 12px 8px;
		text-align: center;
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	@media (prefers-reduced-motion: reduce) {
		.actions {
			transition: none;
		}
	}
</style>
