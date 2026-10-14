<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { fullDate } from '$lib/format';
	import { graph } from '$lib/graph/store.svelte';
	import type { Hunk } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { byOf, dayOf, inkOf, KIND_INK } from './chart';
	import { investigate } from './store.svelte';

	/**
	 * One event, looked at closely (FEAT-132).
	 *
	 * For a file: how much of it the commit changed, how many of the lines it
	 * wrote are still there today, who owns the file now, and the largest
	 * thing it did. Then three ways on — the file as it was, the file then
	 * against the file today, and the commit on the Graph.
	 */
	const entry = $derived(investigate.selected);
	const target = $derived(investigate.target);
	const today = $derived(investigate.today);
	const isFile = $derived(target?.kind === 'file' || target?.kind === 'line');

	const hunks = $derived.by((): Hunk[] | null | undefined => {
		if (investigate.comparing) return investigate.compared ?? undefined;
		if (investigate.hunk === undefined) return undefined;
		return investigate.hunk ? [investigate.hunk] : null;
	});

	function openFile() {
		if (!entry) return;
		const query = new URLSearchParams({ rev: entry.commit });
		if (entry.path) query.set('path', entry.path);
		// A folder has no file to open: Files opens its tree there.
		else if (target?.kind === 'folder') query.set('dir', target.path);
		void goto(`/files?${query}`);
	}

	function showInGraph() {
		if (!entry) return;
		graph.want(entry.commit);
		void goto('/');
	}

	const percent = (share: number) => `${Math.round(share * 100)}%`;
	/** An owner's colour: the one that person or agent has everywhere else. */
	const ownerInk = (owner: { name: string; email: string; agent: boolean }) =>
		inkOf({ agent: owner.agent, authorEmail: owner.email, authorName: owner.name });
</script>

