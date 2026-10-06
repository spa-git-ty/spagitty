<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { SettingView } from './types';

	/**
	 * One declared setting, drawn from its declaration. The host validates
	 * every write again; this only keeps an obviously wrong value from being
	 * sent. Executables are chosen in the tools row, not typed here.
	 */
	interface Props {
		setting: SettingView;
		disabled?: boolean;
		onchange: (value: unknown) => void;
	}

	let { setting, disabled = false, onchange }: Props = $props();
	const label = $derived(setting.title ?? setting.key);
	const id = `setting-${Math.random().toString(36).slice(2)}`;

	function number(text: string) {
		const value = Number(text);
		if (!Number.isFinite(value)) return;
		if (setting.min != null && value < setting.min) return;
		if (setting.max != null && value > setting.max) return;
		onchange(value);
	}
</script>

<div class="field" title={setting.description ?? undefined}>
	<label class="label" for={id}>
		{label}{#if setting.scope === 'repository'}<span class="scope"> · this repository</span>{/if}
	</label>
	{#if setting.type === 'boolean'}
		<input
			{id}
			type="checkbox"
			{disabled}
			checked={setting.value === true}
			onchange={(e) => onchange(e.currentTarget.checked)}
		/>
	{:else if setting.type === 'enum'}
		<select {id} {disabled} value={String(setting.value ?? '')} onchange={(e) => onchange(e.currentTarget.value)}>
			{#each setting.values ?? [] as value (value)}
				<option {value}>{setting.labels?.[value] ?? value}</option>
			{/each}
		</select>
	{:else if setting.type === 'number'}
		<input
			{id}
			type="number"
			{disabled}
			min={setting.min ?? undefined}
			max={setting.max ?? undefined}
			value={Number(setting.value ?? 0)}
			onchange={(e) => number(e.currentTarget.value)}
		/>
	{:else if setting.type === 'text'}
		<input
			{id}
			type="text"
			spellcheck="false"
			{disabled}
			maxlength={setting.maxLength ?? 1000}
			value={String(setting.value ?? '')}
			onchange={(e) => onchange(e.currentTarget.value)}
		/>
	{:else}
		<span {id} class="mono value">{setting.value ? String(setting.value) : 'Found on PATH'}</span>
	{/if}
</div>

<style>
	.field {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		min-height: 26px;
	}

	.label {
		color: var(--ink);
	}

	.scope {
		color: var(--muted);
	}

	select,
	input[type='text'],
	input[type='number'] {
		font-size: var(--fs-secondary);
		padding: 3px 6px;
		max-width: 260px;
	}

	.value {
		color: var(--muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 300px;
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}
</style>
