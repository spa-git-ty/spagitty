<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Found } from './model';
	import type { Pair } from './types';
	import VarField from './VarField.svelte';

	/**
	 * Names and values in rows (FEAT-160): query parameters, headers and form
	 * fields. Each row can be switched off without being lost, and the empty
	 * row at the foot becomes a new one as soon as something is typed in it.
	 */

	interface Props {
		pairs: Pair[];
		onchange: (pairs: Pair[]) => void;
		find: (name: string) => Found | null;
		names: string[];
		/** What a row is, for labels: "header", "parameter", "field". */
		noun: string;
		/** Rows cannot be switched off: the query, which has nowhere to keep one. */
		noSwitch?: boolean;
		/** Set for multipart: whether each value is a file path. */
		files?: boolean[];
		onfiles?: (files: boolean[]) => void;
	}

	let { pairs, onchange, find, names, noun, noSwitch = false, files, onfiles }: Props = $props();

	const rows = $derived([...pairs, ['', ''] as Pair]);

	function set(index: number, at: 0 | 1, text: string) {
		const next = pairs.map((pair) => [...pair] as Pair);
		if (index === pairs.length) next.push(['', '']);
		next[index][at] = text;
		onchange(next);
	}

	function toggle(index: number) {
		onchange(
			pairs.map((pair, at) => {
				if (at !== index) return pair;
				return pair[2] === false ? [pair[0], pair[1]] : [pair[0], pair[1], false];
			})
		);
	}

	function remove(index: number) {
		onchange(pairs.filter((_, at) => at !== index));
		if (files && onfiles) onfiles(files.filter((_, at) => at !== index));
	}

	function toggleFile(index: number) {
		if (!onfiles) return;
		const next = [...(files ?? [])];
		while (next.length <= index) next.push(false);
		next[index] = !next[index];
		onfiles(next);
	}
</script>

<div class="pairs" role="table" aria-label={`${noun}s`}>
	{#each rows as pair, index (index)}
		{@const last = index === pairs.length}
		<div class="row" class:off={pair[2] === false} role="row">
			{#if !noSwitch}
				<input
					type="checkbox"
					aria-label={`Send this ${noun}`}
					title={pair[2] === false ? `Off: this ${noun} is not sent` : `This ${noun} is sent`}
					checked={pair[2] !== false}
					disabled={last}
					onchange={() => toggle(index)}
				/>
			{/if}
			<VarField
				label={`${noun} name`}
				placeholder={last ? `Add a ${noun}` : 'name'}
				value={pair[0]}
				onchange={(text) => set(index, 0, text)}
				{find}
				{names}
			/>
			<VarField
				label={`${noun} value`}
				placeholder={files?.[index] ? 'path to a file' : 'value'}
				value={pair[1]}
				onchange={(text) => set(index, 1, text)}
				{find}
				{names}
			/>
			{#if onfiles}
				<button
					type="button"
					class="kind"
					class:on={files?.[index]}
					disabled={last}
					title={files?.[index] ? 'The value is a file path; its bytes are sent' : 'The value is sent as text'}
					onclick={() => toggleFile(index)}>{files?.[index] ? 'file' : 'text'}</button
				>
			{/if}
			<button
				type="button"
				class="remove"
				aria-label={`Remove this ${noun}`}
				title={`Remove this ${noun}`}
				disabled={last}
				onclick={() => remove(index)}>✕</button
			>
		</div>
	{/each}
</div>

<style>
	.pairs {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.row.off :global(.mirror) {
		opacity: 0.45;
		text-decoration: line-through;
	}

	.remove,
	.kind {
		flex: none;
		padding: 2px 6px;
		border-radius: var(--r-button);
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.remove:disabled {
		visibility: hidden;
	}

	.kind {
		min-width: 42px;
		border: 1px solid var(--line);
	}

	.kind.on {
		color: var(--lane-5);
	}

	.remove:hover:not(:disabled),
	.kind:hover:not(:disabled) {
		background: var(--hover);
		color: var(--ink);
	}
</style>
