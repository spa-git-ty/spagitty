<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick } from 'svelte';
	import { BUILTIN_NOTES, complete, isBuiltin, segments, suggest, typingAt, type Typing } from './vars';
	import type { Found } from './model';

	/**
	 * A field that draws `{{name}}` as a pill (FEAT-160, proposal §5.2).
	 *
	 * The text is typed into an ordinary input, or a textarea with
	 * `multiline`, whose own letters are transparent; behind it a mirror
	 * draws the same text with each variable as a pill, in `--danger` when
	 * nothing defines it. The mirror is laid out exactly as the field is and
	 * scrolled with it, so the caret and the selection stay the field's own.
	 *
	 * Typing `{{` offers the names there are; arrows choose, Enter or Tab
	 * completes, Escape closes. Hovering a pill says what it holds.
	 */

	interface Props {
		value: string;
		onchange: (value: string) => void;
		/** Where a name is defined, or null when nothing defines it. */
		find: (name: string) => Found | null;
		/** Every name that can be offered. */
		names: string[];
		multiline?: boolean;
		rows?: number;
		placeholder?: string;
		label: string;
		mono?: boolean;
		onkeydown?: (event: KeyboardEvent) => void;
		input?: HTMLInputElement | HTMLTextAreaElement | null;
	}

	let {
		value,
		onchange,
		find,
		names,
		multiline = false,
		rows = 6,
		placeholder = '',
		label,
		mono = true,
		onkeydown,
		input = $bindable(null)
	}: Props = $props();

	let mirror = $state<HTMLDivElement | null>(null);
	let typing = $state<Typing | null>(null);
	let offered = $state<string[]>([]);
	let cursor = $state(0);
	let hover = $state('');

	const pieces = $derived(segments(value));

	function describeName(name: string): string {
		const found = find(name);
		if (!found) return `{{${name}}} is not defined here`;
		if (found.from === 'builtin') return `{{${name}}}: ${BUILTIN_NOTES[name as keyof typeof BUILTIN_NOTES]}`;
		const variable = found.variable;
		if (variable?.secret) return `{{${name}}} = •••••••• (secret, ${found.from})`;
		if (variable?.kept) return `{{${name}}} is kept from a response (${found.from})`;
		return `{{${name}}} = ${variable?.value ?? ''} (${found.from})`;
	}

	function sync() {
		if (!input || !mirror) return;
		mirror.scrollTop = input.scrollTop;
		mirror.scrollLeft = input.scrollLeft;
	}

	function look() {
		if (!input) return;
		const caret = input.selectionStart ?? value.length;
		typing = typingAt(value, caret);
		offered = typing ? suggest(typing.query, names) : [];
		cursor = 0;
	}

	function oninput(event: Event) {
		onchange((event.currentTarget as HTMLInputElement | HTMLTextAreaElement).value);
		void tick().then(() => {
			look();
			sync();
		});
	}

	async function choose(name: string) {
		if (!input || !typing) return;
		const caret = input.selectionStart ?? value.length;
		const next = complete(value, caret, typing, name);
		onchange(next.text);
		typing = null;
		offered = [];
		await tick();
		input.focus();
		input.setSelectionRange(next.caret, next.caret);
		sync();
	}

	function keydown(event: KeyboardEvent) {
		if (offered.length) {
			if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
				event.preventDefault();
				const step = event.key === 'ArrowDown' ? 1 : -1;
				cursor = (cursor + step + offered.length) % offered.length;
				return;
			}
			if ((event.key === 'Enter' && !event.ctrlKey && !event.metaKey) || event.key === 'Tab') {
				event.preventDefault();
				void choose(offered[cursor]);
				return;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				event.stopPropagation();
				offered = [];
				typing = null;
				return;
			}
		}
		onkeydown?.(event);
	}

	/** The pill under the pointer, found from the mirror's own boxes. */
	function onpointermove(event: PointerEvent) {
		if (!mirror) return;
		for (const pill of mirror.querySelectorAll<HTMLElement>('[data-name]')) {
			for (const box of pill.getClientRects()) {
				if (event.clientX >= box.left && event.clientX <= box.right && event.clientY >= box.top && event.clientY <= box.bottom) {
					hover = describeName(pill.dataset.name ?? '');
					return;
				}
			}
		}
		hover = '';
	}
</script>

