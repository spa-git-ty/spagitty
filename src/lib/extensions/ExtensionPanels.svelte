<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { untrack } from 'svelte';
	import * as api from './api';
	import { panelsFor, type PlacedPanel } from './contributions';
	import FindingsPanel from './FindingsPanel.svelte';
	import ReviewStatusPanel from './ReviewStatusPanel.svelte';
	import { extensions } from './store.svelte';
	import SummaryPanel from './SummaryPanel.svelte';
	import type { ContextKind, ReviewRecord, ReviewStatusData, ReviewTarget, SummaryData } from './types';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * Every extension panel placed at `location`, drawn by the host's
	 * renderers. Findings come from the host's history; the other two ask the
	 * extension for data, on demand, and show what they get as values.
	 */
	interface Props {
		location: ContextKind;
		/** Only this extension's panels. */
		only?: string | null;
		/**
		 * Ask for data on mount. Off where looking must not start a worker —
		 * Settings — so the panel waits for its button.
		 */
		auto?: boolean;
		/** Bumped by the screen when what it shows changed, to ask again. */
		revision?: unknown;
		onsend?: (extension: string, record: ReviewRecord, findings: string[]) => Promise<void> | void;
		onopen?: (path: string, line: number | null) => void;
	}

	let { location, only = null, auto = true, revision = null, onsend, onopen }: Props = $props();

	const placed = $derived(
		panelsFor(extensions.all, location).filter((entry) => only === null || entry.extension.id === only)
	);
	let data = $state<Record<string, { value?: unknown; error?: string; loading: boolean }>>({});

	const target: ReviewTarget = $derived(
		location === 'farmTask' ? 'farmTask' : location === 'pullRequest' ? 'pullRequest' : 'workingCopy'
	);

	function describe(error: unknown): string {
		if (error && typeof error === 'object' && 'message' in error) return String((error as { message: unknown }).message);
		return String(error);
	}

	async function load(entry: PlacedPanel) {
		data = { ...data, [entry.key]: { ...(data[entry.key] ?? {}), loading: true } };
		try {
			const value = await api.panel(entry.extension.id, entry.panel.id, extensions.invocationFor(location));
			data = { ...data, [entry.key]: { value, loading: false } };
		} catch (error) {
			data = { ...data, [entry.key]: { error: describe(error), loading: false } };
		}
	}

	$effect(() => {
		void revision;
		if (!auto) return;
		const wanted = placed.filter((entry) => entry.panel.renderer !== 'reviewFindings');
		// Loading writes `data`; untracked so the effect does not depend on it.
		untrack(() => {
			for (const entry of wanted) void load(entry);
		});
	});
</script>

{#each placed as entry (entry.key)}
	<div class="panel" data-panel={entry.key}>
		{#if entry.panel.renderer === 'reviewFindings' && entry.provider}
			<FindingsPanel
				extension={entry.extension}
				provider={entry.provider}
				title={entry.panel.title}
				{target}
				taskId={extensions.invocationFor(location).taskId ?? null}
				{onopen}
				onsend={onsend ? (record, findings) => onsend(entry.extension.id, record, findings) : undefined}
			/>
		{:else if entry.panel.renderer !== 'reviewFindings'}
			<div class="head">
				<span class="title">{entry.panel.title}</span>
				<Btn disabled={data[entry.key]?.loading} onclick={() => load(entry)}>
					{data[entry.key] ? 'Refresh' : 'Show'}
				</Btn>
			</div>
			{#if data[entry.key]?.error}
				<p class="error">{data[entry.key]?.error}</p>
			{:else if data[entry.key]?.value}
				{#if entry.panel.renderer === 'reviewStatus'}
					<ReviewStatusPanel data={data[entry.key]?.value as ReviewStatusData} />
				{:else}
					<SummaryPanel data={data[entry.key]?.value as SummaryData} />
				{/if}
			{:else if data[entry.key]?.loading}
				<Loader label="Loading…" />
			{/if}
		{/if}
	</div>
{/each}

<style>
	.panel {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
		border-radius: var(--r-panel);
		background: var(--surface-2);
		border: 1px solid var(--line);
		min-width: 0;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.title {
		font-weight: 600;
	}

	.error {
		color: var(--danger);
		margin: 0;
	}
</style>
