<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Markdown from './Markdown.svelte';
	import type { SummaryData } from './types';

	/** The `summary` renderer (FEAT-096): a title, rows of label and value, and text. */
	interface Props {
		data: SummaryData;
	}

	let { data }: Props = $props();
	const show = (value: unknown) => (typeof value === 'string' ? value : JSON.stringify(value));
</script>

<div class="summary">
	{#if data.title}<p class="title">{data.title}</p>{/if}
	{#if data.rows?.length}
		<dl>
			{#each data.rows.slice(0, 50) as row, index (index)}
				<dt>{row.label}</dt>
				<dd>{show(row.value)}</dd>
			{/each}
		</dl>
	{/if}
	{#if data.text}<Markdown source={data.text} />{/if}
</div>

<style>
	.summary {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.title {
		margin: 0;
		font-weight: 600;
	}

	dl {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 2px 12px;
		margin: 0;
	}

	dt {
		color: var(--muted);
	}

	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
</style>
