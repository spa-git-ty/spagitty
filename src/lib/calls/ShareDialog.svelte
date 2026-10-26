<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import Sheet from '$lib/ui/Sheet.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import * as api from './api';
	import { formatSize } from './model';
	import type { Collection, Environment, ShareFormat, SharePlan, Stored } from './types';

	/**
	 * Share (FEAT-164, proposal §5.5): a collection as one file, a zip, or a
	 * commit. What goes in is chosen here — the calls always, each
	 * environment, the last response of each call as an example, and for a
	 * zip the history — and the preview lists every file with its size
	 * before anything is written. Secrets and kept values go in empty, in
	 * every format; the dialog says so.
	 */

	interface Props {
		collections: Stored<Collection>[];
		environments: Stored<Environment>[];
		/** The collection of the open call, chosen first. */
		stem: string | null;
		/** Settings › Calls' *Share as*. */
		shareAs: 'file' | 'zip' | 'commit';
		onclose: () => void;
	}

	let { collections, environments, stem, shareAs, onclose }: Props = $props();

	const WAYS: { id: 'file' | 'zip' | 'commit'; title: string; note: string }[] = [
		{ id: 'file', title: 'One file', note: 'Spagitty or Postman JSON' },
		{ id: 'zip', title: 'Zip bundle', note: 'With environments and examples' },
		{ id: 'commit', title: 'Commit to the repository', note: 'On this branch; never pushed' }
	];

	const readable = $derived(collections.filter((stored) => stored.value));
	let chosen = $state<string | null>(null);
	let how = $state<'file' | 'zip' | 'commit'>('zip');
	let flavour = $state<'spagitty' | 'postman'>('spagitty');
	let envs = $state<string[]>([]);
	let examples = $state(false);
	let history = $state(false);
	let plan = $state<SharePlan | null>(null);
	let planError = $state<string | null>(null);
	let busy = $state(false);
	let started = false;

	$effect(() => {
		if (started) return;
		started = true;
		chosen = stem ?? readable[0]?.stem ?? null;
		how = shareAs;
		envs = environments.filter((stored) => stored.value).map((stored) => stored.stem);
	});

	const format = $derived<ShareFormat>(how === 'file' ? flavour : how);
	const collection = $derived(readable.find((stored) => stored.stem === chosen)?.value ?? null);

	/** The preview, read again whenever a choice changes. */
	let seq = 0;
	$effect(() => {
		const ask = { stem: chosen ?? '', format, environments: how === 'file' ? [] : envs, examples: how !== 'file' && examples, history: how === 'zip' && history, path: null };
		if (!chosen) return;
		const ticket = ++seq;
		api.sharePlan(ask)
			.then((read) => {
				if (ticket === seq) {
					plan = read;
					planError = null;
				}
			})
			.catch((cause) => {
				if (ticket === seq) {
					plan = null;
					planError = String(cause);
				}
			});
	});

	const label = $derived.by(() => {
		const name = collection?.name ?? 'the collection';
		if (how === 'commit') return `Commit ${plan?.files.length ?? ''} ${plan?.files.length === 1 ? 'file' : 'files'} to this branch`;
		if (how === 'zip') return `Save ${name}.zip${plan ? ` · ${formatSize(plan.total)}` : ''}`;
		return `Save ${name} as ${flavour === 'postman' ? 'Postman 2.1' : 'Spagitty'} JSON`;
	});

	function toggleEnv(stem: string) {
		envs = envs.includes(stem) ? envs.filter((other) => other !== stem) : [...envs, stem];
	}

	async function go() {
		if (!chosen || busy) return;
		let path: string | null = null;
		if (how !== 'commit') {
			const defaultPath = how === 'zip' ? `${chosen}.zip` : flavour === 'postman' ? `${chosen}.postman_collection.json` : `${chosen}.json`;
			path = await saveDialog({ defaultPath }).catch(() => null);
			if (!path) return;
		}
		busy = true;
		try {
			const done = await api.share({
				stem: chosen,
				format,
				environments: how === 'file' ? [] : envs,
				examples: how !== 'file' && examples,
				history: how === 'zip' && history,
				path
			});
			notice.ok(how === 'commit' ? `Committed ${done.slice(0, 7)}: Calls: update ${collection?.name ?? chosen}` : `Saved ${done}`);
			onclose();
		} catch (cause) {
			notice.failed('Could not share it', cause);
		} finally {
			busy = false;
		}
	}
</script>

