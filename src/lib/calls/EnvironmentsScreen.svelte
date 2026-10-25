<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import * as appApi from '$lib/api';
	import { relativeTime } from '$lib/format';
	import { isMac } from '$lib/platform';
	import { repo } from '$lib/repo.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import { envColour } from './model';
	import { calls } from './store.svelte';
	import type { Colour, Environment, Private, Variable } from './types';
	import VarTable from './VarTable.svelte';

	/**
	 * Calls › Environments (FEAT-161, proposal §6.1).
	 *
	 * The environments on the left, one of them in the middle with its
	 * variables and the ones every environment shares, and on the right the
	 * values Calls keeps from responses in it, each with a switch that freezes
	 * it. Plain values are in the repository; secrets and kept values are in
	 * this computer's keychain, and the footer says which file and which
	 * keychain.
	 */

	const COLOURS: { id: Colour; label: string }[] = [
		{ id: 'ok', label: 'Green' },
		{ id: 'warn', label: 'Amber' },
		{ id: 'danger', label: 'Red, for production' },
		{ id: 'blue', label: 'Blue' },
		{ id: 'teal', label: 'Teal' }
	];

	const info = $derived(repo.info);
	const mod = isMac() ? '⌘' : 'Ctrl';

	let selected = $state<string | null>(null);
	let envPrivate = $state<Private | null>(null);
	let sharedPrivate = $state<Private | null>(null);
	let making = $state(false);
	let newName = $state('');
	let renaming = $state<string | null>(null);

	$effect(() => {
		const path = info?.path ?? null;
		void repo.generation;
		untrack(() => {
			if (!path || !appApi.inTauri() || calls.loaded) return;
			void calls.load(path);
		});
	});

	const readable = $derived(calls.environments.filter((stored) => stored.value));
	const chosen = $derived(
		calls.environments.find((stored) => stored.stem === selected && stored.value) ??
			calls.environments.find((stored) => stored.stem === calls.person.active && stored.value) ??
			readable[0] ??
			null
	);
	const env = $derived(chosen?.value ?? null);

	/** Each environment's private side, read when it is chosen. */
	$effect(() => {
		const stem = chosen?.stem ?? null;
		void calls.environments;
		untrack(() => void readPrivate(stem));
	});

	$effect(() => {
		void calls.shared;
		untrack(() => {
			calls
				.privateOf(null)
				.then((read) => (sharedPrivate = read))
				.catch(() => (sharedPrivate = null));
		});
	});

	let seq = 0;
	async function readPrivate(stem: string | null) {
		const ticket = ++seq;
		if (!stem) {
			envPrivate = null;
			return;
		}
		const read = await calls.privateOf(stem).catch(() => null);
		if (ticket === seq) envPrivate = read;
	}

	/** The host an environment talks to, from its first URL-looking value. */
	function hostOf(environment: Environment): string {
		const named = environment.vars.find((variable) => /^(baseUrl|host|url|endpoint)$/i.test(variable.name));
		const value = (named ?? environment.vars.find((variable) => /^https?:\/\//.test(variable.value ?? '')))?.value;
		if (!value) return '';
		return value.replace(/^\w+:\/\//, '').replace(/\/.*$/, '');
	}

	function unique(name: string): string {
		let out = name;
		for (let n = 2; calls.environments.some((stored) => stored.stem === out); n++) out = `${name}-${n}`;
		return out;
	}

	/** A name an environment's file can have: lower case, words joined by `-`. */
	function cleanName(name: string): string {
		return name
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9._]+/g, '-')
			.replace(/^[-.]+|[-.]+$/g, '');
	}

	async function create() {
		const name = cleanName(newName);
		if (!name) return;
		const colours: Colour[] = ['ok', 'warn', 'blue', 'teal'];
		const colour = name === 'production' ? 'danger' : colours[calls.environments.length % colours.length];
		const made = unique(name);
		if (await calls.createEnvironment(made, colour)) {
			selected = made;
			making = false;
			newName = '';
		}
	}

	async function duplicate() {
		if (!chosen?.value) return;
		const name = unique(`${chosen.stem}-copy`);
		if (await calls.createEnvironment(name, chosen.value.color, chosen.stem)) selected = name;
	}

	async function rename(to: string) {
		renaming = null;
		if (!chosen) return;
		const name = cleanName(to);
		if (!name || name === chosen.stem) return;
		if (await calls.renameEnvironment(chosen.stem, name)) selected = name;
	}

	function setVars(vars: Variable[]) {
		if (chosen) calls.editEnvironment(chosen.stem, (value) => ({ ...value, vars }));
	}

	function onkey(event: KeyboardEvent) {
		if (event.key === 'Escape' && !making && renaming === null) {
			const target = event.target as HTMLElement | null;
			if (target?.closest('input, textarea')) return;
			void goto('/calls');
		}
	}
</script>

<svelte:window onkeydown={onkey} />

{#if !info}
	<div class="screen">
		<EmptyState message="Open a repository to manage its environments." />
	</div>
{:else}
	<div class="screen">
		<header class="head">
			<nav class="crumbs" aria-label="Where you are">
				<a href="/calls">Calls</a>
				<span class="sep" aria-hidden="true">›</span>
				<span class="here">Environments</span>
			</nav>
			<span class="gap"></span>
			<button type="button" class="done" onclick={() => goto('/calls')}>Done</button>
		</header>

		<div class="body">
			<aside class="list" aria-label="Environments">
				{#each calls.environments as stored (stored.stem)}
					{#if stored.value}
						<button
							type="button"
							class="env"
							class:on={stored.stem === chosen?.stem}
							aria-current={stored.stem === chosen?.stem ? 'true' : undefined}
							onclick={() => (selected = stored.stem)}
						>
							<span class="dot" style:background={envColour(stored.value.color)} aria-hidden="true"></span>
							<span class="what">
								<span class="name">{stored.value.name}</span>
								<span class="host"
									>{hostOf(stored.value)}{stored.value.askBeforeSending
										? `${hostOf(stored.value) ? ' · ' : ''}asks first`
										: ''}</span
								>
							</span>
							{#if stored.stem === calls.person.active}<span class="use">in use</span>{/if}
						</button>
					{:else}
						<div class="env broken" title={stored.error ?? ''}>
							<span class="dot" aria-hidden="true"></span>
							<span class="what"
								><span class="name">{stored.stem}</span><span class="host">Not read: {stored.error}</span></span
							>
						</div>
					{/if}
				{/each}
				{#if making}
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="new"
						type="text"
						aria-label="The new environment's name"
						placeholder="name, such as staging"
						spellcheck="false"
						autofocus
						bind:value={newName}
						onkeydown={(event) => {
							if (event.key === 'Enter') void create();
							if (event.key === 'Escape') {
								event.stopPropagation();
								making = false;
							}
						}}
						onblur={() => newName.trim() || (making = false)}
					/>
				{:else}
					<button type="button" class="add" onclick={() => (making = true)}>+ New environment</button>
				{/if}
				<span class="grow"></span>
				<p class="hint">Switch from anywhere in Calls with <kbd class="mono">{mod} E</kbd></p>
			</aside>

			<main class="one" aria-label={env ? env.name : 'No environment'}>
				{#if env && chosen}
					<div class="top">
						<span class="dot big" style:background={envColour(env.color)} aria-hidden="true"></span>
						{#if renaming !== null}
							<!-- svelte-ignore a11y_autofocus -->
							<input
								class="title-input"
								type="text"
								aria-label="The environment's name"
								spellcheck="false"
								autofocus
								bind:value={renaming}
								onkeydown={(event) => {
									if (event.key === 'Enter') void rename(renaming ?? '');
									if (event.key === 'Escape') {
										event.stopPropagation();
										renaming = null;
									}
								}}
								onblur={() => rename(renaming ?? '')}
							/>
						{:else}
							<button type="button" class="title" title="Rename" onclick={() => (renaming = env.name)}>{env.name}</button>
						{/if}
						<div class="swatches" role="radiogroup" aria-label="Colour">
							{#each COLOURS as colour (colour.id)}
								<button
									type="button"
									role="radio"
									class="swatch"
									class:on={env.color === colour.id}
									aria-checked={env.color === colour.id}
									aria-label={colour.label}
									title={colour.label}
									style:background={envColour(colour.id)}
									onclick={() => calls.editEnvironment(chosen.stem, (value) => ({ ...value, color: colour.id }))}
								></button>
							{/each}
						</div>
						<span class="gap"></span>
						<label class="ask">
							<input
								type="checkbox"
								role="switch"
								checked={env.askBeforeSending}
								onchange={(event) => {
									const on = event.currentTarget.checked;
									calls.editEnvironment(chosen.stem, (value) => ({ ...value, askBeforeSending: on }));
								}}
							/>
							Ask before sending
						</label>
						<button type="button" class="act" onclick={duplicate}>Duplicate</button>
						<button type="button" class="act danger" onclick={() => calls.deleteEnvironment(chosen.stem)}>Delete</button>
					</div>

					<div class="tables">
						<VarTable
							label={`${env.name}'s variables`}
							vars={env.vars}
							privateValues={envPrivate}
							onvars={setVars}
							onsecret={async (name, value) => {
								await calls.setSecret(chosen.stem, name, value);
								await readPrivate(chosen.stem);
							}}
							onreveal={(name) => calls.reveal(chosen.stem, name)}
							onforget={async (name) => {
								await calls.forgetIn(chosen.stem, name);
								await readPrivate(chosen.stem);
							}}
						/>

						<h3>Shared by every environment</h3>
						<VarTable
							label="Shared variables"
							vars={calls.shared.vars}
							privateValues={sharedPrivate}
							onvars={(vars) => calls.editShared((shared) => ({ ...shared, vars }))}
							onsecret={async (name, value) => {
								await calls.setSecret(null, name, value);
								sharedPrivate = await calls.privateOf(null).catch(() => null);
							}}
							onreveal={(name) => calls.reveal(null, name)}
							onforget={(name) => calls.forgetIn(null, name)}
						/>
					</div>

					<footer class="where">
						<span>Repository: <span class="mono">{chosen.file}</span></span>
						<span>This computer: secrets and kept values, in the system keychain</span>
					</footer>
				{:else if calls.loaded}
					<EmptyState message="No environments yet. Make one on the left: its plain values are kept in the repository." />
				{/if}
			</main>

			<aside class="kept" aria-label="Kept from responses">
				<h2>Kept from responses</h2>
				<p class="hint">Values Calls fills in for you after a call. Turn one off to freeze its value.</p>
				{#if env && chosen}
					{#each env.vars.filter((variable) => variable.kept) as variable (variable.name)}
						{@const view = envPrivate?.kept.find((kept) => kept.name === variable.name)}
						<div class="card">
							<div class="card-top">
								<span class="mono var">{variable.name}</span>
								<input
									type="checkbox"
									role="switch"
									aria-label={`Update ${variable.name} after each call`}
									checked={!variable.frozen}
									onchange={(event) => {
										const on = event.currentTarget.checked;
										calls.editEnvironment(chosen.stem, (value) => ({
											...value,
											vars: value.vars.map((other) =>
												other.name === variable.name ? { ...other, frozen: on ? undefined : true } : other
											)
										}));
									}}
								/>
							</div>
							{#if view?.call}<p>from <strong>{view.method} {view.call}</strong></p>{/if}
							{#if view?.from}<p class="mono path">{view.from}</p>{/if}
							<p class="hint" class:stale={view?.stale}>
								{#if variable.frozen}Frozen{:else if view?.stale}Stale: the last {view.call} found nothing{:else if view?.at}Updated
									{relativeTime(Math.floor(view.at / 1000))} in {env.name}{:else}No value yet{/if}
							</p>
						</div>
					{/each}
				{/if}
				<span class="grow"></span>
				<p class="hint">Add one by clicking any value in a response.</p>
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

	.sep {
		color: var(--muted);
	}

	.gap,
	.grow {
		flex: 1;
	}

	.done,
	.act {
		padding: 4px 14px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
	}

	.done:hover,
	.act:hover {
		background: var(--hover);
	}

	.act.danger {
		color: var(--danger);
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 12px 12px;
	}

	.list,
	.one,
	.kept {
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	.list {
		flex: none;
		width: 240px;
	}

	.one {
		flex: 1;
		min-width: 0;
		gap: 14px;
	}

	.kept {
		flex: none;
		width: 270px;
		overflow: auto;
	}

	.env {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 8px;
		border-radius: var(--r-row);
		text-align: left;
	}

	.env:hover,
	.env.on {
		background: var(--surface-2);
	}

	.what {
		display: flex;
		flex: 1;
		min-width: 0;
		flex-direction: column;
	}

	.name {
		font-weight: 600;
	}

	.host,
	.hint {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.host {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.broken .name {
		color: var(--danger);
	}

	.use {
		color: var(--ok);
		font-size: var(--fs-secondary);
	}

	.dot {
		flex: none;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--muted);
	}

	.dot.big {
		width: 10px;
		height: 10px;
	}

	.add {
		align-self: flex-start;
		padding: 6px 8px;
		color: var(--muted);
	}

	.add:hover {
		color: var(--ink);
	}

	.new {
		padding: 5px 8px;
	}

	.hint {
		margin: 0;
	}

	kbd {
		padding: 0 4px;
		border: 1px solid var(--soft);
		border-radius: 4px;
	}

	.top {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
	}

	.title {
		font-size: var(--fs-title);
	}

	.title-input {
		width: 220px;
		padding: 2px 8px;
		font-size: var(--fs-title);
	}

	.swatches {
		display: flex;
		gap: 6px;
	}

	.swatch {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		border: 2px solid transparent;
	}

	.swatch.on {
		border-color: var(--ink);
	}

	.ask {
		display: inline-flex;
		align-items: center;
		gap: 8px;
	}

	.tables {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	h3 {
		margin: 14px 0 2px;
		padding-top: 14px;
		border-top: 1px solid var(--line);
		color: var(--muted);
		font-size: var(--fs-secondary);
		font-weight: normal;
	}

	.where {
		display: flex;
		flex-wrap: wrap;
		gap: 18px;
		padding-top: 10px;
		border-top: 1px solid var(--line);
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	h2 {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: 600;
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 10px 12px;
		border-radius: var(--r-field);
		background: var(--sunken);
	}

	.card p {
		margin: 0;
	}

	.card-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.var {
		color: var(--ok);
		font-weight: 600;
	}

	.path {
		color: var(--muted);
		font-size: var(--fs-mono);
		overflow-wrap: anywhere;
	}

	.stale {
		color: var(--warn);
	}
</style>
