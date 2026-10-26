<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { notice } from '$lib/ui/notice.svelte';
	import BodyView from './BodyView.svelte';
	import { cookiesOf, formatMillis, formatSize, statusTone } from './model';
	import type { Outcome } from './store.svelte';
	import type { Environment, KeepRule } from './types';

	/**
	 * The response half of Calls (FEAT-160, proposal §5.1 and §5.3): status
	 * with a dot, time and size; *Body*, *Headers* and *Cookies*; *Beautify*
	 * for JSON and XML. A binary body shows its type and size and *Save as…*.
	 * An event stream is shown as it arrives.
	 */

	interface Props {
		outcome: Outcome | null;
		sending: boolean;
		/** On by default; Settings › Calls can turn it off. */
		beautify: boolean;
		name: string;
		ontoggleraw: () => void;
		onsave: (path: string) => void;
		/** Keeping a clicked value (FEAT-161); absent when Settings turns it off. */
		rules?: KeepRule[];
		environment?: Environment | null;
		keepEvery?: boolean;
		suggest?: boolean;
		onkeep?: (ask: {
			path: string;
			value: string;
			name: string;
			every: boolean;
			ns?: Record<string, string>;
		}) => Promise<boolean>;
		onforget?: (name: string) => void;
		/** A SOAP call: the bar says whether there was a fault (FEAT-163). */
		soap?: boolean;
	}

	let {
		outcome,
		sending,
		beautify,
		name,
		ontoggleraw,
		onsave,
		rules = [],
		environment = null,
		keepEvery = true,
		suggest = true,
		onkeep,
		onforget,
		soap = false
	}: Props = $props();

	type Tab = 'body' | 'headers' | 'cookies';
	let tab = $state<Tab>('body');

	const response = $derived(outcome?.response ?? null);
	const cookies = $derived(response ? cookiesOf(response.headers) : []);
	const tone = $derived(response ? statusTone(response.status) : 'plain');
	const formattable = $derived(response?.shape === 'json' || response?.shape === 'xml');
	/** Shown as received: asked for, or Beautify is off in Settings and not asked for. */
	const raw = $derived(outcome ? outcome.raw === beautify : !beautify);

	async function copy() {
		try {
			await navigator.clipboard.writeText(response?.text ?? outcome?.streamed ?? '');
			notice.ok('Response copied');
		} catch (cause) {
			notice.failed('Could not copy the response', cause);
		}
	}

	function suggestedName(): string {
		const base = name.trim().replace(/[^\w.-]+/g, '-') || 'response';
		const ext: Record<string, string> = { json: 'json', xml: 'xml', html: 'html', events: 'txt', text: 'txt' };
		return `${base}.${ext[response?.shape ?? ''] ?? 'bin'}`;
	}

	async function saveAs() {
		const path = await saveDialog({ defaultPath: suggestedName() }).catch(() => null);
		if (path) onsave(path);
	}
</script>

