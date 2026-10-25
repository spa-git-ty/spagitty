<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { isMac } from '$lib/platform';
	import { METHODS, methodColour, methodOf, paramsOf, tabCounts, withParams, type Found } from './model';
	import KeepRules from './KeepRules.svelte';
	import PairsTable from './PairsTable.svelte';
	import type { Auth, AuthType, Body, BodyType, Call, KeepRule, Pair } from './types';
	import VarField from './VarField.svelte';

	/**
	 * The request half of Calls (FEAT-160, proposal §5.1): method, URL and
	 * *Send*, then *Params*, *Headers*, *Body*, *Auth* and *Keep*. A tab
	 * shows a count when it holds something. Every edit goes straight to the
	 * store, which writes the collection a moment later.
	 */

	interface Props {
		call: Call;
		onchange: (change: (call: Call) => Call) => void;
		find: (name: string) => Found | null;
		names: string[];
		sending: boolean;
		/** Why the call was not sent, under the URL. */
		refusal: string | null;
		onsend: () => void;
		oncancel: () => void;
		url?: HTMLInputElement | HTMLTextAreaElement | null;
	}

	let { call, onchange, find, names, sending, refusal, onsend, oncancel, url = $bindable(null) }: Props = $props();

	type Tab = 'params' | 'headers' | 'body' | 'auth' | 'keep';
	const TABS: { id: Tab; label: string }[] = [
		{ id: 'params', label: 'Params' },
		{ id: 'headers', label: 'Headers' },
		{ id: 'body', label: 'Body' },
		{ id: 'auth', label: 'Auth' },
		{ id: 'keep', label: 'Keep' }
	];
	const BODY_TYPES: { id: BodyType; label: string }[] = [
		{ id: 'json', label: 'JSON' },
		{ id: 'xml', label: 'XML' },
		{ id: 'form', label: 'Form' },
		{ id: 'multipart', label: 'Multipart' },
		{ id: 'raw', label: 'Raw' },
		{ id: 'none', label: 'None' }
	];
	const AUTH_TYPES: { id: AuthType; label: string }[] = [
		{ id: 'none', label: 'None' },
		{ id: 'bearer', label: 'Bearer token' },
		{ id: 'basic', label: 'Basic' },
		{ id: 'apiKey', label: 'API key' }
	];

	const PLACES = ['header', 'query'] as const;

	let tab = $state<Tab>('params');
	let seen = '';
	let menu = $state<{ x: number; y: number; anchor: HTMLElement; items: MenuItem[]; label: string } | null>(null);

	/** A call opened for the first time shows its body, when it has one. */
	$effect(() => {
		if (call.id === seen) return;
		seen = call.id;
		tab = call.body && call.body.type !== 'none' ? 'body' : 'params';
	});

	const counts = $derived(tabCounts(call));
	const method = $derived(methodOf(call));
	const body = $derived<Body>(call.body ?? { type: 'none' });
	const auth = $derived<Auth>(call.auth ?? { type: 'none' });
	const sendKeys = isMac() ? '⌘ ↵' : 'Ctrl ↵';

	function open(event: MouseEvent, label: string, items: MenuItem[]) {
		const anchor = event.currentTarget as HTMLElement;
		if (menu?.anchor === anchor) {
			menu = null;
			return;
		}
		const box = anchor.getBoundingClientRect();
		menu = { x: box.left, y: box.bottom + 4, anchor, items, label };
	}

	function pickMethod(event: MouseEvent) {
		open(
			event,
			'Method',
			METHODS.map((name) => ({
				id: name,
				label: name,
				note: name === method ? '✓' : undefined,
				run: () => onchange((current) => ({ ...current, method: name }))
			}))
		);
	}

	function setBody(type: BodyType) {
		onchange((current) => {
			const was = current.body ?? { type: 'none' };
			const next: Body = { ...was, type };
			if ((type === 'form' || type === 'multipart') && !next.fields) next.fields = [];
			if ((type === 'json' || type === 'xml' || type === 'raw') && next.text === undefined) next.text = '';
			return { ...current, body: next };
		});
	}

	function pickBody(event: MouseEvent) {
		open(
			event,
			'Body type',
			BODY_TYPES.map((choice) => ({
				id: choice.id,
				label: choice.label,
				note: choice.id === body.type ? '✓' : undefined,
				run: () => setBody(choice.id)
			}))
		);
	}

	function setAuth(change: Partial<Auth>) {
		onchange((current) => ({ ...current, auth: { ...(current.auth ?? { type: 'none' }), ...change } }));
	}

	const fieldPairs = $derived<Pair[]>(
		(body.fields ?? []).map((field) => (field.on === false ? [field.name, field.value, false] : [field.name, field.value]))
	);

	function setFields(pairs: Pair[], files = (body.fields ?? []).map((field) => field.file === true)) {
		onchange((current) => ({
			...current,
			body: {
				...(current.body ?? { type: 'form' }),
				fields: pairs.map((pair, index) => ({
					name: pair[0],
					value: pair[1],
					...(files[index] ? { file: true } : {}),
					...(pair[2] === false ? { on: false } : {})
				}))
			}
		}));
	}

	function setKeep(rules: KeepRule[]) {
		onchange((current) => ({ ...current, keep: rules }));
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey && !(event.ctrlKey || event.metaKey)) {
			event.preventDefault();
			onsend();
		}
	}
