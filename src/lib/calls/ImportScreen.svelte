<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { goto } from '$app/navigation';
	import * as appApi from '$lib/api';
	import { repo } from '$lib/repo.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import { CODE_KINDS, importer, type Filter } from './import.svelte';
	import { methodColour } from './model';
	import { calls } from './store.svelte';
	import type { Endpoint, Source, SourceKind } from './types';

	/**
	 * Calls › Import (FEAT-162, proposal §8.1).
	 *
	 * Sources on the left: the code, the collection files in the repository,
	 * and what comes from outside. Endpoints in the middle, by controller or
	 * folder, each *New*, *Changed* or *In* a collection, with *New* and
	 * *Changed* ticked. *Bring in* on the right says where they go and how,
	 * and its button says how many and where. Nothing is written until it is
	 * pressed, and then only the collection.
	 */

	const KIND_LABELS: Record<SourceKind, string> = {
		spring: 'Spring',
		jaxRs: 'JAX-RS',
		nest: 'NestJS',
		jaxWs: 'JAX-WS',
		springWs: 'Spring-WS',
		wsdl: 'WSDL',
		openApi: 'OpenAPI',
		postman: 'Postman',
		insomnia: 'Insomnia',
		bruno: 'Bruno',
		http: '.http',
		har: 'HAR',
		curl: 'cURL',
		spagitty: 'Spagitty'
	};
	const FILTERS: { id: Filter; label: string }[] = [
		{ id: 'all', label: 'All' },
		{ id: 'new', label: 'New' },
		{ id: 'changed', label: 'Changed' },
		{ id: 'in', label: 'Already in' }
	];

	const info = $derived(repo.info);
	const found = $derived(importer.found);
	const sources = $derived(found?.sources ?? []);
	const code = $derived(sources.filter((source) => CODE_KINDS.includes(source.kind)));
	const files = $derived(sources.filter((source) => !CODE_KINDS.includes(source.kind) && source.inRepository));
	const outside = $derived(sources.filter((source) => !source.inRepository));
	const counts = $derived(importer.counts());
	const choices = $derived(importer.choices);
	const readable = $derived(calls.collections.filter((stored) => stored.value));
	const picked = $derived(importer.selected.length);

	let pasting = $state(false);
	let pasted = $state('');
	let wsdlUrl = $state<string | null>(null);

	async function readWsdlUrl() {
		if (wsdlUrl && (await importer.importWsdlUrl(wsdlUrl))) wsdlUrl = null;
	}

	$effect(() => {
		const path = info?.path ?? null;
		void repo.generation;
		untrack(() => {
			if (!path || !appApi.inTauri()) return;
			void (async () => {
				if (!calls.loaded) await calls.load(path);
				await importer.load(path);
			})();
		});
	});

	/** The endpoints shown, by source and then by group, in order. */
	const groups = $derived.by(() => {
		const out: { key: string; group: string; file: string | null; endpoints: Endpoint[] }[] = [];
		for (const endpoint of importer.visible) {
			const key = `${endpoint.source}\u0000${endpoint.group}`;
			let group = out.find((g) => g.key === key);
			if (!group) {
				group = { key, group: endpoint.group, file: endpoint.file, endpoints: [] };
				out.push(group);
			}
			group.endpoints.push(endpoint);
		}
		return out;
	});

	const target = $derived(
		choices.as === 'new'
			? `Create collection ${choices.name.trim() || '…'} with ${picked}`
			: `Add ${picked} to ${readable.find((stored) => stored.stem === choices.into)?.value?.name ?? '…'}`
	);
	const ready = $derived(
		picked > 0 && !importer.bringing && (choices.as === 'new' ? choices.name.trim() !== '' : choices.into !== null)
	);

	async function chooseFile() {
		const path = await openDialog({
			multiple: false,
			directory: false,
			filters: [
				{ name: 'Collections', extensions: ['json', 'yaml', 'yml', 'http', 'rest', 'har', 'bru', 'txt', 'sh'] },
				{ name: 'Every file', extensions: ['*'] }
			]
		}).catch(() => null);
		if (typeof path === 'string') await importer.importFile(path);
	}

	async function readPasted() {
		if (await importer.importText(pasted)) {
			pasted = '';
			pasting = false;
		}
	}

	function tag(endpoint: Endpoint): { text: string; tone: string; title: string } {
		const status = endpoint.status;
		if (status.kind === 'new') return { text: 'New', tone: 'ok', title: 'In no collection yet' };
		const name = calls.collections.find((stored) => stored.stem === status.collection)?.value?.name ?? status.collection;
		if (status.kind === 'changed') return { text: 'Changed', tone: 'warn', title: status.what };
		return { text: `In ${name}`, tone: 'muted', title: `Already in ${name}, the same shape` };
	}
