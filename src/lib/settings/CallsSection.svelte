<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import * as appApi from '$lib/api';
	import * as callsApi from '$lib/calls/api';
	import type { CallsSettings, ClientCertFor } from '$lib/calls/types';
	import { repo } from '$lib/repo.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { settings } from './store.svelte';

	/**
	 * Settings › Calls (FEAT-165, proposal §9): every setting Calls reads, in
	 * six groups. A repository can override any of them in its calls folder's
	 * `settings.json`; a setting the open repository overrides says so.
	 *
	 * As everywhere in Settings, the label is the setting and the detail is
	 * the hover (TASK-038). Frameworks that have no reader yet are not shown
	 * at all (§9), and *Leave secrets out when sharing* is not a setting: it
	 * is drawn as *Always*, locked.
	 */

	type Key = keyof CallsSettings;

	const LOOK_FOR: { id: string; label: string }[] = [
		{ id: 'postman', label: 'Postman' },
		{ id: 'insomnia', label: 'Insomnia' },
		{ id: 'bruno', label: 'Bruno' },
		{ id: 'openapi', label: 'OpenAPI' },
		{ id: 'http', label: '.http' },
		{ id: 'wsdl', label: 'WSDL' },
		{ id: 'har', label: 'HAR' }
	];

	/** Only the frameworks a reader exists for (§9). */
	const FRAMEWORKS: { id: string; label: string }[] = [
		{ id: 'springMvc', label: 'Spring MVC' },
		{ id: 'springWebflux', label: 'Spring WebFlux' },
		{ id: 'jaxRs', label: 'JAX-RS' },
		{ id: 'jaxWs', label: 'JAX-WS · SOAP' },
		{ id: 'springWs', label: 'Spring-WS · SOAP' }
	];

	const calls = $derived(settings.settings.calls ?? null);
	let overridden = $state<string[]>([]);
	let error = $state<string | null>(null);

	/** What the open repository overrides: read on arrival and when it changes. */
	$effect(() => {
		void repo.info?.path;
		void repo.generation;
		if (!appApi.inTauri()) return;
		callsApi
			.overridden()
			.then((keys) => (overridden = keys))
			.catch(() => (overridden = []));
	});

	async function set<K extends Key>(key: K, value: CallsSettings[K]) {
		if (!calls) return;
		error = null;
		await settings.write({ ...settings.settings, calls: { ...calls, [key]: value } });
	}

	function toggleIn(key: 'lookFor' | 'frameworks', id: string) {
		if (!calls) return;
		const list = calls[key];
		void set(key, list.includes(id) ? list.filter((other) => other !== id) : [...list, id]);
	}

	async function reset() {
		try {
			const defaults = await callsApi.defaultSettings();
			await settings.write({ ...settings.settings, calls: defaults });
		} catch (cause) {
			error = `Could not reset: ${String(cause)}`;
		}
	}

	/** A folder in the repository, written as the settings store it. */
	function saveFolder(text: string) {
		const folder = text.trim().replace(/\\/g, '/').replace(/^\/+|\/+$/g, '');
		if (!folder || folder.split('/').includes('..')) {
			error = 'The folder must be inside the repository.';
			return;
		}
		if (folder !== calls?.folder) void set('folder', folder);
	}

	function saveNumber(key: 'timeoutSeconds' | 'history', text: string, min: number, max: number) {
		const value = Math.round(Number(text));
		if (!Number.isFinite(value) || value < min || value > max) {
			error = `Between ${min} and ${max}.`;
			return;
		}
		if (calls && value !== calls[key]) void set(key, value);
	}

	let adding = $state<ClientCertFor | null>(null);

	async function pick(which: 'cert' | 'key') {
		const path = await openDialog({ multiple: false, directory: false }).catch(() => null);
		if (adding && typeof path === 'string') adding = { ...adding, [which]: path };
	}

	function addCert() {
		if (!calls || !adding) return;
		const host = adding.host.trim().toLowerCase();
		if (!host || !adding.cert.trim() || !adding.key.trim()) {
			error = 'A client certificate needs a host, a certificate file and a key file.';
			return;
		}
		void set('clientCerts', [...calls.clientCerts.filter((cert) => cert.host !== host), { ...adding, host }]);
		adding = null;
	}

	const isOver = (key: Key) => overridden.includes(key);
</script>

