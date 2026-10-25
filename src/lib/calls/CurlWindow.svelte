<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Sheet from '$lib/ui/Sheet.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import * as api from './api';
	import { methodOf } from './model';
	import type { Call, Collection, Opened, Pasted, Quoting, Stored } from './types';

	/**
	 * The cURL window (FEAT-166): the open call copied as one `curl`
	 * command, and a pasted one brought in.
	 *
	 * The copy is resolved in the active environment. Secrets and kept
	 * values stay `{{name}}` unless *Fill in secrets* is ticked, which is off
	 * every time the window opens. It is quoted for this computer's shell,
	 * with a choice to switch. A paste says which flags it read and which it
	 * did not, and either replaces the open call's request or becomes a new
	 * call.
	 */

	interface Props {
		opened: Opened;
		call: Call;
		environment: string | null;
		collections: Stored<Collection>[];
		onreplace: (call: Call) => void;
		oncreate: (call: Call, stem: string, folder: string | null) => void;
		onclose: () => void;
	}

	let { opened, call, environment, collections, onreplace, oncreate, onclose }: Props = $props();

	const windows = typeof navigator !== 'undefined' && /Win/i.test(navigator.platform || navigator.userAgent || '');
	let side = $state<'copy' | 'paste'>('copy');
	let quoting = $state<Quoting>(windows ? 'cmd' : 'posix');
	let secrets = $state(false);
	let command = $state('');
	let copyError = $state<string | null>(null);
	let pasted = $state('');
	let read = $state<Pasted | null>(null);
	let readError = $state<string | null>(null);
	let target = $state<'replace' | 'new'>('replace');
	const editable = $derived(collections.filter((stored) => stored.value));
	let into = $state<string | null>(null);
	let folder = $state('');
	let seq = 0;
	let timer: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		into ??= opened.stem.startsWith('found:') ? (editable[0]?.stem ?? null) : opened.stem;
	});

	$effect(() => {
		const ask = { stem: opened.stem.startsWith('found:') ? null : opened.stem, call: JSON.parse(JSON.stringify(call)) as Call, environment, quoting, secrets };
		const ticket = ++seq;
		api.curl(ask)
			.then((text) => {
				if (ticket !== seq) return;
				command = text;
				copyError = null;
			})
			.catch((cause) => {
				if (ticket !== seq) return;
				command = '';
				copyError = String(cause);
			});
	});

	function onpaste(text: string) {
		pasted = text;
		if (timer) clearTimeout(timer);
		timer = setTimeout(async () => {
			if (!pasted.trim()) {
				read = null;
				readError = null;
				return;
			}
			try {
				read = await api.curlParse(pasted);
				readError = null;
			} catch (cause) {
				read = null;
				readError = String(cause);
			}
		}, 200);
	}

	async function copy() {
		try {
			await navigator.clipboard.writeText(command);
			notice.ok(secrets ? 'Copied, with its secrets' : 'Copied');
		} catch (cause) {
			notice.failed('Could not copy the command', cause);
		}
	}

	function apply() {
		if (!read) return;
		if (target === 'replace') onreplace(read.call);
		else if (into) oncreate(read.call, into, folder.trim() || null);
		onclose();
	}
</script>

