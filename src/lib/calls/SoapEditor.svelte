<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { notice } from '$lib/ui/notice.svelte';
	import { isMac } from '$lib/platform';
	import * as api from './api';
	import KeepRules from './KeepRules.svelte';
	import type { Found } from './model';
	import PairsTable from './PairsTable.svelte';
	import type { Call, KeepRule, WsSecurity, WsdlView } from './types';
	import VarField from './VarField.svelte';

	/**
	 * A SOAP call (FEAT-163, proposal §7.1): the version, the operation
	 * picker (*Service › Operation*) from the collection's WSDL, the endpoint
	 * URL and *Send*; then *Envelope*, *Headers*, *WS-Security* and *Keep*.
	 *
	 * The header the version needs — `SOAPAction` for 1.1, the `action`
	 * parameter of `Content-Type` for 1.2 — is shown, not hidden. *Rebuild
	 * from WSDL* makes the envelope again and keeps what was typed where its
	 * element still exists.
	 */

	interface Props {
		call: Call;
		/** The collection's WSDL, from the working tree, when it has one. */
		wsdl: string | null;
		onchange: (change: (call: Call) => Call) => void;
		find: (name: string) => Found | null;
		names: string[];
		sending: boolean;
		refusal: string | null;
		onsend: () => void;
		oncancel: () => void;
		url?: HTMLInputElement | HTMLTextAreaElement | null;
	}

	let { call, wsdl, onchange, find, names, sending, refusal, onsend, oncancel, url = $bindable(null) }: Props = $props();

	type Tab = 'envelope' | 'headers' | 'security' | 'keep';
	const TABS: { id: Tab; label: string }[] = [
		{ id: 'envelope', label: 'Envelope' },
		{ id: 'headers', label: 'Headers' },
		{ id: 'security', label: 'WS-Security' },
		{ id: 'keep', label: 'Keep' }
	];
	const MODES: { id: 'none' | 'username'; label: string }[] = [
		{ id: 'none', label: 'None' },
		{ id: 'username', label: 'Username token' }
	];
	let tab = $state<Tab>('envelope');
	let menu = $state<{ x: number; y: number; anchor: HTMLElement; items: MenuItem[]; label: string } | null>(null);
	let view = $state<WsdlView | null>(null);
	let rebuilding = $state(false);

	const version = $derived(call.version === '1.2' ? '1.2' : '1.1');
	const ws = $derived<WsSecurity>(call.wsSecurity ?? { mode: 'none' });
	const action = $derived(call.action ?? '');
	const shownHeader = $derived(
		version === '1.2'
			? `Content-Type: application/soap+xml; charset=utf-8${action ? `; action="${action}"` : ''}`
			: `SOAPAction: "${action}"`
	);
	const sendKeys = isMac() ? '⌘ ↵' : 'Ctrl ↵';
	const counts = $derived({
		envelope: 0,
		headers: (call.headers ?? []).filter((pair) => pair[2] !== false).length,
		security: ws.mode === 'none' ? 0 : 1,
		keep: (call.keep ?? []).length
	});

	function open(event: MouseEvent, label: string, items: MenuItem[]) {
		const anchor = event.currentTarget as HTMLElement;
		if (menu?.anchor === anchor) {
			menu = null;
			return;
		}
		const box = anchor.getBoundingClientRect();
		menu = { x: box.left, y: box.bottom + 4, anchor, items, label };
	}

	function pickVersion(event: MouseEvent) {
		open(
			event,
			'SOAP version',
			(['1.1', '1.2'] as const).map((choice) => ({
				id: choice,
				label: `SOAP ${choice}`,
				note: choice === version ? '✓' : undefined,
				run: () => rebuild({ version: choice })
			}))
		);
	}

	async function pickOperation(event: MouseEvent) {
		const anchor = event.currentTarget as HTMLElement;
		if (!wsdl) return;
		if (!view) {
			try {
				view = await api.wsdl(wsdl);
			} catch (cause) {
				notice.failed(`Could not read ${wsdl}`, cause);
				return;
			}
		}
		const items: MenuItem[] = [];
		for (const service of view.services) {
			for (const port of service.ports) {
				items.push({ heading: `${service.name} · ${port.name} · SOAP ${port.version}` });
				for (const operation of port.operations) {
					items.push({
						id: `${service.name}/${port.name}/${operation.name}`,
						label: operation.name,
						note: call.service === service.name && call.operation === operation.name ? '✓' : undefined,
						run: () =>
							rebuild({
								service: service.name,
								port: port.name,
								operation: operation.name,
								version: port.version
							})
					});
				}
			}
		}
		open({ currentTarget: anchor } as unknown as MouseEvent, 'Operation', items);
	}

	/**
	 * The envelope made again from the WSDL, for this operation or another,
	 * keeping what was typed. With no WSDL, a version change only changes
	 * the version.
	 */
	async function rebuild(change: { service?: string; port?: string; operation?: string; version?: '1.1' | '1.2' } = {}) {
		const next = { service: call.service, port: call.port, operation: call.operation, version, ...change };
		if (!wsdl || !next.service || !next.operation) {
			onchange((current) => ({ ...current, version: next.version }));
			return;
		}
		rebuilding = true;
		try {
			const [envelope, foundAction] = await api.rebuild({
				wsdl,
				service: next.service,
				port: next.port ?? null,
				operation: next.operation,
				version: next.version,
				envelope: next.operation === call.operation ? (call.envelope ?? null) : null
			});
			onchange((current) => ({
				...current,
				service: next.service,
				port: next.port,
				operation: next.operation,
				version: next.version,
				action: foundAction || undefined,
				envelope,
				name: current.name === current.operation ? (next.operation ?? current.name) : current.name
			}));
		} catch (cause) {
			notice.failed('Could not rebuild the envelope', cause);
		} finally {
			rebuilding = false;
		}
	}

	function setWs(change: Partial<WsSecurity>) {
		onchange((current) => ({ ...current, wsSecurity: { ...(current.wsSecurity ?? { mode: 'none' }), ...change } }));
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey && !(event.ctrlKey || event.metaKey)) {
			event.preventDefault();
			onsend();
		}
	}