<div class="field" class:multiline class:mono>
	<div class="mirror" bind:this={mirror} aria-hidden="true">
		{#each pieces as piece (piece.start)}{#if piece.name}<span
					class="pill"
					class:unknown={!find(piece.name)}
					class:builtin={isBuiltin(piece.name)}
					class:kept={find(piece.name)?.variable?.kept}
					data-name={piece.name}>{piece.text}</span
				>{:else}{piece.text}{/if}{/each}{#if multiline}&#8203;{/if}
	</div>
	{#if multiline}
		<textarea
			bind:this={input}
			{rows}
			{placeholder}
			aria-label={label}
			title={hover || undefined}
			spellcheck="false"
			autocomplete="off"
			{value}
			{oninput}
			onkeydown={keydown}
			onscroll={sync}
			onclick={look}
			onblur={() => setTimeout(() => (offered = []), 120)}
			{onpointermove}
		></textarea>
	{:else}
		<input
			bind:this={input}
			type="text"
			{placeholder}
			aria-label={label}
			title={hover || undefined}
			spellcheck="false"
			autocomplete="off"
			{value}
			{oninput}
			onkeydown={keydown}
			onscroll={sync}
			onkeyup={sync}
			onclick={look}
			onblur={() => setTimeout(() => (offered = []), 120)}
			{onpointermove}
		/>
	{/if}
	{#if offered.length}
		<ul class="offer" role="listbox" aria-label="Variables">
			{#each offered as name, index (name)}
				<li role="option" aria-selected={index === cursor}>
					<button
						type="button"
						class="mono"
						class:on={index === cursor}
						onmousedown={(event) => event.preventDefault()}
						onclick={() => choose(name)}
						><span>{name}</span><span class="note">{describeName(name).replace(/^\{\{[^}]*\}\}:? ?/, '')}</span></button
					>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.field {
		position: relative;
		min-width: 0;
		flex: 1;
	}

	.mirror,
	input,
	textarea {
		box-sizing: border-box;
		width: 100%;
		margin: 0;
		padding: 6px 10px;
		border: 1px solid transparent;
		font: inherit;
		font-size: var(--fs-ui);
		line-height: var(--lh-ui);
		letter-spacing: normal;
	}

	.mono .mirror,
	.mono input,
	.mono textarea {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.mirror {
		position: absolute;
		inset: 0;
		overflow: hidden;
		white-space: pre;
		color: var(--ink);
		pointer-events: none;
	}

	.multiline .mirror {
		white-space: pre-wrap;
		overflow-wrap: break-word;
	}

	input,
	textarea {
		position: relative;
		display: block;
		background: transparent;
		border-color: var(--line);
		border-radius: var(--r-field);
		color: transparent;
		caret-color: var(--ink);
	}

	textarea {
		resize: vertical;
		white-space: pre-wrap;
		overflow-wrap: break-word;
	}

	input::placeholder,
	textarea::placeholder {
		color: var(--placeholder);
	}

	input:focus,
	textarea:focus {
		border-color: var(--ring);
		outline: none;
	}

	input::selection,
	textarea::selection {
		color: transparent;
		background: var(--selection);
	}

	/* Drawn with a shadow, not padding: the pill must take exactly the room its letters do. */
	.pill {
		border-radius: 4px;
		background: color-mix(in srgb, var(--lane-5) 15%, transparent);
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--lane-5) 15%, transparent);
		color: var(--lane-5);
	}

	.pill.builtin {
		color: var(--agent);
		background: color-mix(in srgb, var(--agent) 13%, transparent);
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--agent) 13%, transparent);
	}

	.pill.kept {
		color: var(--ok);
		background: color-mix(in srgb, var(--ok) 14%, transparent);
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--ok) 14%, transparent);
	}

	.pill.unknown {
		color: var(--danger);
		background: color-mix(in srgb, var(--danger) 14%, transparent);
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--danger) 30%, transparent);
	}

	.offer {
		position: absolute;
		z-index: 30;
		top: calc(100% + 4px);
		left: 0;
		min-width: 260px;
		max-width: 420px;
		margin: 0;
		padding: 4px;
		list-style: none;
		background: var(--panel);
		border: 1px solid var(--soft);
		border-radius: var(--r-floating);
		box-shadow: var(--shadow-2);
	}

	.multiline .offer {
		top: auto;
		bottom: calc(100% + 4px);
	}

	.offer button {
		display: flex;
		justify-content: space-between;
		gap: 14px;
		width: 100%;
		padding: 4px 8px;
		border-radius: var(--r-row);
		font-size: var(--fs-mono);
		text-align: left;
	}

	.offer button.on,
	.offer button:hover {
		background: var(--hover);
	}

	.offer .note {
		overflow: hidden;
		color: var(--muted);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