<Sheet labelledby="curl-title" width="680px" ondismiss={onclose}>
	<div class="curl">
		<header>
			<h2 id="curl-title">cURL</h2>
			<div class="sides" role="tablist" aria-label="Copy or paste">
				<button type="button" role="tab" class="side" class:on={side === 'copy'} aria-selected={side === 'copy'} onclick={() => (side = 'copy')}>Copy</button>
				<button type="button" role="tab" class="side" class:on={side === 'paste'} aria-selected={side === 'paste'} onclick={() => (side = 'paste')}>Paste</button>
			</div>
		</header>

		{#if side === 'copy'}
			<div class="row">
				<div class="choices" role="radiogroup" aria-label="Quoted for">
					<button type="button" role="radio" class="choice" class:on={quoting === 'posix'} aria-checked={quoting === 'posix'} onclick={() => (quoting = 'posix')}>Linux and macOS</button>
					<button type="button" role="radio" class="choice" class:on={quoting === 'cmd'} aria-checked={quoting === 'cmd'} onclick={() => (quoting = 'cmd')}>Windows cmd</button>
				</div>
				<span class="gap"></span>
				<label class="check" title="Off every time the window opens"><input type="checkbox" bind:checked={secrets} /> Fill in secrets</label>
			</div>
			{#if copyError}
				<p class="error" role="alert">{copyError}</p>
			{:else}
				<textarea class="mono" rows="10" readonly aria-label="The command" value={command}></textarea>
			{/if}
			<p class="hint">
				{methodOf(call)}
				{call.name}, in {environment ?? 'no environment'}. {secrets ? 'Secrets are filled in: keep the command to yourself.' : 'Secrets and kept values stay as their names.'}
			</p>
			<footer>
				<span class="gap"></span>
				<button type="button" class="act" onclick={onclose}>Close</button>
				<button type="button" class="act primary" disabled={!command} onclick={copy}>Copy</button>
			</footer>
		{:else}
			<textarea
				class="mono"
				rows="8"
				aria-label="A curl command"
				placeholder="curl https://…"
				spellcheck="false"
				value={pasted}
				oninput={(event) => onpaste(event.currentTarget.value)}
			></textarea>
			{#if readError}
				<p class="error" role="alert">{readError}</p>
			{:else if read}
				<p class="hint">
					<strong class="method">{read.call.method}</strong>
					<span class="mono">{read.call.url}</span>
				</p>
				<p class="hint">
					{#if read.read.length}Read <span class="mono">{read.read.join(' ')}</span>.{/if}
					{#if read.ignored.length}Not read: <span class="mono">{read.ignored.join(' ')}</span>.{/if}
				</p>
			{/if}
			<div class="choices" role="radiogroup" aria-label="Where it goes">
				<button type="button" role="radio" class="choice" class:on={target === 'replace'} aria-checked={target === 'replace'} onclick={() => (target = 'replace')}>Replace this call's request</button>
				<button type="button" role="radio" class="choice" class:on={target === 'new'} aria-checked={target === 'new'} disabled={editable.length === 0} onclick={() => (target = 'new')}>A new call</button>
			</div>
			{#if target === 'new'}
				<div class="row">
					<select aria-label="Collection" value={into ?? ''} onchange={(event) => (into = event.currentTarget.value)}>
						{#each editable as stored (stored.stem)}
							<option value={stored.stem}>{stored.value?.name}</option>
						{/each}
					</select>
					<input type="text" aria-label="Folder" placeholder="folder, optional" spellcheck="false" bind:value={folder} />
				</div>
			{/if}
			<footer>
				<span class="gap"></span>
				<button type="button" class="act" onclick={onclose}>Cancel</button>
				<button type="button" class="act primary" disabled={!read} onclick={apply}>{target === 'replace' ? 'Replace the request' : 'Add the call'}</button>
			</footer>
		{/if}
	</div>
</Sheet>

<style>
	.curl {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 16px 18px;
	}

	header,
	.row,
	footer {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	h2 {
		margin: 0 8px 0 0;
		font-size: var(--fs-title);
		font-weight: normal;
	}

	.sides,
	.choices {
		display: flex;
		gap: 4px;
	}

	.side {
		padding: 3px 12px;
		border-radius: var(--r-pill);
		color: var(--muted);
	}

	.side.on {
		background: var(--surface-2);
		color: var(--ink);
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

	.gap {
		flex: 1;
	}

	.check {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	textarea {
		resize: vertical;
		font-size: var(--fs-mono);
	}

	.hint {
		margin: 0;
		color: var(--muted);
		font-size: var(--fs-secondary);
		overflow-wrap: anywhere;
	}

	.method {
		color: var(--ink);
	}

	.error {
		margin: 0;
		color: var(--danger);
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
</style>