</script>

<section class="request" aria-label="The request">
	<div class="line">
		<button
			type="button"
			class="method mono"
			style:color={methodColour(method)}
			title="The method"
			disabled={call.kind === 'soap'}
			onclick={pickMethod}>{method} <span class="caret">▾</span></button
		>
		<VarField
			label="URL"
			placeholder={'{{baseUrl}}/path'}
			value={call.url}
			onchange={(text) => onchange((current) => ({ ...current, url: text }))}
			{find}
			{names}
			onkeydown={keydown}
			bind:input={url}
		/>
		{#if sending}
			<button type="button" class="send cancel" title="Stop waiting for the answer" onclick={oncancel}>Cancel</button>
		{:else}
			<button type="button" class="send" title={`Send (${sendKeys})`} onclick={onsend}
				>Send <span class="keys">{sendKeys}</span></button
			>
		{/if}
	</div>
	{#if refusal}
		<p class="refusal" role="alert">{refusal}</p>
	{/if}

	<div class="tabs" role="tablist" aria-label="Parts of the request">
		{#each TABS as item (item.id)}
			<button
				type="button"
				role="tab"
				class="tab"
				class:on={tab === item.id}
				aria-selected={tab === item.id}
				onclick={() => (tab = item.id)}
				>{item.label}{#if counts[item.id] > 0 && item.id !== 'body' && item.id !== 'auth'}<span class="count"
						>{counts[item.id]}</span
					>{:else if counts[item.id] > 0}<span class="dot" aria-label="set"></span>{/if}</button
			>
		{/each}
		<span class="gap"></span>
		{#if tab === 'body'}
			<button type="button" class="picker" title="What the body is" onclick={pickBody}
				>{BODY_TYPES.find((choice) => choice.id === body.type)?.label} ▾</button
			>
		{/if}
	</div>

	<div class="pane" role="tabpanel">
		{#if tab === 'params'}
			<PairsTable
				noun="parameter"
				noSwitch
				pairs={paramsOf(call.url)}
				onchange={(pairs) => onchange((current) => ({ ...current, url: withParams(current.url, pairs) }))}
				{find}
				{names}
			/>
		{:else if tab === 'headers'}
			<PairsTable
				noun="header"
				pairs={call.headers ?? []}
				onchange={(pairs) => onchange((current) => ({ ...current, headers: pairs }))}
				{find}
				{names}
			/>
			{#if body.type === 'json' || body.type === 'xml' || body.type === 'form'}
				<p class="hint">Content-Type follows the body unless a header here sets it.</p>
			{/if}
		{:else if tab === 'body'}
			{#if body.type === 'none'}
				<p class="hint">This call sends no body. Choose a type on the right to give it one.</p>
			{:else if body.type === 'form' || body.type === 'multipart'}
				<PairsTable
					noun="field"
					pairs={fieldPairs}
					onchange={(pairs) => setFields(pairs)}
					files={body.type === 'multipart' ? (body.fields ?? []).map((field) => field.file === true) : undefined}
					onfiles={body.type === 'multipart' ? (files) => setFields(fieldPairs, files) : undefined}
					{find}
					{names}
				/>
			{:else}
				{#if body.type === 'raw'}
					<label class="inline">
						<span>Content-Type</span>
						<input
							class="mono"
							type="text"
							spellcheck="false"
							placeholder="text/plain"
							value={body.contentType ?? ''}
							oninput={(event) =>
								onchange((current) => ({
									...current,
									body: { ...(current.body ?? { type: 'raw' }), contentType: event.currentTarget.value || undefined }
								}))}
						/>
					</label>
				{/if}
				<VarField
					multiline
					rows={8}
					label="Body"
					placeholder={body.type === 'json' ? '{\n  "name": "{{value}}"\n}' : ''}
					value={body.text ?? ''}
					onchange={(text) =>
						onchange((current) => ({ ...current, body: { ...(current.body ?? { type: 'raw' }), text } }))}
					{find}
					{names}
				/>
			{/if}
		{:else if tab === 'auth'}
			<div class="choices" role="radiogroup" aria-label="Authorization">
				{#each AUTH_TYPES as choice (choice.id)}
					<button
						type="button"
						role="radio"
						aria-checked={auth.type === choice.id}
						class="choice"
						class:on={auth.type === choice.id}
						onclick={() => setAuth({ type: choice.id })}>{choice.label}</button
					>
				{/each}
			</div>
			{#if auth.type === 'none'}
				<p class="hint">No Authorization is added. A header set by hand is still sent.</p>
			{:else if auth.type === 'bearer'}
				<label class="inline">
					<span>Token</span>
					<VarField
						label="Token"
						placeholder={'{{token}}'}
						value={auth.token ?? ''}
						onchange={(text) => setAuth({ token: text })}
						{find}
						{names}
					/>
				</label>
			{:else if auth.type === 'basic'}
				<label class="inline">
					<span>User</span>
					<VarField label="User" value={auth.user ?? ''} onchange={(text) => setAuth({ user: text })} {find} {names} />
				</label>
				<label class="inline">
					<span>Password</span>
					<VarField
						label="Password"
						placeholder={'{{password}}'}
						value={auth.password ?? ''}
						onchange={(text) => setAuth({ password: text })}
						{find}
						{names}
					/>
				</label>
			{:else}
				<label class="inline">
					<span>Name</span>
					<VarField
						label="Key name"
						placeholder="X-Api-Key"
						value={auth.key ?? ''}
						onchange={(text) => setAuth({ key: text })}
						{find}
						{names}
					/>
				</label>
				<label class="inline">
					<span>Value</span>
					<VarField
						label="Key value"
						placeholder={'{{apiKey}}'}
						value={auth.value ?? ''}
						onchange={(text) => setAuth({ value: text })}
						{find}
						{names}
					/>
				</label>
				<div class="choices" role="radiogroup" aria-label="Sent in">
					<span class="hint">Sent in</span>
					{#each PLACES as place (place)}
						<button
							type="button"
							role="radio"
							aria-checked={(auth.in ?? 'header') === place}
							class="choice"
							class:on={(auth.in ?? 'header') === place}
							onclick={() => setAuth({ in: place })}>{place === 'header' ? 'a header' : 'the query'}</button
						>
					{/each}
				</div>
			{/if}
			{#if auth.type !== 'none'}
				<p class="hint">Write a secret as a variable, such as <code>{'{{password}}'}</code>, so it stays out of the file.</p>
			{/if}
		{:else}
			<KeepRules rules={call.keep ?? []} onchange={setKeep} />
		{/if}
	</div>
</section>

{#if menu}
	<Menu x={menu.x} y={menu.y} anchor={menu.anchor} label={menu.label} items={menu.items} onclose={() => (menu = null)} />
{/if}

<style>
	.request {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-height: 0;
	}

	.line {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.method {
		flex: none;
		min-width: 86px;
		padding: 6px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		background: var(--sunken);
		font-size: var(--fs-mono);
		font-weight: 600;
		text-align: left;
	}

	.method:hover:not(:disabled) {
		background: var(--hover);
	}

	.caret {
		color: var(--muted);
		font-weight: normal;
	}

	.line :global(input) {
		background: var(--sunken);
	}

	.send {
		flex: none;
		padding: 6px 14px;
		border-radius: var(--r-button);
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
		white-space: nowrap;
	}

	.send:hover {
		background: var(--accent-lift);
	}

	.send.cancel {
		background: var(--surface-2);
		color: var(--ink);
		border: 1px solid var(--soft);
	}

	.keys {
		margin-left: 4px;
		font-size: var(--fs-secondary);
		font-weight: normal;
		opacity: 0.75;
	}

	.refusal {
		margin: 0;
		color: var(--danger);
		font-size: var(--fs-secondary);
	}

	.tabs {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.tab {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 4px 10px;
		border-radius: var(--r-pill);
		color: var(--muted);
	}

	.tab:hover {
		color: var(--ink);
	}

	.tab.on {
		background: var(--surface-2);
		color: var(--ink);
	}

	.count {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.dot {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--ok);
	}

	.gap {
		flex: 1;
	}

	.picker {
		padding: 3px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
	}

	.picker:hover {
		background: var(--hover);
	}

	.pane {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-height: 0;
		overflow: auto;
		padding: 10px;
		background: var(--sunken);
		border-radius: var(--r-panel);
	}

	.pane :global(textarea) {
		min-height: 120px;
	}

	.hint {
		margin: 0;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.inline {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.inline > span {
		flex: none;
		width: 92px;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.inline > input {
		flex: 1;
		padding: 6px 10px;
		font-size: var(--fs-mono);
	}

	.choices {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
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

	code {
		font-family: var(--font-mono);
	}
</style>
