<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import type { Private, Variable } from './types';

	/**
	 * An environment's variables, or the shared ones, as a table (FEAT-161,
	 * proposal §6.1): name, value, kind and where it is kept.
	 *
	 * - **Plain**: the value is in the repository's file, edited here.
	 * - **Secret**: the value is in the keychain. It is typed in as dots and
	 *   shown only while the eye is held.
	 * - **Kept**: filled from a response; shown, not edited here. *Freeze*
	 *   stops its rules updating it.
	 */

	interface Props {
		vars: Variable[];
		/** The kept values and which secrets have one, for this scope. */
		privateValues: Private | null;
		label: string;
		onvars: (vars: Variable[]) => void;
		/** Set a secret's value in the keychain, or remove it with `null`. */
		onsecret: (name: string, value: string | null) => Promise<void>;
		onreveal: (name: string) => Promise<string | null>;
		onforget: (name: string) => void;
	}

	let { vars, privateValues, label, onvars, onsecret, onreveal, onforget }: Props = $props();

	let menu = $state<{ x: number; y: number; anchor: HTMLElement; items: MenuItem[] } | null>(null);
	let shown = $state<Record<string, string>>({});
	let typed = $state<Record<string, string>>({});
	const timers = new Map<string, ReturnType<typeof setTimeout>>();

	const kept = $derived(new Map((privateValues?.kept ?? []).map((view) => [view.name, view])));
	const secrets = $derived(new Set(privateValues?.secrets ?? []));

	function set(index: number, change: Partial<Variable>) {
		onvars(vars.map((variable, at) => (at === index ? { ...variable, ...change } : variable)));
	}

	function typeSecret(name: string, value: string) {
		typed[name] = value;
		const timer = timers.get(name);
		if (timer) clearTimeout(timer);
		timers.set(
			name,
			setTimeout(() => {
				timers.delete(name);
				void onsecret(name, value === '' ? null : value);
			}, 400)
		);
	}

	async function hold(name: string) {
		const value = await onreveal(name);
		shown[name] = value ?? '';
	}

	function release(name: string) {
		const { [name]: _, ...rest } = shown;
		shown = rest;
	}

	async function makeSecret(index: number) {
		const variable = vars[index];
		if (variable.value) await onsecret(variable.name, variable.value);
		set(index, { secret: true, value: undefined });
	}

	async function makePlain(index: number) {
		const variable = vars[index];
		const value = (await onreveal(variable.name)) ?? '';
		set(index, { secret: false, value });
		await onsecret(variable.name, null);
	}

	function more(event: MouseEvent, index: number) {
		const anchor = event.currentTarget as HTMLElement;
		if (menu?.anchor === anchor) {
			menu = null;
			return;
		}
		const variable = vars[index];
		const box = anchor.getBoundingClientRect();
		const items: MenuItem[] = variable.kept
			? [
					{
						id: 'freeze',
						label: variable.frozen ? 'Update it again' : 'Freeze its value',
						run: () => set(index, { frozen: !variable.frozen })
					},
					{ separator: true },
					{ id: 'forget', label: 'Forget', danger: true, run: () => onforget(variable.name) }
				]
			: [
					variable.secret
						? { id: 'plain', label: 'Make it plain', note: 'into the repository', run: () => makePlain(index) }
						: { id: 'secret', label: 'Make it secret', note: 'into the keychain', run: () => makeSecret(index) },
					{ separator: true },
					{
						id: 'remove',
						label: 'Remove',
						danger: true,
						run: async () => {
							if (variable.secret) await onsecret(variable.name, null);
							onvars(vars.filter((_, at) => at !== index));
						}
					}
				];
		menu = { x: box.right - 180, y: box.bottom + 4, anchor, items };
	}

	function add() {
		let name = 'name';
		for (let n = 2; vars.some((variable) => variable.name === name); n++) name = `name${n}`;
		onvars([...vars, { name, value: '' }]);
	}
</script>