</script>

<section class="request" aria-label="The SOAP request">
	<div class="line">
		<button type="button" class="version mono" title="The SOAP version" onclick={pickVersion}
			>SOAP {version} <span class="caret">▾</span></button
		>
		{#if wsdl}
			<button type="button" class="operation" title={`From ${wsdl}`} onclick={pickOperation}
				>{call.service ?? '?'} › {call.operation ?? 'operation'} <span class="caret">▾</span></button
			>
		{/if}
		<VarField
			label="Endpoint URL"
			placeholder={'{{baseUrl}}/ws/service'}
			value={call.url}
			onchange={(text) => onchange((current) => ({ ...current, url: text }))}
			{find}
			{names}
			onkeydown={keydown}
			bind:input={url}
		/>
		{#if sending}
			<button type="button" class="send cancel" onclick={oncancel}>Cancel</button>
		{:else}
			<button type="button" class="send" title={`Send (${sendKeys})`} onclick={onsend}>Send <span class="keys">{sendKeys}</span></button>
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
				>{item.label}{#if counts[item.id] > 0 && item.id !== 'security'}<span class="count">{counts[item.id]}</span
					>{:else if counts[item.id] > 0}<span class="dot" aria-label="set"></span>{/if}</button
			>
		{/each}
		<span class="gap"></span>
		{#if tab === 'envelope' && wsdl && call.operation}
			<button
				type="button"
				class="picker"
				disabled={rebuilding}
				title={`Make the envelope again from ${wsdl}, keeping what you typed where its element still exists`}
				onclick={() => rebuild()}>Rebuild from WSDL</button
			>
		{/if}
	</div>

	<div class="pane" role="tabpanel">
		{#if tab === 'envelope'}
			<VarField
				multiline
				rows={14}
				label="Envelope"
				value={call.envelope ?? ''}
				onchange={(text) => onchange((current) => ({ ...current, envelope: text }))}
				{find}
				{names}
			/>
		{:else if tab === 'headers'}
			<p class="shown mono" title="Sent with every call of this version, from the WSDL">{shownHeader}</p>
			<label class="inline">
				<span>Action</span>
				<VarField
					label="SOAP action"
					placeholder="urn:service:Operation"
					value={action}
					onchange={(text) => onchange((current) => ({ ...current, action: text || undefined }))}
					{find}
					{names}
				/>
			</label>
			<PairsTable
				noun="header"
				pairs={call.headers ?? []}
				onchange={(pairs) => onchange((current) => ({ ...current, headers: pairs }))}
				{find}
				{names}
			/>
		{:else if tab === 'security'}
			<div class="choices" role="radiogroup" aria-label="WS-Security">
				{#each MODES as choice (choice.id)}
					<button
						type="button"
						role="radio"
						class="choice"
						class:on={ws.mode === choice.id}
						aria-checked={ws.mode === choice.id}
						onclick={() => setWs({ mode: choice.id })}>{choice.label}</button
					>
				{/each}
				<button type="button" role="radio" class="choice" aria-checked="false" disabled title="An X.509 signature comes later (FEAT-167)"
					>Signed</button
				>
			</div>
			{#if ws.mode === 'username'}
				<label class="inline">
					<span>User</span>
					<VarField label="WS-Security user" placeholder={'{{user}}'} value={ws.user ?? ''} onchange={(text) => setWs({ user: text })} {find} {names} />
				</label>
				<label class="inline">
					<span>Password</span>
					<VarField
						label="WS-Security password"
						placeholder={'{{password}}'}
						value={ws.password ?? ''}
						onchange={(text) => setWs({ password: text })}
						{find}
						{names}
					/>
				</label>
				<div class="choices" role="radiogroup" aria-label="Password as">
					<span class="hint">Password as</span>
					<button type="button" role="radio" class="choice" class:on={!ws.digest} aria-checked={!ws.digest} onclick={() => setWs({ digest: false })}
						>Text</button
					>
					<button type="button" role="radio" class="choice" class:on={ws.digest === true} aria-checked={ws.digest === true} onclick={() => setWs({ digest: true })}
						>Digest</button
					>
				</div>
				<p class="hint">A nonce and the time it was sent are added to each call. Keep the password a variable so it stays out of the file.</p>
			{:else}
				<p class="hint">No WS-Security header is added.</p>
			{/if}
		{:else}
			<KeepRules rules={call.keep ?? []} start="//" onchange={(rules: KeepRule[]) => onchange((current) => ({ ...current, keep: rules }))} />
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

	.version,
	.operation {
		flex: none;
		padding: 6px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		background: var(--sunken);
		white-space: nowrap;
	}

	.version {
		color: var(--agent);
		font-size: var(--fs-mono);
		font-weight: 600;
	}

	.version:hover,
	.operation:hover {
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

	.picker:hover:not(:disabled) {
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
		min-height: 200px;
	}

	.shown {
		margin: 0;
		color: var(--muted);
		font-size: var(--fs-mono);
		overflow-wrap: anywhere;
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

	.choice:disabled {
		opacity: 0.5;
	}

	.hint {
		margin: 0;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}
</style>