<Sheet labelledby="share-title" width="640px" ondismiss={onclose}>
	<div class="share">
		<header>
			<h2 id="share-title">Share</h2>
			{#if readable.length > 1}
				<select aria-label="Collection" value={chosen ?? ''} onchange={(event) => (chosen = event.currentTarget.value)}>
					{#each readable as stored (stored.stem)}
						<option value={stored.stem}>{stored.value?.name}</option>
					{/each}
				</select>
			{:else if collection}
				<span class="name">{collection.name}</span>
			{/if}
		</header>

		<div class="ways" role="radiogroup" aria-label="How">
			{#each WAYS as way (way.id)}
				<button
					type="button"
					role="radio"
					class="way"
					class:on={how === way.id}
					aria-checked={how === way.id}
					onclick={() => (how = way.id)}><strong>{way.title}</strong><span>{way.note}</span></button
				>
			{/each}
		</div>

		{#if how === 'file'}
			<div class="choices" role="radiogroup" aria-label="Format">
				<button type="button" role="radio" class="choice" class:on={flavour === 'spagitty'} aria-checked={flavour === 'spagitty'} onclick={() => (flavour = 'spagitty')}>Spagitty</button>
				<button type="button" role="radio" class="choice" class:on={flavour === 'postman'} aria-checked={flavour === 'postman'} onclick={() => (flavour = 'postman')}>Postman 2.1</button>
			</div>
			{#if flavour === 'postman'}
				<p class="hint">Keep rules become <code>pm.environment.set</code> lines; a SOAP call is a raw POST of its envelope.</p>
			{/if}
		{:else}
			<fieldset>
				<legend>What goes in</legend>
				<label class="check"><input type="checkbox" checked disabled /> The calls</label>
				{#each environments.filter((stored) => stored.value) as stored (stored.stem)}
					<label class="check"><input type="checkbox" checked={envs.includes(stored.stem)} onchange={() => toggleEnv(stored.stem)} /> Environment {stored.value?.name}</label>
				{/each}
				<label class="check"><input type="checkbox" bind:checked={examples} /> The last response of each call, as an example</label>
				{#if how === 'zip'}
					<label class="check"><input type="checkbox" bind:checked={history} /> History</label>
				{/if}
			</fieldset>
		{/if}

		<p class="safe">Secrets and kept values go in empty, in every format. {how === 'zip' ? 'The README lists what to fill in.' : ''}</p>

		<div class="preview" aria-label="What is written">
			{#if planError}
				<p class="error" role="alert">{planError}</p>
			{:else if plan}
				<ul class="mono">
					{#each plan.files as file (file.path)}
						<li><span>{file.path}</span><span class="size">{formatSize(file.size)}</span></li>
					{/each}
				</ul>
			{/if}
		</div>

		<footer>
			<span class="gap"></span>
			<button type="button" class="act" onclick={onclose}>Cancel</button>
			<button type="button" class="act primary" disabled={!chosen || busy || !plan} onclick={go}>{label}</button>
		</footer>
	</div>
</Sheet>

<style>
	.share {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 16px 18px;
	}

	header {
		display: flex;
		align-items: baseline;
		gap: 10px;
	}

	h2 {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: normal;
	}

	.name {
		color: var(--muted);
	}

	.ways {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 8px;
	}

	.way {
		display: flex;
		flex-direction: column;
		gap: 3px;
		padding: 10px 12px;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		text-align: left;
	}

	.way span,
	.hint,
	.safe,
	.size,
	legend {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.way.on {
		border-color: color-mix(in srgb, var(--accent) 60%, transparent);
		background: color-mix(in srgb, var(--accent) 10%, var(--surface));
	}

	.choices {
		display: flex;
		gap: 4px;
	}

	.choice {
		padding: 3px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
	}

	.choice.on {
		border-color: color-mix(in srgb, var(--accent) 60%, transparent);
		color: var(--accent);
	}

	fieldset {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin: 0;
		padding: 0;
		border: 0;
	}

	.check {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.hint,
	.safe {
		margin: 0;
	}

	.preview {
		max-height: 180px;
		overflow: auto;
		padding: 8px 10px;
		border-radius: var(--r-field);
		background: var(--sunken);
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
		font-size: var(--fs-mono);
	}

	li {
		display: flex;
		justify-content: space-between;
		gap: 12px;
	}

	.error {
		margin: 0;
		color: var(--danger);
	}

	footer {
		display: flex;
		gap: 8px;
	}

	.gap {
		flex: 1;
	}

	.act {
		padding: 5px 14px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
	}

	.act.primary {
		border-color: transparent;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	.act:disabled {
		opacity: 0.5;
	}

	code {
		font-family: var(--font-mono);
	}
</style>
