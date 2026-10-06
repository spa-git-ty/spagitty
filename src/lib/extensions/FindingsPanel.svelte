<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import * as api from './api';
	import { elapsed, GATE_LABELS, headline, location, ordered, SEVERITY_LABELS, tally, when } from './describe';
	import Markdown from './Markdown.svelte';
	import { extensions } from './store.svelte';
	import type { Disposition, ExtensionView, ReviewRecord, ReviewTarget } from './types';
	import Btn from '$lib/ui/Btn.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * The `reviewFindings` renderer (FEAT-096): one review provider's progress,
	 * its latest result and its history, drawn by the host from the host's own
	 * records.
	 */
	interface Props {
		extension: ExtensionView;
		provider: string;
		title: string;
		target?: ReviewTarget;
		/** For a farm task: only that task's reviews. */
		taskId?: string | null;
		/** Hand selected findings on (FEAT-097). Absent where there is nowhere to send them. */
		onsend?: (record: ReviewRecord, findings: string[]) => Promise<void> | void;
		/** Jump to a file in the diff, when the screen has one. */
		onopen?: (path: string, line: number | null) => void;
	}

	let { extension, provider, title, target = 'workingCopy', taskId = null, onsend, onopen }: Props = $props();

	let chosen = $state<string | null>(null);
	let selected = $state<string[]>([]);
	let now = $state(Date.now());
	let ticker: ReturnType<typeof setInterval> | null = null;

	const history = $derived(
		extensions
			.reviewsOf(extension.id)
			.filter(
				(r) =>
					r.provider === provider &&
					r.snapshot.target === target &&
					(taskId === null || r.snapshot.taskId === taskId)
			)
	);
	const record = $derived(history.find((r) => r.result.reviewId === chosen) ?? history[0] ?? null);
	const live = $derived(extensions.running.find((r) => r.extension === extension.id && r.reviewId !== null) ?? null);
	const findings = $derived(record ? ordered(record.result.findings) : []);

	onMount(() => {
		void extensions.loadReviews(extension.id);
		ticker = setInterval(() => (now = Date.now()), 1000);
	});
	onDestroy(() => {
		if (ticker) clearInterval(ticker);
	});

	function describe(error: unknown): string {
		if (error && typeof error === 'object' && 'message' in error) return String((error as { message: unknown }).message);
		return String(error);
	}

	async function dispose(findingId: string, disposition: Disposition) {
		if (!record || !extensions.workdir) return;
		try {
			await api.setDisposition(extension.id, extensions.workdir, record.result.reviewId, findingId, disposition);
			await extensions.loadReviews(extension.id);
		} catch (error) {
			notice.failed('Could not save that', describe(error));
		}
	}

	function toggle(id: string, on: boolean) {
		selected = on ? [...selected, id] : selected.filter((s) => s !== id);
	}

	async function send() {
		if (!record || !onsend || !selected.length) return;
		await onsend(record, selected);
		selected = [];
		await extensions.loadReviews(extension.id);
	}

	async function clear() {
		if (!extensions.workdir) return;
		const yes = await dialog.confirm({
			title: `Delete ${title} history?`,
			body: 'Every saved review for this repository.',
			confirmLabel: 'Delete',
			danger: true
		});
		if (!yes) return;
		try {
			await api.deleteReviews(extension.id, extensions.workdir, null);
			chosen = null;
			await extensions.loadReviews(extension.id);
		} catch (error) {
			notice.failed('Could not delete it', describe(error));
		}
	}
</script>