<section class="response" aria-label="The response">
	<div class="bar">
		{#if response}
			<span class="status {tone}" title={response.url}
				><span class="dot" aria-hidden="true"></span><span class="mono">{response.status} {response.reason}</span></span
			>
			<span class="meta mono">{formatMillis(response.millis)} · {formatSize(response.size)}</span>
			{#if response.fault}
				<span class="fault-tag" title={response.fault.reason}>Fault · {response.fault.code}</span>
			{:else if soap && response.shape === 'xml'}
				<span class="meta">no fault</span>
			{/if}
			<span class="sep" aria-hidden="true"></span>
			<div class="tabs" role="tablist" aria-label="Parts of the response">
				<button type="button" role="tab" class="tab" class:on={tab === 'body'} aria-selected={tab === 'body'} onclick={() => (tab = 'body')}
					>Body</button
				>
				<button
					type="button"
					role="tab"
					class="tab"
					class:on={tab === 'headers'}
					aria-selected={tab === 'headers'}
					onclick={() => (tab = 'headers')}>Headers <span class="count">{response.headers.length}</span></button
				>
				{#if cookies.length}
					<button
						type="button"
						role="tab"
						class="tab"
						class:on={tab === 'cookies'}
						aria-selected={tab === 'cookies'}
						onclick={() => (tab = 'cookies')}>Cookies <span class="count">{cookies.length}</span></button
					>
				{/if}
			</div>
			<span class="gap"></span>
			{#if formattable && raw}
				<span class="note">As received{onkeep ? ' · values cannot be kept' : ''}</span>
			{:else if formattable && onkeep && !response.plain}
				<span class="note">Click a value to keep it</span>
			{/if}
			{#if response.shape !== 'binary'}
				<button type="button" class="act" title="Copy the body as it arrived" onclick={copy}>Copy</button>
			{/if}
			<button type="button" class="act" title="Save the body as it arrived" onclick={saveAs}>Save as…</button>
			{#if formattable && !response.plain}
				<button
					type="button"
					class="act beautify"
					class:on={!raw}
					aria-pressed={!raw}
					title={raw ? 'Show it formatted' : 'Show it as it arrived'}
					onclick={ontoggleraw}>{'{ }'} Beautify</button
				>
			{/if}
		{:else if sending}
			<span class="status plain"><span class="dot live" aria-hidden="true"></span>Waiting for the answer…</span>
		{:else if outcome?.cancelled}
			<span class="status plain"><span class="dot" aria-hidden="true"></span>Cancelled</span>
		{:else if outcome?.error}
			<span class="status danger"><span class="dot" aria-hidden="true"></span>Not answered</span>
		{:else}
			<span class="note">The response shows here after <em>Send</em>.</span>
		{/if}
	</div>

	<div class="pane">
		{#if response}
			{#if tab === 'headers'}
				<table class="mono">
					<tbody>
						{#each response.headers as [header, value], index (index)}
							<tr><th>{header}</th><td>{value}</td></tr>
						{/each}
					</tbody>
				</table>
			{:else if tab === 'cookies'}
				<table class="mono">
					<tbody>
						{#each cookies as cookie, index (index)}
							<tr><th>{cookie.name}</th><td>{cookie.value}</td><td class="attrs">{cookie.attributes}</td></tr>
						{/each}
					</tbody>
				</table>
			{:else if response.shape === 'binary' || response.text === null}
				<div class="binary">
					<p>{response.mediaType || 'A binary body'} · {formatSize(response.size)}</p>
					<button type="button" class="act" onclick={saveAs}>Save as…</button>
				</div>
			{:else}
				{#if response.fault}
					<div class="fault" role="alert">
						<p><strong>{response.fault.code}</strong> {response.fault.reason}</p>
						{#if response.fault.detail}<pre class="mono">{response.fault.detail}</pre>{/if}
					</div>
				{/if}
				{#if response.truncated || response.plain}
					<p class="cut">
						{response.truncated ? 'Cut at 50 MB: the rest was not read.' : ''}
						{response.plain ? 'Over 2 MB, so shown without colour.' : ''}
					</p>
				{/if}
				{#if response.text === ''}
					<p class="empty">No body.</p>
				{:else}
					<BodyView
						text={response.text}
						shape={response.shape}
						{raw}
						plain={response.plain}
						{rules}
						{environment}
						{keepEvery}
						{suggest}
						onkeep={formattable && !raw ? onkeep : undefined}
						{onforget}
					/>
				{/if}
			{/if}
		{:else if outcome?.streamed}
			<BodyView text={outcome.streamed} shape="events" raw plain />
		{:else if outcome?.error}
			<p class="error" role="alert">{outcome.error}</p>
		{/if}
	</div>
</section>

<style>
	.response {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
		min-height: 30px;
	}

	.status {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-weight: 600;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--muted);
	}

	.status.ok {
		color: var(--ok);
	}
	.status.ok .dot {
		background: var(--ok);
	}
	.status.warn {
		color: var(--warn);
	}
	.status.warn .dot {
		background: var(--warn);
	}
	.status.danger {
		color: var(--danger);
	}
	.status.danger .dot {
		background: var(--danger);
	}
	.status.plain {
		color: var(--muted);
		font-weight: normal;
	}

	.dot.live {
		background: var(--accent);
		animation: pulse 1.2s var(--ease) infinite;
	}

	@keyframes pulse {
		50% {
			opacity: 0.3;
		}
	}

	.meta,
	.note,
	.count {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.sep {
		width: 1px;
		height: 16px;
		background: var(--line);
	}

	.tabs {
		display: flex;
		gap: 2px;
	}

	.tab {
		padding: 3px 10px;
		border-radius: var(--r-pill);
		color: var(--muted);
	}

	.tab:hover {
		color: var(--ink);
	}

	.tab.on {
		background: var(--surface-2);
		color: var(--ink);
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

	.act:hover {
		background: var(--hover);
	}

	.beautify.on {
		border-color: color-mix(in srgb, var(--accent) 60%, transparent);
		color: var(--accent);
	}

	.pane {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: auto;
		padding: 8px 0;
		background: var(--sunken);
		border-radius: var(--r-panel);
	}

	table {
		border-collapse: collapse;
		margin: 0 12px;
		font-size: var(--fs-mono);
	}

	th {
		padding: 3px 16px 3px 0;
		color: var(--tok-tag);
		font-weight: normal;
		text-align: left;
		vertical-align: top;
		white-space: nowrap;
	}

	td {
		padding: 3px 12px 3px 0;
		overflow-wrap: anywhere;
	}

	.attrs {
		color: var(--muted);
	}

	.binary,
	.empty,
	.cut,
	.error {
		margin: 0;
		padding: 4px 12px;
	}

	.binary {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.fault-tag {
		padding: 1px 8px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--danger) 15%, transparent);
		color: var(--danger);
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	.fault {
		margin: 0 12px 8px;
		padding: 8px 12px;
		border: 1px solid color-mix(in srgb, var(--danger) 45%, transparent);
		border-radius: var(--r-field);
		background: color-mix(in srgb, var(--danger) 8%, transparent);
	}

	.fault p {
		margin: 0;
	}

	.fault strong {
		color: var(--danger);
	}

	.fault pre {
		margin: 6px 0 0;
		font-size: var(--fs-mono);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.cut,
	.empty {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.error {
		color: var(--danger);
		white-space: pre-wrap;
	}
</style>