<section class="card detail" aria-label={entry ? `${entry.kind}, ${dayOf(entry.time)}` : 'No event selected'}>
	{#if !entry}
		<p class="note said">Choose an event</p>
	{:else}
		<header class="head">
			<span class="kind" style:color={KIND_INK[entry.kind]}>{entry.kind}</span>
			<span class="summary">{entry.summary}</span>
			<span class="note" title={fullDate(entry.time)}>
				{#if entry.agent}<Icon name="agent" size={12} weight={2.2} />{/if}
				{byOf(entry)} · {dayOf(entry.time)}{#if entry.note}&nbsp;· {entry.note}{/if}
			</span>
			<span class="gap"></span>
			<span class="mono muted">{entry.short}</span>
		</header>

		<div class="stats">
			{#if target?.kind === 'file' && entry.share !== null}
				<div class="stat">
					<span class="figure">{percent(entry.share)}</span>
					<span class="note small">of the file changed</span>
				</div>
			{/if}
			{#if target?.kind === 'file' && investigate.alive !== null && entry.added > 0}
				<div class="stat">
					<span class="figure">{investigate.alive} / {entry.added}</span>
					<span class="note small">of its lines still alive</span>
				</div>
			{/if}
			{#if target?.kind === 'folder'}
				<div class="stat">
					<span class="figure">
						{entry.filesAdded} · {entry.filesRemoved} · {entry.filesRenamed}
					</span>
					<span class="note small">files added · removed · renamed</span>
				</div>
			{/if}
			{#if target?.kind !== 'file' && (entry.added > 0 || entry.removed > 0)}
				<div class="stat">
					<span class="figure mono"
						><span class="plus">+{entry.added}</span> <span class="minus">−{entry.removed}</span></span
					>
					<span class="note small">lines</span>
				</div>
			{/if}
			{#if isFile && today && today.owners.length > 0}
				<div class="owners">
					<span class="note small">Who owns it today</span>
					<div
						class="bar"
						role="img"
						aria-label={today.owners.map((owner) => `${owner.name} ${percent(owner.share)}`).join(', ')}
					>
						{#each today.owners as owner (owner.name + owner.agent)}
							<span
								style:width="{owner.share * 100}%"
								style:background={ownerInk(owner)}
								title="{owner.name} · {owner.lines} lines"
							></span>
						{/each}
					</div>
					<div class="legend small">
						{#each today.owners.slice(0, 5) as owner (owner.name + owner.agent)}
							<span style:color={ownerInk(owner)}>
								{#if owner.agent}<Icon name="agent" size={11} weight={2.2} />{/if}
								{owner.name}
								{percent(owner.share)}
							</span>
						{/each}
						{#if today.owners.length > 5}<span class="note">+ {today.owners.length - 5} more</span>{/if}
					</div>
				</div>
			{/if}
		</div>

		{#if target?.kind === 'line' && entry.text !== null}
			<pre class="then mono" aria-label="The line as this commit left it">{entry.text}</pre>
		{/if}

		{#if isFile}
			<div class="hunks mono" aria-label={investigate.comparing ? 'This commit against today' : 'Largest change'}>
				{#if hunks === undefined}
					<Loader size="inline" label="Reading…" />
				{:else if hunks === null || hunks.length === 0}
					<p class="note said">
						{investigate.comparing ? 'The same as today' : 'No line of the file changed'}
					</p>
				{:else}
					{#each hunks as hunk (hunk.header)}
						<div class="at">{hunk.header}</div>
						{#each hunk.lines as line, i (i)}
							<div class="row {line.origin}">
								<span class="sign" aria-hidden="true"
									>{line.origin === 'added' ? '+' : line.origin === 'removed' ? '−' : ''}</span
								><span class="text">{line.text}</span>
							</div>
						{/each}
					{/each}
				{/if}
			</div>
		{:else}
			<div class="grow"></div>
		{/if}

		<div class="actions">
			{#if target?.kind !== 'branch'}
				<Btn onclick={openFile} title="Read it in Files as {entry.short} left it">
					{target?.kind === 'folder' ? 'Open folder at' : 'Open file at'}
					{entry.short}
				</Btn>
			{/if}
			{#if isFile}
				<Btn
					onclick={() => investigate.toggleCompare()}
					title="This file as {entry.short} left it, against the file today"
				>
					{investigate.comparing ? 'Back to this commit' : 'Compare with today'}
				</Btn>
			{/if}
			<Btn onclick={showInGraph}>Show in Graph</Btn>
		</div>
	{/if}
</section>

<style>
	.detail {
		flex: 999 1 480px;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 16px 18px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
	}

	.kind {
		padding: 1px 8px;
		border: 1px solid currentColor;
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.summary {
		font-weight: 600;
	}

	.gap {
		flex: 1;
	}

	.stats {
		display: flex;
		flex-wrap: wrap;
		gap: 12px 24px;
	}

	.stat {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.figure {
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.plus {
		color: var(--ok);
	}

	.minus {
		color: var(--danger);
	}

	.owners {
		flex: 1 1 260px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
	}

	.bar {
		display: flex;
		gap: 2px;
		height: 12px;
		border-radius: var(--r-pill);
		overflow: hidden;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 14px;
	}

	.then {
		margin: 0;
		padding: 8px 12px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-field);
		background: var(--sunken);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.hunks {
		flex: 1;
		min-height: 80px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		background: var(--sunken);
		overflow: auto;
		line-height: 1.7;
	}

	.grow {
		flex: 1;
	}

	.at {
		padding: 4px 12px;
		border-bottom: 1px solid var(--soft);
		color: var(--muted);
	}

	.row {
		display: grid;
		grid-template-columns: 22px minmax(0, 1fr);
		padding: 0 12px;
		white-space: pre;
	}

	.row.added {
		background: var(--diff-add-bg);
	}

	.row.removed {
		background: var(--diff-del-bg);
	}

	.row.added .sign {
		color: var(--ok);
	}

	.row.removed .sign {
		color: var(--danger);
	}

	.said {
		margin: auto;
		padding: 18px;
		text-align: center;
		font-family: var(--font-ui);
	}

	.actions {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
</style>
