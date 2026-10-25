<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import * as appApi from '$lib/api';
	import { isMac } from '$lib/platform';
	import { repo } from '$lib/repo.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import CollectionTree from './CollectionTree.svelte';
	import HistoryPanel from './HistoryPanel.svelte';
	import { importer } from './import.svelte';
	import { envColour, findCall, lookup, methodColour, methodOf } from './model';
	import RequestEditor from './RequestEditor.svelte';
	import ShareDialog from './ShareDialog.svelte';
	import SoapEditor from './SoapEditor.svelte';
	import ResponseView from './ResponseView.svelte';
	import { calls, keyOf } from './store.svelte';
	import type { HistoryEntry, Opened } from './types';
	import VariablesPanel from './VariablesPanel.svelte';

	/**
	 * Calls (2C, FEAT-160): requests saved beside the code.
	 *
	 * Collections on the left, the request above its response in the middle,
	 * the active environment's variables on the right. A chip in the header
	 * for each open call; `Ctrl+W` or a middle-click closes one. Requests are
	 * made in Rust; this screen only shows what comes back.
	 */

	const info = $derived(repo.info);
	const current = $derived(calls.current);
	const environment = $derived(calls.environment);
	const mod = isMac() ? '⌘' : 'Ctrl';

	let search = $state<HTMLInputElement | null>(null);
	let url = $state<HTMLInputElement | HTMLTextAreaElement | null>(null);
	let envMenu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	/** This repository's calls: on arrival, and when another is opened. */
	$effect(() => {
		const path = info?.path ?? null;
		void repo.generation;
		untrack(() => {
			if (!path || !appApi.inTauri()) return;
			void (async () => {
				await calls.load(path);
				await importer.load(path);
			})();
		});
	});

	/** FEAT-162: endpoints the code declares that no collection has, as a card. */
	const card = $derived.by(() => {
		const missing = importer.notInACollection;
		if (!missing.length) return null;
		const files = [...new Set(missing.map((endpoint) => endpoint.file?.split('/').pop() ?? endpoint.group))];
		return { count: missing.length, files };
	});
	const askScan = $derived(
		calls.settings?.scanOnOpen === 'ask' && !importer.found && !importer.scanning && !importer.dismissed
	);

	/** Every name a field can offer: what each scope defines. */
	const names = $derived(
		[
			...(environment?.vars ?? []),
			...calls.shared.vars,
			...(current?.collection.vars ?? [])
		].map((variable) => variable.name)
	);

	/** Where a name is defined, with a kept value's value from the keychain. */
	const find = (name: string) => {
		const found = lookup(name, environment, calls.shared, current?.collection ?? null);
		if (!found?.variable?.kept) return found;
		const view = calls.private.kept.find((kept) => kept.name === name);
		return { ...found, variable: { ...found.variable, value: view?.value ?? undefined } };
	};

	let envButton = $state<HTMLButtonElement | null>(null);
	/** FEAT-164: the Share dialog is open. */
	let sharing = $state(false);

	const envItems = $derived<MenuItem[]>([
		...calls.environments.map((stored) => ({
			id: stored.stem,
			label: stored.value?.name ?? stored.stem,
			note: stored.stem === calls.person.active ? '✓' : undefined,
			disabled: !stored.value,
			reason: stored.error ? 'could not be read' : undefined,
			run: () => calls.setActive(stored.stem)
		})),
		...(calls.environments.length ? [{ separator: true } as const] : []),
		{
			id: '-',
			label: 'No environment',
			note: calls.person.active === null ? '✓' : undefined,
			run: () => calls.setActive(null)
		}
	]);

	function openEnv(event: { currentTarget: EventTarget | null }) {
		const anchor = event.currentTarget as HTMLElement;
		if (envMenu?.anchor === anchor) {
			envMenu = null;
			return;
		}
		const box = anchor.getBoundingClientRect();
		envMenu = { x: box.left, y: box.bottom + 4, anchor };
	}

	function onkey(event: KeyboardEvent) {
		if (!info || !(event.ctrlKey || event.metaKey) || event.altKey) {
			if (event.key === 'Escape' && calls.asking) calls.dismissAsk();
			return;
		}
		if (event.key === 'Enter') {
			event.preventDefault();
			void calls.send(calls.asking !== null && current !== null && calls.asking === keyOf(current.opened));
		} else if (event.key === 'w' || event.key === 'W') {
			if (!calls.person.current) return;
			event.preventDefault();
			calls.close(calls.person.current);
		} else if (event.key === 'e' || event.key === 'E') {
			// FEAT-161: switch environment from anywhere in Calls.
			event.preventDefault();
			if (envButton) openEnv({ currentTarget: envButton });
		} else if (event.key === 'k' || event.key === 'K') {
			event.preventDefault();
			search?.focus();
			search?.select();
		}
	}

	function chipDown(event: MouseEvent, opened: Opened) {
		if (event.button === 1) {
			event.preventDefault();
			calls.close(opened);
		}
	}

	async function toggleHistory() {
		if (calls.history) calls.closeHistory();
		else await calls.loadHistory();
	}

	function openFromHistory(entry: HistoryEntry) {
		const stored = calls.collections.find((candidate) => candidate.stem === entry.collection);
		if (!stored?.value || !entry.callId || !findCall(stored.value.items, entry.callId)) return;
		calls.closeHistory();
		calls.open({ stem: stored.stem, id: entry.callId });
	}