<section class="panel" aria-label={title}>
	<header class="head">
		<span class="title">{title}</span>
		<div class="tools">
			{#if history.length > 1}
				<select aria-label="Earlier reviews" value={record?.result.reviewId} onchange={(e) => (chosen = e.currentTarget.value)}>
					{#each history as entry (entry.result.reviewId)}
						<option value={entry.result.reviewId}>{when(entry)} · {headline(entry)}</option>
					{/each}
				</select>
			{/if}
			{#if !live}
				<Btn onclick={() => extensions.beginReview(extension.id, provider, target)}>
					{record ? 'Review again' : 'Review'}
				</Btn>
			{/if}
			{#if history.length}<Btn onclick={clear} title="Delete saved reviews">Clear</Btn>{/if}
		</div>
	</header>

	{#if live}
		<div class="live" role="status" aria-live="polite">
			<span class="spinner" aria-hidden="true"></span>
			<span>{live.cancelling ? 'Cancelling' : 'Reviewing'} · {elapsed(Math.max(live.elapsedMs, now - live.startedAt))}</span>
			{#if live.message}<span class="muted">{live.message}</span>{/if}
			{#if live.findings}<span class="muted">{live.findings} so far</span>{/if}
			<Btn disabled={live.cancelling} onclick={() => extensions.cancel(live.operation)}>Cancel</Btn>
		</div>
	{/if}

	{#if record}
		<div class="result">
			<div class="line">
				<span class="status status-{record.result.status}">{headline(record)}</span>
				{#if record.gate}
					<span class="gate gate-{record.gate.gate}" title={record.gate.reason}>{GATE_LABELS[record.gate.gate]}</span>
				{/if}
				<span class="muted" title={`${record.provider} ${record.result.providerVersion} · ${record.snapshot.headCommit}`}>
					{when(record)}
				</span>
			</div>
			{#if record.result.actionRequired}
				<p class="action">{record.result.actionRequired.message}</p>
			{/if}
			{#if record.result.summary}<Markdown source={record.result.summary} />{/if}
			{#if findings.length}
				<p class="tally">
					{#each tally(findings) as entry (entry.severity)}
						<span class="sev sev-{entry.severity}">{entry.count} {SEVERITY_LABELS[entry.severity]}</span>
					{/each}
				</p>
			{/if}
		</div>

		<ol class="findings">
			{#each findings as finding (finding.id)}
				<li class="finding" class:dismissed={finding.disposition === 'dismissed'}>
					<div class="finding-head">
						{#if onsend}
							<input
								type="checkbox"
								aria-label={`Select ${finding.title}`}
								checked={selected.includes(finding.id)}
								onchange={(e) => toggle(finding.id, e.currentTarget.checked)}
							/>
						{/if}
						<span class="sev sev-{finding.severity}" title={finding.providerSeverity ?? undefined}>
							{SEVERITY_LABELS[finding.severity]}
						</span>
						<span class="finding-title">{finding.title}</span>
					</div>
					<div class="where">
						{#if finding.path && onopen}
							<button type="button" class="jump" onclick={() => onopen?.(finding.path ?? '', finding.startLine ?? null)}>
								{location(finding)}
							</button>
						{:else}
							<span class="mono">{location(finding)}</span>
						{/if}
						{#if finding.disposition !== 'open'}
							<span class="muted">{finding.disposition === 'sentToAgent' ? 'sent to an agent' : finding.disposition}</span>
						{/if}
					</div>
					<details>
						<summary>Details</summary>
						<Markdown source={finding.message} />
						{#if finding.suggestion}
							<p class="muted">Suggestion — not run automatically</p>
							<pre>{finding.suggestion}</pre>
						{/if}
					</details>
					<div class="finding-actions">
						{#if finding.disposition === 'open'}
							<Btn onclick={() => dispose(finding.id, 'acknowledged')}>Acknowledge</Btn>
							<Btn onclick={() => dispose(finding.id, 'dismissed')}>Dismiss</Btn>
						{:else}
							<Btn onclick={() => dispose(finding.id, 'open')}>Reopen</Btn>
						{/if}
					</div>
				</li>
			{/each}
		</ol>

		{#if onsend && findings.length}
			<div class="send">
				<Btn primary quiet disabled={!selected.length} onclick={send}>
					Send {selected.length || ''} to an agent
				</Btn>
			</div>
		{/if}
	{:else if !live}
		<p class="muted">No reviews yet.</p>
	{/if}
</section>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}

	.head,
	.line,
	.live,
	.finding-head,
	.where,
	.finding-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.head {
		justify-content: space-between;
	}

	.title {
		font-weight: 600;
	}

	.tools {
		display: flex;
		gap: 6px;
		align-items: center;
	}

	.tools select {
		font-size: var(--fs-secondary);
		max-width: 260px;
	}

	.muted {
		color: var(--muted);
	}

	.live {
		padding: 6px 8px;
		border-radius: var(--r-field);
		background: var(--sunken);
	}

	.spinner {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		border: 2px solid var(--line);
		border-top-color: var(--accent);
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation: none;
		}
	}

	.result {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.status {
		font-weight: 550;
	}

	.status-failed,
	.status-stale,
	.status-incomplete,
	.status-actionRequired {
		color: var(--danger);
	}

	.gate {
		font-size: var(--fs-secondary);
		padding: 0 6px;
		border-radius: var(--r-pill);
		background: var(--sunken);
	}

	.gate-pass {
		color: var(--ok);
	}

	.gate-changesRequested,
	.gate-blocked {
		color: var(--danger);
	}

	.action {
		margin: 0;
		padding: 6px 8px;
		border-radius: var(--r-field);
		background: var(--warn-soft);
	}

	.tally {
		display: flex;
		gap: 8px;
		margin: 0;
	}

	.sev {
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	.sev-critical,
	.sev-high {
		color: var(--danger);
	}

	.sev-medium {
		color: var(--warn);
	}

	.sev-low,
	.sev-info,
	.sev-unknown {
		color: var(--muted);
	}

	.findings {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.finding {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px;
		border-radius: var(--r-field);
		background: var(--surface);
		border: 1px solid var(--line);
		min-width: 0;
	}

	.finding.dismissed {
		opacity: 0.6;
	}

	.finding-title {
		font-weight: 550;
		overflow-wrap: anywhere;
		min-width: 0;
	}

	.mono,
	pre,
	.jump {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.jump {
		all: unset;
		font-family: var(--font-mono);
		color: var(--accent);
		cursor: pointer;
	}

	.jump:focus-visible {
		outline: 2px solid var(--ring);
	}

	pre {
		margin: 0;
		padding: 6px 8px;
		background: var(--sunken);
		border-radius: var(--r-field);
		overflow-x: auto;
	}

	details summary {
		cursor: pointer;
		color: var(--muted);
	}

	.send {
		display: flex;
		justify-content: flex-end;
	}

	p {
		margin: 0;
	}
</style>
