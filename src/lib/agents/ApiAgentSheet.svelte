<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import FarmSheet from '$lib/farm/components/FarmSheet.svelte';
	import * as api from './api';
	import { agents } from './store.svelte';
	import { leaves } from './levels';
	import type { Jobs, ModelProvider, RemoteAgent, Tested } from './types';

	/**
	 * Add an API agent (2.0): provider, key, model, *Test*, what it may do,
	 * its limits, and what leaves the machine.
	 *
	 * The key goes to the keychain on *Add* and is never read back: editing an
	 * agent shows that a key is stored and its last four characters, and a new
	 * key replaces it only when one is typed.
	 */
	interface Props {
		/** The agent being edited; absent to add one. */
		editing?: RemoteAgent | null;
		onclose: () => void;
	}

	let { editing = null, onclose }: Props = $props();

	const PROVIDERS: { id: ModelProvider; label: string }[] = [
		{ id: 'anthropic', label: 'Anthropic' },
		{ id: 'openAi', label: 'OpenAI' },
		{ id: 'google', label: 'Google' },
		{ id: 'compatible', label: 'OpenAI-compatible' }
	];

	const label = (id: ModelProvider) => PROVIDERS.find((p) => p.id === id)?.label ?? id;

	/** The fields start from the agent being edited, once: the sheet is the
	 * person's from then on. */
	const start = untrack(() => editing);

	let provider = $state<ModelProvider>(start?.provider ?? 'anthropic');
	let name = $state(start?.name ?? 'Anthropic');
	let base = $state(start?.base ?? '');
	let key = $state('');
	let model = $state(start?.model ?? '');
	let jobs = $state<Jobs>(start?.jobs ?? { review: true, merge: true, farm: false });
	let perRun = $state(start?.tokensPerRun?.toString() ?? '40000');
	let perDay = $state(start?.tokensPerDay?.toString() ?? '400000');
	let models = $state<string[]>([]);
	let tested = $state<Tested | null>(null);
	let testing = $state(false);
	let saving = $state(false);
	let failed = $state<string | null>(null);

	/** The name follows the provider until somebody types one of their own. */
	let named = $state(!!start);

	function pick(next: ModelProvider) {
		provider = next;
		if (!named) name = label(next);
		models = [];
		tested = null;
	}

	const local = $derived(
		provider === 'compatible' && /^https?:\/\/(localhost|127\.|\[::1\])/i.test(base.trim())
	);
	const keyed = $derived(!!key.trim() || !!editing?.keyEnd || local);
	const ready = $derived(!!model.trim() && (provider !== 'compatible' || !!base.trim()) && keyed);

	const probe = () => ({
		id: editing?.id ?? null,
		provider,
		base: base.trim(),
		model: model.trim(),
		key: key.trim() || null
	});

	async function listModels() {
		if (!keyed && !local) return;
		try {
			models = await api.models(probe());
		} catch {
			// A provider with no list, or no key yet: the model is typed.
			models = [];
		}
	}

	async function test() {
		testing = true;
		tested = null;
		try {
			tested = await api.testRemote(probe());
		} catch (cause) {
			tested = { ok: false, said: agents.failure(cause).message, ms: 0 };
		} finally {
			testing = false;
		}
	}

	function count(text: string): number | null {
		const value = Number(text.replace(/[^0-9]/g, ''));
		return Number.isFinite(value) && value > 0 ? value : null;
	}

	async function add() {
		if (!ready || saving) return;
		saving = true;
		failed = null;
		try {
			await api.saveRemote({
				id: editing?.id ?? null,
				name: name.trim(),
				provider,
				base: base.trim(),
				model: model.trim(),
				jobs,
				tokensPerRun: count(perRun),
				tokensPerDay: count(perDay),
				minutes: editing?.minutes ?? null,
				key: key.trim() || null
			});
			await agents.load(agents.repo);
			onclose();
		} catch (cause) {
			failed = agents.failure(cause).message;
		} finally {
			saving = false;
		}
	}