</script>

<svelte:window onkeydown={onkey} />

{#if !info}
	<div class="screen">
		<EmptyState message="Open a repository to keep calls beside its code." />
	</div>
{:else}
	<div class="screen">
		<header class="head">
			<span class="title">Calls</span>
			<div class="chips" role="tablist" aria-label="Open calls">
				{#each calls.tabs as { opened, call } (keyOf(opened))}
					{@const on = current !== null && keyOf(current.opened) === keyOf(opened)}
					{@const method = methodOf(call)}
					<span class="chip" class:on>
						<button
							type="button"
							role="tab"
							class="open"
							aria-selected={on}
							title={call.url}
							onclick={() => calls.open(opened)}
							onauxclick={(event) => chipDown(event, opened)}
							onmousedown={(event) => event.button === 1 && event.preventDefault()}
							><span class="method mono" style:color={methodColour(method)}>{method}</span>
							{call.name}</button
						><button
							type="button"
							class="close"
							aria-label={`Close ${call.name}`}
							title={`Close (${mod}+W)`}
							onclick={() => calls.close(opened)}>✕</button
						>
					</span>
				{/each}
				<button type="button" class="chip add" aria-label="New call" title="New call" onclick={() => calls.create()}
					>+</button
				>
			</div>
			<span class="gap"></span>
			<button
				type="button"
				class="env"
				title={`The environment calls are sent in (${mod}+E)`}
				bind:this={envButton}
				onclick={openEnv}
			>
				<span class="muted">Env</span>
				{#if environment}
					<span class="dot" style:background={envColour(environment.color)} aria-hidden="true"></span>
					{environment.name}
				{:else}
					none
				{/if}
				<span class="muted">▾</span>
			</button>
			<button
				type="button"
				class="env"
				title="Bring calls in from the code, or from a Postman, Insomnia, Bruno, OpenAPI, .http, HAR or cURL file"
				onclick={() => goto('/calls/import')}>Import</button
			>
			<button
				type="button"
				class="env"
				title="A collection as one file, a zip, or a commit; secrets stay here"
				disabled={!calls.collections.some((stored) => stored.value)}
				onclick={() => (sharing = true)}>Share</button
			>
		</header>

		{#if calls.error}
			<p class="error" role="alert">{calls.error}</p>
		{/if}

		<div class="body">
			<CollectionTree
				collections={calls.collections}
				filter={calls.person.filter}
				collapsed={calls.person.collapsed}
				current={calls.person.current}
				today={calls.today}
				historyOpen={calls.history !== null}
				onfilter={(filter) => calls.setFilter(filter)}
				ontoggle={(id) => calls.toggleCollapsed(id)}
				onopen={(opened) => {
					calls.closeHistory();
					calls.open(opened);
				}}
				oncreate={(stem, folder) => calls.create(stem, folder)}
				onduplicate={(opened) => calls.duplicate(opened)}
				onremove={(opened) => calls.remove(opened)}
				onhistory={toggleHistory}
				bind:search
				found={calls.found}
				{card}
				{askScan}
				onscan={() => importer.scan()}
				onreview={() => goto('/calls/import')}
				ondismiss={() => importer.dismiss()}
			/>

			<main class="middle">
				{#if calls.history}
					<div class="card grow">
						<HistoryPanel
							entries={calls.history}
							onclose={() => calls.closeHistory()}
							onclear={() => calls.clearHistory()}
							onopen={openFromHistory}
						/>
					</div>
				{:else if current}
					{@const opened = current.opened}
					<div class="card">
						<input
							class="name"
							type="text"
							aria-label="The call's name"
							spellcheck="false"
							value={current.call.name}
							oninput={(event) => {
								const name = event.currentTarget.value;
								calls.edit(opened, (call) => ({ ...call, name }));
							}}
						/>
						{#if current.call.kind === 'soap'}
							<SoapEditor
								call={current.call}
								wsdl={current.collection.wsdl ?? null}
								onchange={(change) => calls.edit(opened, change)}
								{find}
								{names}
								sending={calls.sendingFor(opened)}
								refusal={calls.refusalFor(opened)}
								onsend={() => calls.send()}
								oncancel={() => calls.cancel(opened)}
								bind:url
							/>
						{:else}
							<RequestEditor
								call={current.call}
								onchange={(change) => calls.edit(opened, change)}
								{find}
								{names}
								sending={calls.sendingFor(opened)}
								refusal={calls.refusalFor(opened)}
								onsend={() => calls.send()}
								oncancel={() => calls.cancel(opened)}
								bind:url
							/>
						{/if}
						{#if environment && calls.asking === keyOf(opened)}
							<div class="ask" role="alertdialog" aria-label="Ask before sending" style:--env={envColour(environment.color)}>
								<span class="dot" aria-hidden="true"></span>
								<span>Send <strong>{current.call.name}</strong> to <strong>{environment.name}</strong>?</span>
								<span class="gap"></span>
								<button type="button" class="yes" onclick={() => calls.send(true)}>Send</button>
								<button type="button" class="no" onclick={() => calls.dismissAsk()}>Not now</button>
							</div>
						{/if}
					</div>
					<div class="card grow">
						<ResponseView
							outcome={calls.outcomeFor(opened)}
							sending={calls.sendingFor(opened)}
							beautify={calls.settings?.beautify ?? true}
							name={current.call.name}
							ontoggleraw={() => calls.toggleRaw(opened)}
							onsave={(path) => calls.saveBody(opened, path)}
							rules={current.call.keep ?? []}
							{environment}
							keepEvery={calls.settings?.keepEvery ?? true}
							suggest={calls.settings?.suggestNames ?? true}
							onkeep={calls.settings?.clickToKeep === false ? undefined : (ask) => calls.keep(opened, ask)}
							onforget={(name) => calls.forget(opened, name)}
							soap={current.call.kind === 'soap'}
						/>
					</div>
				{:else if calls.loaded}
					<div class="card grow">
						<EmptyState message="Open a call on the left, or press + for a new one." />
					</div>
				{/if}
			</main>

			<VariablesPanel
				{environment}
				shared={calls.shared}
				collection={current?.collection ?? null}
				privateValues={calls.private}
				privateError={calls.privateError}
				onmanage={() => goto('/calls/environments')}
			/>
		</div>
	</div>
{/if}

{#if sharing}
	<ShareDialog
		collections={calls.collections}
		environments={calls.environments}
		stem={current && !current.opened.stem.startsWith('found:') ? current.opened.stem : null}
		shareAs={calls.settings?.shareAs ?? 'zip'}
		onclose={() => (sharing = false)}
	/>
{/if}

{#if envMenu}
	<Menu
		x={envMenu.x}
		y={envMenu.y}
		anchor={envMenu.anchor}
		label="Environment"
		items={envItems}
		onclose={() => (envMenu = null)}
	/>
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
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		padding: 12px 16px 10px;
	}

	.title {
		margin-right: 6px;
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		min-width: 0;
	}

	.chip {
		display: inline-flex;
		align-items: center;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		background-color: var(--surface-veil);
		white-space: nowrap;
	}

	.chip:hover {
		background-color: var(--hover);
	}

	.chip.on {
		border-color: color-mix(in srgb, var(--accent) 62%, transparent);
		background-color: color-mix(in srgb, var(--accent) 14%, var(--surface));
	}

	.chip .open {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 4px 3px 10px;
	}

	.chip.on .open {
		color: var(--accent);
	}

	.chip .method {
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	.chip .close {
		padding: 3px 9px 3px 5px;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.chip .close:hover {
		color: var(--ink);
	}

	.chip.add {
		padding: 3px 10px;
		color: var(--muted);
	}

	.gap {
		flex: 1;
	}

	.env {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
	}

	.env:hover {
		background: var(--hover);
	}

	.muted {
		color: var(--muted);
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
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

	.middle {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.card {
		flex: none;
		max-height: 55%;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.card.grow {
		flex: 1;
		max-height: none;
		min-height: 0;
	}

	.name {
		align-self: flex-start;
		min-width: 200px;
		padding: 2px 6px;
		border: 1px solid transparent;
		background: transparent;
		font-weight: 600;
	}

	.name:hover,
	.name:focus {
		border-color: var(--line);
	}

	.ask {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: 1px solid color-mix(in srgb, var(--env) 55%, transparent);
		border-radius: var(--r-field);
		background: color-mix(in srgb, var(--env) 12%, var(--surface));
	}

	.ask .dot {
		background: var(--env);
	}

	.ask strong {
		color: var(--env);
	}

	.ask .yes,
	.ask .no {
		padding: 3px 12px;
		border-radius: var(--r-button);
	}

	.ask .yes {
		background: var(--env);
		color: var(--on-accent);
		font-weight: 600;
	}

	.ask .no {
		border: 1px solid var(--soft);
	}
</style>