</script>

{#snippet sourceRow(source: Source)}
	<button
		type="button"
		class="source"
		class:on={importer.source === source.id}
		aria-pressed={importer.source === source.id}
		title={source.file ?? source.label}
		onclick={() => importer.setSource(importer.source === source.id ? null : source.id)}
	>
		<span class="kind">{KIND_LABELS[source.kind]}</span>
		<span class="label">{source.label}</span>
		<span class="count">{source.count}</span>
	</button>
{/snippet}

{#if !info}
	<div class="screen"><EmptyState message="Open a repository to import calls into it." /></div>
{:else}
	<div class="screen">
		<header class="head">
			<nav class="crumbs" aria-label="Where you are">
				<a href="/calls">Calls</a>
				<span class="sep" aria-hidden="true">›</span>
				<span>Import</span>
			</nav>
			<span class="gap"></span>
			{#if importer.scanning}
				<span class="note" role="status">Reading files… {importer.scanning.files.toLocaleString()}</span>
				<button type="button" class="act" onclick={() => importer.cancel()}>Cancel</button>
			{:else}
				<button type="button" class="act" onclick={() => importer.scan()}>{found ? 'Scan again' : 'Scan the code'}</button>
			{/if}
			<button type="button" class="act" onclick={() => goto('/calls')}>Done</button>
		</header>
		{#if importer.error}
			<p class="error" role="alert">{importer.error}</p>
		{/if}

		<div class="body">
			<aside class="panel sources" aria-label="Sources">
				<button type="button" class="source all" class:on={importer.source === null} onclick={() => importer.setSource(null)}>
					<span class="label">Every source</span>
					<span class="count">{found?.endpoints.length ?? 0}</span>
				</button>

				<h3>Scan the code</h3>
				{#each code as source (source.id)}
					{@render sourceRow(source)}
				{:else}
					<p class="hint">
						{found
							? 'No Spring, JAX-RS, NestJS, SOAP or OpenAPI endpoints found.'
							: 'Reads Spring, JAX-RS and NestJS controllers, JAX-WS and Spring-WS services, WSDL and OpenAPI files. Nothing is built or run.'}
					</p>
				{/each}

				<h3>Collection files in this repository</h3>
				{#each files as source (source.id)}
					{@render sourceRow(source)}
				{:else}
					<p class="hint">{found ? 'None found.' : 'Postman, Insomnia, Bruno, .http and HAR files are found by the scan.'}</p>
				{/each}

				<h3>From outside</h3>
				{#each outside as source (source.id)}
					{@render sourceRow(source)}
				{/each}
				<button type="button" class="add" onclick={chooseFile}>Choose a file…</button>
				{#if pasting}
					<textarea
						class="mono"
						rows="5"
						aria-label="A cURL command"
						placeholder="curl https://…"
						spellcheck="false"
						bind:value={pasted}
					></textarea>
					<span class="row">
						<button type="button" class="act" onclick={() => (pasting = false)}>Cancel</button>
						<button type="button" class="act primary" disabled={!pasted.trim()} onclick={readPasted}>Read it</button>
					</span>
				{:else}
					<button type="button" class="add" onclick={() => (pasting = true)}>Paste cURL</button>
				{/if}
				{#if wsdlUrl !== null}
					<input
						class="mono"
						type="url"
						aria-label="A WSDL's address"
						placeholder="https://…/service?wsdl"
						spellcheck="false"
						bind:value={wsdlUrl}
						onkeydown={(event) => event.key === 'Enter' && readWsdlUrl()}
					/>
					<span class="row">
						<button type="button" class="act" onclick={() => (wsdlUrl = null)}>Cancel</button>
						<button type="button" class="act primary" disabled={!wsdlUrl?.trim()} onclick={readWsdlUrl}>Read it</button>
					</span>
				{:else}
					<button
						type="button"
						class="add"
						title="Fetched once, as a call is; schemas it imports by address are not fetched"
						onclick={() => (wsdlUrl = '')}>WSDL from a URL…</button
					>
				{/if}
			</aside>

			<main class="panel endpoints" aria-label="Endpoints">
				<div class="filters" role="radiogroup" aria-label="Show">
					{#each FILTERS as item (item.id)}
						<button
							type="button"
							role="radio"
							class="filter"
							class:on={importer.filter === item.id}
							aria-checked={importer.filter === item.id}
							onclick={() => importer.setFilter(item.id)}>{item.label} <span class="count">{counts[item.id]}</span></button
						>
					{/each}
					<span class="gap"></span>
					{#if found?.capped}<span class="note">Stopped at 20,000 files.</span>{/if}
				</div>
				{#if found?.scripted}
					<p class="note">
						{found.scripted}
						{found.scripted === 1 ? 'call had' : 'calls had'} Postman script lines that are not run; they are in each call's notes.
					</p>
				{/if}
				<div class="list">
					{#each groups as group (group.key)}
						{@const ids = group.endpoints.map((endpoint) => endpoint.id)}
						<section class="group">
							<label class="group-head">
								<input
									type="checkbox"
									aria-label={`Every endpoint in ${group.group}`}
									checked={ids.every((id) => importer.selected.includes(id))}
									onchange={() => importer.toggleAll(ids)}
								/>
								<span class="name">{group.group}</span>
								{#if group.file}<span class="file mono">{group.file}</span>{/if}
							</label>
							{#each group.endpoints as endpoint (endpoint.id)}
								{@const shown = tag(endpoint)}
								<label class="endpoint">
									<input
										type="checkbox"
										aria-label={`${endpoint.method} ${endpoint.path}`}
										checked={importer.selected.includes(endpoint.id)}
										onchange={() => importer.toggle(endpoint.id)}
									/>
									<span class="method mono" style:color={methodColour(endpoint.method)}>{endpoint.method}</span>
									<span class="path mono" title={endpoint.call.name}>{endpoint.path}</span>
									<span class="hint-text">{endpoint.status.kind === 'changed' ? endpoint.status.what : endpoint.hint}</span>
									<span class="tag {shown.tone}" title={shown.title}>{shown.text}</span>
								</label>
							{/each}
						</section>
					{:else}
						<EmptyState
							message={importer.scanning
								? 'Reading the working tree…'
								: found
									? 'Nothing here with these filters.'
									: 'Scan the code, or choose a file, to find endpoints to bring in.'}
						/>
					{/each}
				</div>
			</main>

			<aside class="panel bring" aria-label="Bring in">
				<h3>Bring in</h3>
				<div class="choices" role="radiogroup" aria-label="As">
					<button
						type="button"
						role="radio"
						class="choice"
						class:on={choices.as === 'into'}
						aria-checked={choices.as === 'into'}
						disabled={readable.length === 0}
						onclick={() => importer.choose({ as: 'into' })}>Calls in a collection</button
					>
					<button
						type="button"
						role="radio"
						class="choice"
						class:on={choices.as === 'new'}
						aria-checked={choices.as === 'new'}
						onclick={() => importer.choose({ as: 'new' })}>New collection</button
					>
				</div>
				{#if choices.as === 'into'}
					<label class="field"
						><span>Into</span>
						<select
							aria-label="Into"
							value={choices.into ?? ''}
							onchange={(event) => importer.choose({ into: event.currentTarget.value || null })}
						>
							{#each readable as stored (stored.stem)}
								<option value={stored.stem}>{stored.value?.name}</option>
							{/each}
						</select></label
					>
				{:else}
					<label class="field"
						><span>Name</span>
						<input
							type="text"
							aria-label="The new collection's name"
							spellcheck="false"
							value={choices.name}
							oninput={(event) => importer.choose({ name: event.currentTarget.value })}
						/></label
					>
				{/if}
				<div class="choices" role="radiogroup" aria-label="Folders">
					<button
						type="button"
						role="radio"
						class="choice"
						class:on={choices.folders === 'perGroup'}
						aria-checked={choices.folders === 'perGroup'}
						onclick={() => importer.choose({ folders: 'perGroup' })}>One per controller</button
					>
					<button
						type="button"
						role="radio"
						class="choice"
						class:on={choices.folders === 'byPath'}
						aria-checked={choices.folders === 'byPath'}
						onclick={() => importer.choose({ folders: 'byPath' })}>By path</button
					>
				</div>
				<label class="switch" title="A sample of the request class's fields, to a depth of four">
					<input
						type="checkbox"
						role="switch"
						checked={choices.bodies}
						onchange={(event) => importer.choose({ bodies: event.currentTarget.checked })}
					/>
					Example bodies from request classes
				</label>
				<label class="switch" title="/orders/{'{'}id{'}'} is sent as /orders/{'{{'}id{'}}'}">
					<input
						type="checkbox"
						role="switch"
						checked={choices.pathVars}
						onchange={(event) => importer.choose({ pathVars: event.currentTarget.checked })}
					/>
					Path variables become variables
				</label>
				{#if found?.baseUrl}
					<label class="switch" title={`A local environment with baseUrl ${found.baseUrl}, when there is none`}>
						<input
							type="checkbox"
							role="switch"
							checked={choices.serverAddress}
							onchange={(event) => importer.choose({ serverAddress: event.currentTarget.checked })}
						/>
						Server address from application.yml
					</label>
					<p class="hint mono">{found.baseUrl}</p>
				{/if}
				<span class="grow"></span>
				<button type="button" class="go" disabled={!ready} onclick={() => importer.bringIn()}>{target}</button>
				<p class="hint">A changed call gains its new fields and loses the old ones; what you typed stays.</p>
			</aside>
		</div>
	</div>
{/if}

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 16px 10px;
	}

	.crumbs {
		display: flex;
		align-items: baseline;
		gap: 8px;
		font-size: var(--fs-title);
	}

	.crumbs a {
		color: var(--muted);
		text-decoration: none;
	}

	.crumbs a:hover {
		color: var(--ink);
	}

	.sep,
	.note,
	.hint,
	.count,
	.file,
	.hint-text {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.gap,
	.grow {
		flex: 1;
	}

	.act {
		padding: 4px 14px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
	}

	.act:hover:not(:disabled) {
		background: var(--hover);
	}

	.act.primary,
	.go {
		border: 0;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	.error {
		margin: 0;
		padding: 0 16px 8px;
		color: var(--danger);
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 12px 12px;
	}

	.panel {
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: auto;
	}

	.sources {
		flex: none;
		width: 250px;
	}

	.endpoints {
		flex: 1;
		min-width: 0;
	}

	.bring {
		flex: none;
		width: 280px;
		gap: 10px;
	}

	h3 {
		margin: 10px 0 2px;
		color: var(--muted);
		font-size: var(--fs-secondary);
		font-weight: normal;
	}

	.bring h3 {
		margin-top: 0;
		color: var(--ink);
		font-size: var(--fs-ui);
		font-weight: 600;
	}

	.hint {
		margin: 0;
	}

	.source {
		display: flex;
		align-items: baseline;
		gap: 8px;
		padding: 5px 8px;
		border-radius: var(--r-row);
		text-align: left;
	}

	.source:hover,
	.source.on {
		background: var(--surface-2);
	}

	.source .kind {
		flex: none;
		color: var(--agent);
		font-size: var(--fs-secondary);
	}

	.source .label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.add {
		align-self: flex-start;
		padding: 4px 8px;
		color: var(--muted);
	}

	.add:hover {
		color: var(--ink);
	}

	textarea {
		resize: vertical;
		font-size: var(--fs-mono);
	}

	.row {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}

	.filters {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.filter {
		padding: 3px 10px;
		border-radius: var(--r-pill);
		color: var(--muted);
	}

	.filter.on {
		background: var(--surface-2);
		color: var(--ink);
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.group-head,
	.endpoint {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 4px 6px;
		border-radius: var(--r-row);
	}

	.group-head .name {
		font-weight: 600;
	}

	.endpoint:hover {
		background: var(--hover);
	}

	.method {
		flex: none;
		width: 56px;
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	.path {
		flex: none;
		max-width: 45%;
		overflow: hidden;
		font-size: var(--fs-mono);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.hint-text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tag {
		flex: none;
		padding: 1px 8px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
	}

	.tag.ok {
		background: color-mix(in srgb, var(--ok) 15%, transparent);
		color: var(--ok);
	}

	.tag.warn {
		background: color-mix(in srgb, var(--warn) 15%, transparent);
		color: var(--warn);
	}

	.tag.muted {
		color: var(--muted);
	}

	.choices {
		display: flex;
		flex-wrap: wrap;
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

	.field {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.field > span {
		width: 44px;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.field select,
	.field input {
		flex: 1;
		min-width: 0;
		padding: 4px 8px;
	}

	.switch {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.go {
		padding: 8px 12px;
		border-radius: var(--r-button);
	}

	.go:disabled {
		opacity: 0.5;
	}
</style>
