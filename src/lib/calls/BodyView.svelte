<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick } from 'svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { envColour, suggestName } from './model';
	import { formatJson, type Line, type Piece } from './view/json';
	import { formatXml, prefixesOf } from './view/xml';
	import type { Environment, KeepRule, Shape } from './types';

	/**
	 * A response body as lines (FEAT-160, proposal §5.4).
	 *
	 * JSON and XML are formatted and coloured unless `raw` is set or the body
	 * is over the colour limit (`plain`); either way only the lines near the
	 * viewport are drawn. Formatting is display only: what is copied or
	 * saved is the body as it arrived.
	 *
	 * With `onkeep`, a value can be clicked to keep it (FEAT-161, §6.2): the
	 * keep card opens inline under its line, never over the text. A value a
	 * rule of the call already keeps is underlined in `--ok` with `→ name`,
	 * and clicking it says so and offers *Forget*.
	 */

	interface KeepAsk {
		path: string;
		value: string;
		name: string;
		every: boolean;
		ns?: Record<string, string>;
	}

	interface Props {
		text: string;
		shape: Shape;
		raw: boolean;
		plain: boolean;
		/** The call's keep rules, to mark what is kept. */
		rules?: KeepRule[];
		/** Where a value is kept: the active environment, or none for shared. */
		environment?: Environment | null;
		/** Whether *Update on every call* starts on (Settings › Calls). */
		keepEvery?: boolean;
		/** Whether to offer a name from the path (Settings › Calls). */
		suggest?: boolean;
		onkeep?: (ask: KeepAsk) => Promise<boolean>;
		onforget?: (name: string) => void;
	}

	let {
		text,
		shape,
		raw,
		plain,
		rules = [],
		environment = null,
		keepEvery = true,
		suggest = true,
		onkeep,
		onforget
	}: Props = $props();

	interface Row {
		index: number;
		line: Line;
	}

	const formatted = $derived.by<{ lines: Line[]; ns: Record<string, string> }>(() => {
		if (!raw && !plain) {
			if (shape === 'json') {
				const lines = formatJson(text);
				if (lines) return { lines, ns: {} };
			} else if (shape === 'xml') {
				const xml = formatXml(text);
				if (xml) return xml;
			}
		}
		return { lines: text.split(/\r?\n/).map((line) => [{ text: line }]), ns: {} };
	});

	const rows = $derived<Row[]>(formatted.lines.map((line, index) => ({ index, line })));
	const keptAt = $derived(new Map(rules.map((rule) => [rule.from, rule])));
	const where = $derived(environment?.name ?? 'the shared variables');

	let open = $state<{ index: number; path: string; value: string } | null>(null);
	let name = $state('');
	let every = $state(true);
	let busy = $state(false);
	let nameInput = $state<HTMLInputElement | null>(null);

	/** A new body closes the card: its line may not be there any more. */
	$effect(() => {
		void text;
		open = null;
	});

	async function choose(index: number, piece: Piece) {
		if (!piece.path || piece.value === undefined) return;
		if (open?.index === index && open.path === piece.path) {
			open = null;
			return;
		}
		open = { index, path: piece.path, value: piece.value };
		name = keptAt.get(piece.path)?.var ?? (suggest ? suggestName(piece.path) : '');
		every = keepEvery;
		await tick();
		nameInput?.focus();
		nameInput?.select();
	}

	async function keep() {
		if (!open || !onkeep || !name.trim()) return;
		busy = true;
		const ns = shape === 'xml' ? prefixesOf(open.path, formatted.ns) : undefined;
		const done = await onkeep({ path: open.path, value: open.value, name: name.trim(), every, ns });
		busy = false;
		if (done) open = null;
	}

	function cardKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.stopPropagation();
			open = null;
		} else if (event.key === 'Enter' && !event.ctrlKey && !event.metaKey) {
			event.preventDefault();
			void keep();
		}
	}
</script>