<div class="table" role="table" aria-label={label}>
	<div class="row head" role="row">
		<span role="columnheader">Name</span>
		<span role="columnheader">Value</span>
		<span role="columnheader">Kind</span>
		<span role="columnheader">Kept in</span>
		<span></span>
	</div>
	{#each vars as variable, index (index)}
		{@const view = kept.get(variable.name)}
		<div class="row" role="row">
			<input
				class="mono name"
				class:kept={variable.kept}
				type="text"
				aria-label="Name"
				spellcheck="false"
				value={variable.name}
				readonly={variable.secret || variable.kept}
				title={variable.secret || variable.kept ? 'Remove it and add it again to rename it' : undefined}
				oninput={(event) => set(index, { name: event.currentTarget.value })}
			/>
			{#if variable.secret}
				<span class="secret">
					<input
						class="mono"
						type={variable.name in shown ? 'text' : 'password'}
						aria-label={`The value of ${variable.name}`}
						placeholder={secrets.has(variable.name) ? '••••••••••' : 'no value yet'}
						autocomplete="off"
						value={shown[variable.name] ?? typed[variable.name] ?? ''}
						readonly={variable.name in shown}
						oninput={(event) => typeSecret(variable.name, event.currentTarget.value)}
					/>
					<button
						type="button"
						class="eye"
						aria-label={`Hold to show ${variable.name}`}
						title="Hold to show"
						disabled={!secrets.has(variable.name)}
						onpointerdown={() => hold(variable.name)}
						onpointerup={() => release(variable.name)}
						onpointerleave={() => release(variable.name)}><Icon name="review" size={14} /></button
					>
				</span>
			{:else if variable.kept}
				<span class="mono value kept-value" title={view?.value ?? ''}>{view?.value ?? '—'}</span>
			{:else}
				<input
					class="mono"
					type="text"
					aria-label={`The value of ${variable.name}`}
					spellcheck="false"
					value={variable.value ?? ''}
					oninput={(event) => set(index, { value: event.currentTarget.value })}
				/>
			{/if}
			<span
				class="kind"
				class:warn={variable.secret}
				class:ok={variable.kept && !view?.stale}
				class:stale={variable.kept && view?.stale}
			>
				{#if variable.secret}Secret{:else if variable.kept}Kept · {variable.frozen
						? 'frozen'
						: view?.stale
							? 'stale'
							: 'every call'}{:else}Plain{/if}
			</span>
			<span class="where">{variable.secret || variable.kept ? 'This computer' : 'Repository'}</span>
			<button type="button" class="more" aria-label={`More for ${variable.name}`} onclick={(event) => more(event, index)}
				>⋯</button
			>
		</div>
	{/each}
	<button type="button" class="add" onclick={add}>+ Add variable</button>
</div>

{#if menu}
	<Menu x={menu.x} y={menu.y} anchor={menu.anchor} label="Variable" items={menu.items} onclose={() => (menu = null)} />
{/if}

<style>
	.table {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.row {
		display: grid;
		grid-template-columns: minmax(120px, 1fr) minmax(160px, 2fr) 120px 110px 28px;
		align-items: center;
		gap: 10px;
	}

	.head {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	input {
		min-width: 0;
		padding: 5px 9px;
		font-size: var(--fs-mono);
	}

	.name {
		font-weight: 600;
	}

	.name.kept {
		color: var(--ok);
	}

	.secret {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}

	.secret input {
		flex: 1;
	}

	.eye {
		padding: 4px;
		border-radius: var(--r-button);
		color: var(--muted);
	}

	.eye:hover:not(:disabled) {
		background: var(--hover);
		color: var(--ink);
	}

	.value {
		overflow: hidden;
		padding: 5px 9px;
		font-size: var(--fs-mono);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.kind,
	.where {
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.kind.warn {
		color: var(--warn);
	}

	.kind.ok {
		color: var(--ok);
	}

	.kind.stale {
		color: var(--warn);
	}

	.more {
		padding: 2px 6px;
		border-radius: var(--r-button);
		color: var(--muted);
	}

	.more:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.add {
		align-self: flex-start;
		padding: 4px 2px;
		color: var(--muted);
	}

	.add:hover {
		color: var(--ink);
	}
</style>
