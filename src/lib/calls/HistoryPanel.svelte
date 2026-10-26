<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import * as api from './api';
	import BodyView from './BodyView.svelte';
	import { formatMillis, formatSize, methodColour, statusTone } from './model';
	import type { HistoryEntry } from './types';

	/**
	 * What was sent, newest first (FEAT-160, proposal §5.3). Each entry is the
	 * request as resolved, with secrets as their names, and the response up
	 * to 1 MB. It lives in application data, never in the repository.
	 */

	interface Props {
		entries: HistoryEntry[];
		onclose: () => void;
		onclear: () => void;
		/** Open the call an entry was sent from, when it still exists. */
		onopen: (entry: HistoryEntry) => void;
	}

	let { entries, onclose, onclear, onopen }: Props = $props();

	let chosen = $state<HistoryEntry | null>(null);
	let body = $state<string | null>(null);
	let reading = 0;

	async function choose(entry: HistoryEntry) {
		chosen = entry;
		body = null;
		const ticket = ++reading;
		const text = await api.historyBody(entry.id).catch(() => null);
		if (ticket === reading) body = text;
	}

	const when = (at: number) =>
		new Date(at).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit' });
</script>

<section class="history" aria-label="History">
	<header>
		<span class="title">History</span>
		<span class="count">{entries.length}</span>
		<span class="gap"></span>
		<button type="button" class="act" disabled={entries.length === 0} onclick={onclear}>Clear</button>
		<button type="button" class="act" onclick={onclose}>Done</button>
	</header>
	<div class="split">
		<ol class="entries">
			{#each entries as entry (entry.id)}
				<li>
					<button type="button" class:on={chosen?.id === entry.id} onclick={() => choose(entry)}>
						<span class="method mono" style:color={methodColour(entry.request.method)}>{entry.request.method}</span>
						<span class="name">{entry.name}</span>
						<span class="gap"></span>
						{#if entry.status !== null}
							<span class="status mono {statusTone(entry.status)}">{entry.status}</span>
						{:else}
							<span class="status mono danger" title={entry.error ?? ''}>failed</span>
						{/if}
						<span class="when">{when(entry.at)}</span>
					</button>
				</li>
			{:else}
				<li class="none">Nothing sent yet.</li>
			{/each}
		</ol>
		{#if chosen}
			<div class="entry">
				<p class="line mono">
					<span style:color={methodColour(chosen.request.method)}>{chosen.request.method}</span>
					{chosen.request.url}
				</p>
				<p class="meta">
					{#if chosen.environment}in {chosen.environment} ·
					{/if}{chosen.status !== null
						? `${chosen.status} ${chosen.reason} · ${formatMillis(chosen.millis)} · ${formatSize(chosen.size)}`
						: chosen.error}
					{#if chosen.callId}
						· <button type="button" class="link" onclick={() => chosen && onopen(chosen)}>Open the call</button>
					{/if}
				</p>
				<details>
					<summary>Request headers {chosen.request.headers.length}</summary>
					<table class="mono">
						<tbody>
							{#each chosen.request.headers as [name, value], index (index)}
								<tr><th>{name}</th><td>{value}</td></tr>
							{/each}
						</tbody>
					</table>
					{#if chosen.request.body}
						<pre class="mono">{chosen.request.body}</pre>
					{/if}
				</details>
				{#if chosen.cut}
					<p class="meta">The response was over 1 MB; the start of it is kept.</p>
				{/if}
				<div class="body">
					{#if body}
						<BodyView
							text={body}
							shape={/^\s*[[{]/.test(body) ? 'json' : /^\s*</.test(body) ? 'xml' : 'text'}
							raw={false}
							plain={body.length > 2 * 1024 * 1024}
						/>
					{:else if body === null && chosen.status !== null}
						<p class="meta">No body kept.</p>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</section>

<style>
	.history {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.title {
		font-weight: 600;
	}

	.count,
	.when,
	.meta,
	.none,
	summary {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.gap {
		flex: 1;
	}

	.act {
		padding: 3px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-button);
		font-size: var(--fs-secondary);
	}

	.act:hover:not(:disabled) {
		background: var(--hover);
	}

	.split {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
	}

	.entries {
		flex: 0 0 44%;
		margin: 0;
		padding: 0;
		overflow: auto;
		list-style: none;
	}

	.entries button {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 4px 8px;
		border-radius: var(--r-row);
		text-align: left;
	}

	.entries button:hover {
		background: var(--hover);
	}

	.entries button.on {
		background: var(--surface-2);
	}

	.method {
		width: 52px;
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status {
		font-size: var(--fs-secondary);
	}
	.status.ok {
		color: var(--ok);
	}
	.status.warn {
		color: var(--warn);
	}
	.status.danger {
		color: var(--danger);
	}

	.entry {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
		background: var(--sunken);
		border-radius: var(--r-panel);
		overflow: auto;
	}

	.line {
		margin: 0;
		font-size: var(--fs-mono);
		overflow-wrap: anywhere;
	}

	.meta {
		margin: 0;
	}

	.link {
		color: var(--accent);
		text-decoration: underline;
	}

	table {
		margin-top: 4px;
		font-size: var(--fs-mono);
		border-collapse: collapse;
	}

	th {
		padding: 2px 12px 2px 0;
		color: var(--tok-tag);
		font-weight: normal;
		text-align: left;
		white-space: nowrap;
	}

	td {
		overflow-wrap: anywhere;
	}

	pre {
		margin: 6px 0 0;
		font-size: var(--fs-mono);
		white-space: pre-wrap;
	}

	.body {
		flex: 1;
		min-height: 120px;
		display: flex;
		flex-direction: column;
	}
</style>