<div class="body mono">
	<VirtualRows items={rows} key={(row) => String(row.index)} estimate={() => 20} label="Response body">
		{#snippet row(item: Row)}
			<div class="line">
				{#each item.line as piece, at (at)}{#if piece.path && onkeep}{@const rule = keptAt.get(piece.path)}<button
							type="button"
							class="value {piece.cls ? `tok-${piece.cls}` : ''}"
							class:kept={rule}
							class:chosen={open?.index === item.index && open.path === piece.path}
							title={rule ? `Kept as {{${rule.var}}}` : `Keep ${piece.path}`}
							onclick={() => choose(item.index, piece)}>{piece.text}</button
						>{#if rule}<span class="arrow">→ {rule.var}</span>{/if}{:else}<span
							class={piece.cls ? `tok-${piece.cls}` : undefined}
							title={piece.path}>{piece.text}</span
						>{/if}{/each}{#if item.line.every((piece) => piece.text === '')}&nbsp;{/if}
			</div>
			{#if open && open.index === item.index}
				{@const rule = keptAt.get(open.path)}
				<div class="card" role="dialog" aria-label="Keep a value" tabindex="-1" onkeydown={cardKey}>
					{#if rule}
						<p class="says">
							Kept as <span class="name">{`{{${rule.var}}}`}</span> in
							<span class="env" style:--env={envColour(environment?.color)}>{where}</span>
							· {rule.every ? 'updates on every call' : 'kept once'}
						</p>
						<div class="actions">
							<span class="path">{open.path}</span>
							<span class="gap"></span>
							<button type="button" class="act" onclick={() => (open = null)}>Close</button>
							<button
								type="button"
								class="act danger"
								onclick={() => {
									onforget?.(rule.var);
									open = null;
								}}>Forget</button
							>
						</div>
					{:else}
						<p class="says">Keep <span class="path">{open.path}</span></p>
						<div class="actions">
							<label class="as"
								>as <input
									bind:this={nameInput}
									class="mono"
									type="text"
									aria-label="Keep as"
									spellcheck="false"
									autocomplete="off"
									bind:value={name}
								/></label
							>
							<span class="in"
								>in <span class="env" style:--env={envColour(environment?.color)}
									><span class="dot" aria-hidden="true"></span>{where}</span
								></span
							>
							<label class="every"><input type="checkbox" bind:checked={every} /> Update on every call</label>
							<span class="gap"></span>
							<button type="button" class="act" onclick={() => (open = null)}>Cancel</button>
							<button type="button" class="act keep" disabled={busy || !name.trim()} onclick={keep}>Keep</button>
						</div>
					{/if}
				</div>
			{/if}
		{/snippet}
	</VirtualRows>
</div>

<style>
	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		font-size: var(--fs-code);
		line-height: var(--code-lh);
	}

	.line {
		padding: 0 12px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.value {
		padding: 0;
		border-radius: 3px;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.value:hover {
		background: var(--hover);
	}

	.value.chosen {
		outline: 1px solid var(--accent);
		background: color-mix(in srgb, var(--accent) 12%, transparent);
	}

	.value.kept {
		text-decoration: underline dotted var(--ok);
		text-underline-offset: 3px;
	}

	.arrow {
		margin-left: 0.6ch;
		color: var(--ok);
		font-size: var(--fs-secondary);
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin: 6px 12px 8px 40px;
		padding: 10px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
		background: var(--panel);
		box-shadow: var(--shadow-1);
		font-family: var(--font-ui);
		font-size: var(--fs-ui);
		white-space: normal;
	}

	.says {
		margin: 0;
	}

	.path {
		color: var(--muted);
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		overflow-wrap: anywhere;
	}

	.name {
		color: var(--ok);
		font-family: var(--font-mono);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
	}

	.as,
	.in,
	.every {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--muted);
	}

	.as input {
		width: 160px;
		padding: 4px 8px;
		color: var(--ink);
		font-size: var(--fs-mono);
	}

	.env {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		color: var(--ink);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--env);
	}

	.gap {
		flex: 1;
	}

	.act {
		padding: 3px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
	}

	.act:hover:not(:disabled) {
		background: var(--hover);
	}

	.act.keep {
		border-color: transparent;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	.act.danger {
		color: var(--danger);
	}
</style>