</script>

<FarmSheet title="API agent" {onclose}>
	<div class="row">
		<span class="label">Provider</span>
		<div class="group" role="radiogroup" aria-label="Provider">
			{#each PROVIDERS as option (option.id)}
				<Chip active={provider === option.id} onclick={() => pick(option.id)}>{option.label}</Chip>
			{/each}
		</div>
	</div>
	<div class="row">
		<label class="label" for="api-name">Name</label>
		<input id="api-name" class="field" bind:value={name} oninput={() => (named = true)} />
	</div>
	{#if provider === 'compatible'}
		<div class="row">
			<label class="label" for="api-base">Base URL</label>
			<input
				id="api-base"
				class="field mono"
				bind:value={base}
				placeholder="http://localhost:11434/v1"
				spellcheck="false"
			/>
		</div>
	{/if}
	<div class="row">
		<label class="label" for="api-key">Key</label>
		<input
			id="api-key"
			class="field mono"
			type="password"
			autocomplete="off"
			bind:value={key}
			onchange={listModels}
			placeholder={editing?.keyEnd ? `stored, ends ${editing.keyEnd}` : local ? 'not needed here' : ''}
		/>
		<span class="lock" title="Kept in the system keychain"><Icon name="lock" size="1em" /></span>
	</div>
	<div class="row">
		<label class="label" for="api-model">Model</label>
		<input
			id="api-model"
			class="field mono"
			list="api-models"
			bind:value={model}
			onfocus={listModels}
			spellcheck="false"
		/>
		<datalist id="api-models">
			{#each models as option (option)}<option value={option}></option>{/each}
		</datalist>
		<Btn disabled={!ready} busy={testing} onclick={test}>Test</Btn>
		{#if tested}
			<span class="tested" class:ok={tested.ok} title={tested.said}>
				{#if tested.ok}<Icon name="check" size="1em" />{(tested.ms / 1000).toFixed(1)} s{:else}{tested.said}{/if}
			</span>
		{/if}
	</div>
	<hr class="hr" />
	<div class="row">
		<span class="label">May do</span>
		<div class="group" role="group" aria-label="What it may do">
			<Chip active={jobs.review} onclick={() => (jobs = { ...jobs, review: !jobs.review })}>Review</Chip>
			<Chip active={jobs.merge} onclick={() => (jobs = { ...jobs, merge: !jobs.merge })}>Merge</Chip>
		</div>
	</div>
	<div class="row">
		<label class="label" for="api-run">Tokens</label>
		<input id="api-run" class="field number mono" bind:value={perRun} inputmode="numeric" />
		<span class="note">per run</span>
		<input class="field number mono" aria-label="Tokens per day" bind:value={perDay} inputmode="numeric" />
		<span class="note">per day</span>
	</div>
	<p class="leaves" class:here={local}>
		<Icon name={local ? 'machine' : 'upload'} size="1.1em" />
		{leaves({ local, providerLabel: provider === 'compatible' ? base.trim() || 'that endpoint' : label(provider) })}
	</p>
	{#if failed}<p class="note error">{failed}</p>{/if}
	<div class="actions">
		<Btn onclick={onclose}>Cancel</Btn>
		<Btn primary quiet disabled={!ready} busy={saving} onclick={add}>{editing ? 'Save' : 'Add'}</Btn>
	</div>
</FarmSheet>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.label {
		width: 96px;
		flex: none;
		color: var(--muted);
	}

	.field {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-ui);
	}

	.field.number {
		flex: 0 1 120px;
	}

	.group {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.lock {
		color: var(--muted);
		display: inline-flex;
	}

	.tested {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--danger);
		font-size: var(--fs-secondary);
		max-width: 180px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tested.ok {
		color: var(--ok);
	}

	.leaves {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		border-radius: var(--r-panel);
		background: var(--warn-soft);
	}

	.leaves.here {
		background: var(--ok-soft);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