{#snippet over(key: Key)}
	{#if isOver(key)}<span class="over" title="This repository's settings.json sets it">Overridden in this repository</span>{/if}
{/snippet}

{#snippet toggle(key: Key, label: string, what: string)}
	<label class="row" title={what}>
		<span class="label">{label}{@render over(key)}</span>
		<input
			type="checkbox"
			role="switch"
			aria-label={label}
			checked={calls?.[key] === true}
			onchange={(event) => set(key, event.currentTarget.checked as never)}
		/>
	</label>
{/snippet}

{#snippet choice(key: Key, label: string, what: string, options: { id: string; label: string; disabled?: string }[])}
	<div class="row" title={what}>
		<span class="label">{label}{@render over(key)}</span>
		<div class="segments" role="radiogroup" aria-label={label}>
			{#each options as option (option.id)}
				<button
					type="button"
					role="radio"
					class="segment"
					class:on={calls?.[key] === option.id}
					aria-checked={calls?.[key] === option.id}
					disabled={option.disabled !== undefined}
					title={option.disabled}
					onclick={() => set(key, option.id as never)}>{option.label}</button
				>
			{/each}
		</div>
	</div>
{/snippet}

<section class="section" aria-label="Calls">
	<header class="top">
		<h2 class="heading">Calls</h2>
		<span class="note"
			>For every repository · a repository can override any of these in <span class="mono"
				>{calls?.folder ?? '.spagitty/calls'}/settings.json</span
			></span
		>
		<span class="gap"></span>
		<button type="button" class="act" onclick={reset} title="Every setting in this section as a new install has it"
			>Reset</button
		>
	</header>
	{#if error}<p class="error" role="alert">{error}</p>{/if}

	{#if !calls}
		<p class="note">Reading the settings…</p>
	{:else}
		<div class="groups">
			<div class="group">
				<h3>Where calls live</h3>
				<div class="row" title="Collections and shared environments, as plain JSON you can review.">
					<span class="label">Folder in the repository{@render over('folder')}</span>
					<input
						class="mono text"
						type="text"
						aria-label="Folder in the repository"
						spellcheck="false"
						value={calls.folder}
						onchange={(event) => saveFolder(event.currentTarget.value)}
					/>
				</div>
				{@render toggle(
					'openFound',
					'Open collection files found in the repository',
					'Shown under Found in this repository. Never moved or rewritten.'
				)}
				<div class="row wrap" title="The kinds of collection file Import looks for in the repository.">
					<span class="label">Look for{@render over('lookFor')}</span>
					<div class="chips">
						{#each LOOK_FOR as kind (kind.id)}
							<button
								type="button"
								class="chip"
								class:on={calls.lookFor.includes(kind.id)}
								aria-pressed={calls.lookFor.includes(kind.id)}
								onclick={() => toggleIn('lookFor', kind.id)}>{kind.label}</button
							>
						{/each}
					</div>
				</div>
			</div>

			<div class="group">
				<h3>Sending</h3>
				<div class="row" title="How long to wait for an answer.">
					<span class="label">Time out after{@render over('timeoutSeconds')}</span>
					<span class="unit"
						><input
							class="mono number"
							type="number"
							min="1"
							max="3600"
							aria-label="Time out after, in seconds"
							value={calls.timeoutSeconds}
							onchange={(event) => saveNumber('timeoutSeconds', event.currentTarget.value, 1, 3600)}
						/> s</span
					>
				</div>
				{@render toggle('followRedirects', 'Follow redirects', 'Up to ten, as a browser would.')}
				{@render toggle(
					'checkTls',
					'Check TLS certificates',
					'Turn it off per environment, not here, for a self-signed local server.'
				)}
				<div class="row" title="System uses HTTPS_PROXY, HTTP_PROXY, ALL_PROXY and NO_PROXY.">
					<span class="label">Proxy{@render over('proxy')}</span>
					<div class="segments" role="radiogroup" aria-label="Proxy">
						{#each [{ id: 'system', label: 'System' }, { id: 'none', label: 'None' }, { id: 'custom', label: 'Custom' }] as option (option.id)}
							<button
								type="button"
								role="radio"
								class="segment"
								class:on={calls.proxy.kind === option.id}
								aria-checked={calls.proxy.kind === option.id}
								onclick={() =>
									set(
										'proxy',
										option.id === 'custom'
											? { kind: 'custom', url: calls.proxy.kind === 'custom' ? calls.proxy.url : 'http://' }
											: { kind: option.id as 'system' | 'none' }
									)}>{option.label}</button
							>
						{/each}
					</div>
				</div>
				{#if calls.proxy.kind === 'custom'}
					<div class="row sub">
						<span class="label">Proxy URL</span>
						<input
							class="mono text"
							type="text"
							aria-label="Proxy URL"
							spellcheck="false"
							value={calls.proxy.url}
							onchange={(event) => set('proxy', { kind: 'custom', url: event.currentTarget.value.trim() })}
						/>
					</div>
				{/if}
				<div class="row wrap" title="For mutual TLS: a certificate and key, as PEM files on this computer, for a host that asks.">
					<span class="label">Client certificates{@render over('clientCerts')}</span>
					{#if !adding}
						<button type="button" class="act" onclick={() => (adding = { host: '', cert: '', key: '' })}>Add…</button>
					{/if}
				</div>
				{#each calls.clientCerts as cert (cert.host)}
					<div class="row sub cert">
						<span class="mono">{cert.host}</span>
						<span class="mono path" title={`${cert.cert}\n${cert.key}`}>{cert.cert}</span>
						<button
							type="button"
							class="act"
							aria-label={`Remove the certificate for ${cert.host}`}
							onclick={() =>
								set(
									'clientCerts',
									calls.clientCerts.filter((other) => other.host !== cert.host)
								)}>Remove</button
						>
					</div>
				{/each}
				{#if adding}
					<div class="adding">
						<input
							class="mono"
							type="text"
							placeholder="host, such as api.bank.local"
							aria-label="Host"
							spellcheck="false"
							bind:value={adding.host}
						/>
						<span class="pick">
							<input class="mono" type="text" placeholder="certificate (.pem)" aria-label="Certificate file" bind:value={adding.cert} />
							<button type="button" class="act" onclick={() => pick('cert')}>Browse</button>
						</span>
						<span class="pick">
							<input class="mono" type="text" placeholder="key (.pem)" aria-label="Key file" bind:value={adding.key} />
							<button type="button" class="act" onclick={() => pick('key')}>Browse</button>
						</span>
						<span class="pick">
							<span class="gap"></span>
							<button type="button" class="act" onclick={() => (adding = null)}>Cancel</button>
							<button type="button" class="act primary" onclick={addCert}>Add</button>
						</span>
					</div>
				{/if}
				{@render toggle(
					'beautify',
					'Beautify responses',
					'JSON and XML that arrive minified are shown formatted. The Beautify button still switches one back.'
				)}
				<div class="row" title="Sent with every call that does not set the same header itself. One per line.">
					<span class="label">Header on every call{@render over('everyHeader')}</span>
					<input
						class="mono text"
						type="text"
						aria-label="Header on every call"
						spellcheck="false"
						placeholder="User-Agent: Spagitty/<version>"
						value={calls.everyHeader ?? ''}
						onchange={(event) => set('everyHeader', event.currentTarget.value.trim() || null)}
					/>
				</div>
			</div>

			<div class="group">
				<h3>Scanning code</h3>
				{@render choice('scanOnOpen', 'Scan when a repository opens', 'Reads source files only. Nothing is built or run.', [
					{ id: 'ask', label: 'Ask' },
					{ id: 'always', label: 'Always' },
					{ id: 'never', label: 'Never' }
				])}
				<div class="row wrap" title="REST and SOAP endpoints these declare become calls.">
					<span class="label">Frameworks{@render over('frameworks')}</span>
					<div class="chips">
						{#each FRAMEWORKS as framework (framework.id)}
							<button
								type="button"
								class="chip"
								class:on={calls.frameworks.includes(framework.id)}
								aria-pressed={calls.frameworks.includes(framework.id)}
								onclick={() => toggleIn('frameworks', framework.id)}>{framework.label}</button
							>
						{/each}
					</div>
				</div>
				<div class="row" title="Globs, separated by commas. Files under them are not read.">
					<span class="label">Skip paths{@render over('skipPaths')}</span>
					<input
						class="mono text"
						type="text"
						aria-label="Skip paths"
						spellcheck="false"
						value={calls.skipPaths}
						onchange={(event) => set('skipPaths', event.currentTarget.value)}
					/>
				</div>
			</div>

			<div class="group">
				<h3>SOAP</h3>
				{@render choice('soapVersion', 'Default version', 'A WSDL that names one wins.', [
					{ id: '1.1', label: '1.1' },
					{ id: '1.2', label: '1.2' }
				])}
				{@render toggle(
					'envelopeFromWsdl',
					'Build the envelope from the WSDL',
					'Fills every element with a sample value and a comment with its type.'
				)}
				{@render choice('wsSecurity', 'WS-Security', 'What a new SOAP call starts with.', [
					{ id: 'none', label: 'None' },
					{ id: 'username', label: 'Username token' },
					{ id: 'signed', label: 'Signed', disabled: 'Signing comes later (FEAT-167)' }
				])}
				{@render toggle('faultAsError', 'Show a SOAP fault as an error', 'A fault is drawn in --danger with its code and reason.')}
			</div>

			<div class="group">
				<h3>Kept values</h3>
				{@render toggle('clickToKeep', 'Click a response value to keep it', 'JSONPath for JSON, XPath for XML and SOAP.')}
				<div class="row" title="Whether a value kept from now on is read again after every call, or kept once.">
					<span class="label">A new kept value updates{@render over('keepEvery')}</span>
					<div class="segments" role="radiogroup" aria-label="A new kept value updates">
						<button
							type="button"
							role="radio"
							class="segment"
							class:on={calls.keepEvery}
							aria-checked={calls.keepEvery}
							onclick={() => set('keepEvery', true)}>On every call</button
						>
						<button
							type="button"
							role="radio"
							class="segment"
							class:on={!calls.keepEvery}
							aria-checked={!calls.keepEvery}
							onclick={() => set('keepEvery', false)}>Once</button
						>
					</div>
				</div>
				{@render toggle('suggestNames', 'Suggest names', 'accessToken is offered as token, user.id as userId.')}
			</div>

			<div class="group">
				<h3>Safety and sharing</h3>
				<div class="row" title="Never a file in the repository, a share or history.">
					<span class="label">Secrets and kept values go in</span>
					<span class="fixed">System keychain</span>
				</div>
				<div class="row" title="Sends kept per repository, on this computer. The oldest go first.">
					<span class="label">Keep history{@render over('history')}</span>
					<span class="unit"
						><input
							class="mono number"
							type="number"
							min="0"
							max="100000"
							aria-label="Keep history, in calls"
							value={calls.history}
							onchange={(event) => saveNumber('history', event.currentTarget.value, 0, 100000)}
						/> calls</span
					>
				</div>
				{@render choice('shareAs', 'Share as', 'What Share starts with.', [
					{ id: 'file', label: 'One file' },
					{ id: 'zip', label: 'Zip' },
					{ id: 'commit', label: 'Commit' }
				])}
				<div class="row" title="Not a setting: a share never holds a secret or a kept value.">
					<span class="label">Leave secrets out when sharing</span>
					<span class="fixed"><Icon name="lock" size={12} /> Always</span>
				</div>
			</div>
		</div>
	{/if}
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.top {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 10px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.note,
	.over,
	.unit,
	.fixed {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.over {
		margin-left: 8px;
		color: var(--warn);
	}

	.gap {
		flex: 1;
	}

	.error {
		margin: 0;
		color: var(--danger);
	}

	.groups {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
		gap: 12px;
		align-items: start;
	}

	.group {
		display: flex;
		flex-direction: column;
		padding: 12px 16px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	h3 {
		margin: 0 0 4px;
		color: var(--muted);
		font-size: var(--fs-secondary);
		font-weight: normal;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 40px;
		border-bottom: 1px solid var(--line);
	}

	.row:last-child {
		border-bottom: 0;
	}

	.row.wrap {
		flex-wrap: wrap;
		padding: 6px 0;
	}

	.row.sub {
		padding-left: 14px;
	}

	.label {
		flex: 1;
		min-width: 160px;
	}

	.text {
		width: 240px;
		padding: 4px 8px;
		font-size: var(--fs-mono);
	}

	.number {
		width: 80px;
		padding: 4px 8px;
		font-size: var(--fs-mono);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.chip {
		padding: 2px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
	}

	.chip.on {
		border-color: color-mix(in srgb, var(--accent) 60%, transparent);
		background: color-mix(in srgb, var(--accent) 14%, var(--surface));
		color: var(--accent);
	}

	.segments {
		display: inline-flex;
		padding: 2px;
		border-radius: var(--r-pill);
		background: var(--sunken);
	}

	.segment {
		padding: 3px 12px;
		border-radius: var(--r-pill);
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.segment.on {
		background: var(--surface-2);
		color: var(--ink);
	}

	.segment:disabled {
		opacity: 0.5;
	}

	.act {
		padding: 3px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
		font-size: var(--fs-secondary);
	}

	.act:hover {
		background: var(--hover);
	}

	.act.primary {
		border-color: transparent;
		background: var(--accent);
		color: var(--on-accent);
	}

	.cert .path {
		flex: 1;
		overflow: hidden;
		color: var(--muted);
		font-size: var(--fs-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.adding {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px 0 8px 14px;
	}

	.adding input {
		flex: 1;
		padding: 4px 8px;
		font-size: var(--fs-mono);
	}

	.pick {
		display: flex;
		gap: 6px;
	}

</style>
