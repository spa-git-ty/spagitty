<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import { builder } from './builder.svelte';
	import { CATALOGUE, PRESETS, STAGES, type Action, type Block, type Stage } from './builder-types';

	/** The editor changes a typed block. It never evaluates a command or a message rule. */
	const block = $derived(builder.block);
	const action = $derived(block?.action);
	const kind = $derived(CATALOGUE.find((item) => item.kind === action?.kind));
	let message = $state('feat(api): add a refund');
	function edit(patch: Partial<Block>) { if (block) builder.set({ ...block, ...patch }); }
	function settings(patch: Record<string, unknown>) { if (block) edit({ action: { ...block.action, ...patch } as Action }); }
	const lines = (text: string) => text.split('\n').map((value) => value.trim()).filter(Boolean);
	const words = (text: string) => text.split(',').map((value) => value.trim()).filter(Boolean);
</script>

<section class="editor" aria-label="Hook block settings">
	{#if block && action}
		<header>
			<span class="square" style:--tone={kind?.tone}></span>
			<input class="name" aria-label="Block name" value={block.label} disabled={builder.readonly} oninput={(event) => edit({ label: event.currentTarget.value })} />
			<span class="kind">{kind?.label}</span>
			<Btn disabled={builder.readonly} onclick={() => builder.remove(block.id)}>Remove</Btn>
		</header>
		<fieldset disabled={builder.readonly || builder.saving || builder.running}>
			<div class="pair">
				<label>Stage<select aria-label="Block stage" value={block.stage} disabled={action.kind === 'message'} onchange={(event) => edit({ stage: event.currentTarget.value as Stage })}>{#each STAGES as stage}<option value={stage}>{stage}</option>{/each}</select></label>
				<label class="check"><input type="checkbox" checked={block.enabled} onchange={(event) => edit({ enabled: event.currentTarget.checked })} /> Enabled</label>
			</div>
			{#if action.kind === 'tests' || action.kind === 'format'}
				<label>When staged files match<textarea class="mono" aria-label="Path globs" rows="2" value={action.paths.join('\n')} placeholder="services/payments/**" oninput={(event) => settings({ paths: lines(event.currentTarget.value) })}></textarea></label>
				{#if action.kind === 'format'}
					<label>Tool preset<select aria-label="Tool preset" value={action.preset} onchange={(event) => settings({ preset: event.currentTarget.value, command: PRESETS[event.currentTarget.value] })}>{#each Object.keys(PRESETS) as preset}<option value={preset}>{preset}</option>{/each}</select></label>
				{/if}
				<label>{action.kind === 'tests' ? 'Run' : 'Command · staged filenames are appended'}<textarea class="mono" aria-label="Block command" rows="3" value={action.command} oninput={(event) => settings({ command: event.currentTarget.value })}></textarea></label>
				{#if action.kind === 'format'}
					<label class="check"><input type="checkbox" checked={action.restage} onchange={(event) => settings({ restage: event.currentTarget.checked })} /> Fix and restage</label>
					<p class="note">Files with unstaged changes are left alone. Their unstaged content is never added to the commit.</p>
				{/if}
			{:else if action.kind === 'message'}
				<label>Style<select aria-label="Message style" value={action.style} onchange={(event) => settings({ style: event.currentTarget.value })}><option value="conventional">Conventional Commits</option><option value="ticket">Ticket prefix</option><option value="regex">Custom regex</option></select></label>
				{#if action.style === 'conventional'}
					<label>Allowed types<input class="mono" aria-label="Allowed types" value={action.types.join(', ')} oninput={(event) => settings({ types: words(event.currentTarget.value) })} /></label>
					<label class="check"><input type="checkbox" checked={action.scopeRequired} onchange={(event) => settings({ scopeRequired: event.currentTarget.checked })} /> Scope required</label>
				{:else}
					<label>{action.style === 'ticket' ? 'Ticket prefix pattern' : 'Subject pattern'}<input class="mono" aria-label="Subject pattern" value={action.pattern} oninput={(event) => settings({ pattern: event.currentTarget.value })} /></label>
				{/if}
				<div class="pair"><label>Subject maximum<input type="number" min="1" max="1000" aria-label="Subject maximum" value={action.subjectMax} oninput={(event) => settings({ subjectMax: Number(event.currentTarget.value) })} /></label><label>Ticket pattern in footer<input class="mono" aria-label="Footer ticket pattern" value={action.ticketPattern} oninput={(event) => settings({ ticketPattern: event.currentTarget.value })} /></label></div>
			{:else if action.kind === 'script'}
				<label>Run a script or command<textarea class="mono" aria-label="Block command" rows="5" value={action.command} placeholder="./scripts/check.sh" oninput={(event) => settings({ command: event.currentTarget.value })}></textarea></label>
			{:else if action.kind === 'secrets'}
				<p class="note">Checks added lines for private keys, access tokens and cloud keys. Matched secrets are never printed.</p>
				<label>Extra patterns · one per line<textarea class="mono" aria-label="Extra secret patterns" rows="4" value={action.patterns.join('\n')} oninput={(event) => settings({ patterns: lines(event.currentTarget.value) })}></textarea></label>
			{:else if action.kind === 'largeFiles'}
				<label>Size limit · bytes<input type="number" min="1" aria-label="File size limit" value={action.bytes} oninput={(event) => settings({ bytes: Number(event.currentTarget.value) })} /></label>
			{:else if action.kind === 'protectBranches'}
				<label>Branch names · one per line<textarea class="mono" aria-label="Protected branches" rows="3" value={action.branches.join('\n')} oninput={(event) => settings({ branches: lines(event.currentTarget.value) })}></textarea></label>
			{/if}
			<div class="pair"><label>In folder<input class="mono" aria-label="Block folder" value={block.folder} oninput={(event) => edit({ folder: event.currentTarget.value })} /></label><label>Timeout · seconds<input type="number" min="1" max="86400" aria-label="Block timeout" value={block.timeoutSeconds} oninput={(event) => edit({ timeoutSeconds: Number(event.currentTarget.value) })} /></label></div>
			<div><span class="label">If it fails</span><div class="choices"><button class:on={!block.warn} aria-pressed={!block.warn} onclick={() => edit({ warn: false })}>Block the {block.stage === 'pre-push' ? 'push' : 'commit'}</button><button class:on={block.warn} aria-pressed={block.warn} onclick={() => edit({ warn: true })}>Warn and continue</button></div></div>
			<div><span class="label">Skip when</span><div class="choices"><label class="check"><input type="checkbox" checked={block.skipMarkdown} onchange={(event) => edit({ skipMarkdown: event.currentTarget.checked })} /> Only *.md changed</label><label class="check" title="The subject is available at commit-msg."><input type="checkbox" checked={block.skipWip} onchange={(event) => edit({ skipWip: event.currentTarget.checked })} /> WIP subject</label></div></div>
		</fieldset>
		{#if action.kind === 'tests' || action.kind === 'format'}
			<div class="preview"><span class="label">Against what is staged now</span>{#if builder.errors.matches}<p class="error">{builder.errors.matches}</p>{:else}<p class:quiet={builder.matches.length === 0}>{builder.matches.length ? `Would run · ${builder.matches.length} files match` : 'Would skip · no change matched'}</p><div class="paths mono">{builder.matches.slice(0, 8).join('\n')}{builder.matches.length > 8 ? `\n+${builder.matches.length - 8} more` : ''}</div>{/if}</div>
		{:else if action.kind === 'message'}
			<div class="preview"><label>Try a message<textarea class="mono" aria-label="Try a message" rows="3" bind:value={message}></textarea></label><Btn disabled={!message.trim() || builder.readonly} busy={builder.running} onclick={() => builder.tryMessage(message)}>Try message</Btn>{#if builder.tried}<p class:error={!builder.tried.passed}>{builder.tried.passed ? 'Passed' : 'Blocked'}</p><pre class="mono">{builder.tried.output}</pre>{/if}</div>
		{/if}
	{:else}
		<p class="note empty">Choose a block to edit it, or add one from the catalogue.</p>
	{/if}
</section>

<style>
	.editor { flex: 1; min-width: 0; overflow: auto; padding: 18px; background: var(--surface); border: 1px solid var(--pane-edge); border-radius: var(--r-panel); }
	header { display: flex; align-items: center; gap: 10px; margin-bottom: 18px; }
	.square { width: 9px; height: 9px; border-radius: 3px; background: var(--tone); flex: none; }
	.name { min-width: 0; flex: 1; font-size: var(--fs-title); font-weight: 600; background: transparent; border-color: transparent; padding: 2px; }
	.kind { color: var(--muted); border: 1px solid var(--line); border-radius: var(--r-pill); padding: 3px 10px; font-size: var(--fs-secondary); }
	fieldset { border: 0; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 18px; }
	label { display: flex; flex-direction: column; gap: 7px; color: var(--muted); font-size: var(--fs-secondary); }
	input, textarea, select { width: 100%; min-width: 0; color: var(--ink); font-size: var(--fs-ui); }
	input, textarea { padding: 8px 10px; }
	select { padding-top: 8px; padding-bottom: 8px; }
	textarea { resize: vertical; line-height: 1.5; }
	.pair { display: flex; align-items: center; gap: 14px; }
	.pair > label { flex: 1; min-width: 0; }
	.check { display: flex; flex-direction: row; align-items: center; gap: 8px; color: var(--ink); }
	.check input { width: auto; accent-color: var(--ok); }
	.label { color: var(--muted); font-size: var(--fs-secondary); }
	.choices { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 7px; }
	.choices button { padding: 4px 10px; border: 1px solid var(--line); border-radius: var(--r-pill); font-size: var(--fs-secondary); }
	.choices button.on { background: var(--hover); border-color: var(--muted); }
	.preview { padding: 14px; margin-top: 18px; border-radius: var(--r-button); background: var(--sunken); }
	.preview p { margin: 10px 0; }
	.paths, pre { white-space: pre-wrap; overflow-wrap: anywhere; font-size: var(--fs-mono); line-height: 1.5; color: var(--muted); }
	.note { margin: 0; line-height: 1.5; font-size: var(--fs-secondary); }
	.error { color: var(--danger); }
	.quiet { color: var(--muted); }
	.empty { padding-top: 15px; }
	@media (max-width: 1000px) { header { flex-wrap: wrap; } .kind { display: none; } .pair { flex-wrap: wrap; } }
</style>
