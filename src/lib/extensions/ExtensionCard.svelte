<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import * as api from './api';
	import { PROVENANCE_LABELS, STATE_LABELS } from './contributions';
	import ExtensionPanels from './ExtensionPanels.svelte';
	import SettingField from './SettingField.svelte';
	import { extensions } from './store.svelte';
	import type { Capability, ExtensionView } from './types';
	import Btn from '$lib/ui/Btn.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * One extension in Settings › Extensions: who made it and where it came
	 * from, whether it can run here, what it may do in this repository, its
	 * settings and tools, and — behind Diagnostics — what it last said.
	 */
	interface Props {
		extension: ExtensionView;
		workdir: string | null;
		onupdate: (path: string) => void;
	}

	let { extension, workdir, onupdate }: Props = $props();
	let busy = $state(false);

	const sendsCode = $derived(extension.sendsCodeTo.length > 0);

	function describe(error: unknown): string {
		if (error && typeof error === 'object' && 'message' in error) return String((error as { message: unknown }).message);
		return String(error);
	}

	async function act(title: string, work: () => Promise<unknown>) {
		busy = true;
		try {
			await work();
		} catch (error) {
			notice.failed(title, describe(error));
		} finally {
			busy = false;
			await extensions.refresh();
		}
	}

	/** The words the person agrees to, kept with their consent. */
	function consentStatement(): string {
		const where = extension.sendsCodeTo.join('; ');
		return `Reviews in this repository send the code you choose, and its context, to ${where}, using your own account there. They can count against your plan.`;
	}

	async function toggle() {
		if (!workdir) return;
		const dir = workdir;
		if (extension.enabled) {
			await act('Could not turn it off', () => api.disable(extension.id, dir, false));
			return;
		}
		let consent: string | null = null;
		if (sendsCode && !extension.consented) {
			const statement = consentStatement();
			const yes = await dialog.confirm({
				title: `Turn on ${extension.name} here?`,
				body: `${statement} Turning it on reviews nothing; each review is started by you or by a farm policy you set.`,
				confirmLabel: 'Turn on'
			});
			if (!yes) return;
			consent = statement;
		}
		await act('Could not turn it on', () => api.enable(extension.id, dir, [], consent));
	}

	async function grant(capability: Capability, granted: boolean) {
		if (!workdir) return;
		const dir = workdir;
		await act('Could not change the permission', () => api.setGrant(extension.id, dir, capability, granted));
	}

	async function saveSetting(key: string, value: unknown) {
		await act('Could not save the setting', () => api.setSetting(extension.id, key, value, workdir));
	}

	async function choose(tool: string) {
		const picked = await openDialog({ multiple: false, directory: false, title: 'Choose the program' });
		if (typeof picked !== 'string') return;
		await act('Could not use that program', () => api.chooseExecutable(extension.id, tool, picked));
	}

	async function update() {
		const picked = await openDialog({
			multiple: false,
			directory: false,
			title: 'Choose the new version',
			filters: [{ name: 'Spagitty extension', extensions: ['spagitty-extension'] }]
		});
		if (typeof picked === 'string') onupdate(picked);
	}

	async function remove() {
		const development = extension.provenance === 'development';
		const yes = await dialog.confirm({
			title: development ? `Detach ${extension.name}?` : `Remove ${extension.name}?`,
			body: development
				? 'Spagitty stops using the folder. The folder is not changed.'
				: 'Its files and permissions are deleted. Programs it used that you installed yourself are left alone.',
			confirmLabel: development ? 'Detach' : 'Remove',
			danger: !development
		});
		if (!yes) return;
		let keepHistory = true;
		if (workdir) {
			keepHistory = !(await dialog.confirm({
				title: 'Delete its review history too?',
				body: 'The reviews it saved for this repository.',
				confirmLabel: 'Delete history',
				danger: true
			}));
		}
		await act('Could not remove it', () => api.uninstall(extension.id, keepHistory, workdir));
	}
</script>

<article class="card" aria-label={extension.name}>
	<header class="head">
		<div class="identity">
			<span class="name">{extension.name}</span>
			<span class="version">{extension.version}</span>
			<span
				class="badge"
				class:official={extension.official}
				title={extension.official ? 'Ships with Spagitty' : `Publisher as stated by the package: ${extension.publisher}`}
			>
				{PROVENANCE_LABELS[extension.provenance]}
			</span>
		</div>
		<span class="state state-{extension.state}" title={extension.stateReason ?? undefined}>
			{STATE_LABELS[extension.state]}
		</span>
	</header>

	{#if extension.description}<p class="description">{extension.description}</p>{/if}
	{#if extension.stateReason && extension.state !== 'disabled'}
		<p class="reason" class:error={extension.state === 'failed' || extension.state === 'incompatible'}>
			{extension.stateReason}
		</p>
	{/if}

	<div class="row">
		{#if workdir && extension.compatibility.compatible}
			<label class="switch">
				<input type="checkbox" checked={extension.enabled} disabled={busy} onchange={toggle} />
				<span>On for this repository</span>
			</label>
		{/if}
		<div class="buttons">
			{#if extension.state === 'failed'}
				<Btn disabled={busy} onclick={() => act('Could not restart it', () => api.restart(extension.id))}>Restart</Btn>
			{/if}
			{#if extension.provenance === 'local'}
				<Btn disabled={busy} onclick={update}>Update from file…</Btn>
				{#if extension.previousVersion}
					<Btn
						disabled={busy}
						title="Go back to the version before the last update"
						onclick={() => act('Could not roll back', () => api.rollback(extension.id))}
					>
						Roll back to {extension.previousVersion}
					</Btn>
				{/if}
			{/if}
			{#if extension.provenance !== 'bundled'}
				<Btn danger={extension.provenance === 'local'} disabled={busy} onclick={remove}>
					{extension.provenance === 'development' ? 'Detach' : 'Remove'}
				</Btn>
			{/if}
		</div>
	</div>

	{#if extension.enabled}
		{#if extension.capabilities.length}
			<div class="group" aria-label="Permissions">
				{#each extension.capabilities as capability (capability.capability)}
					<label class="permission" title={capability.capability}>
						<input
							type="checkbox"
							checked={capability.granted}
							disabled={capability.required || busy}
							onchange={(e) => grant(capability.capability, e.currentTarget.checked)}
						/>
						<span>{capability.description}</span>
						{#if capability.required}<span class="muted">required</span>{/if}
					</label>
				{/each}
			</div>
		{/if}

		{#if extension.tools.length}
			<div class="group" aria-label="Programs">
				{#each extension.tools as tool (tool.id)}
					<div class="tool">
						<span class="tool-name">{tool.name}</span>
						<span
							class="tool-state"
							class:error={tool.detected && (!tool.detected.found || tool.detected.compatible === false)}
							title={tool.detected?.path ?? undefined}
						>
							{#if !tool.detected}
								Not checked
							{:else if !tool.detected.found || tool.detected.compatible === false}
								{tool.detected.reason ?? 'Not found'}
							{:else}
								{tool.detected.version ?? 'Found'}
							{/if}
						</span>
						<Btn disabled={busy} onclick={() => act('Could not check it', () => api.detectTool(extension.id, tool.id))}>
							Check
						</Btn>
						<Btn disabled={busy} onclick={() => choose(tool.id)}>Choose…</Btn>
						{#if tool.chosen}
							<Btn
								disabled={busy}
								title={tool.chosen}
								onclick={() => act('Could not reset it', () => api.chooseExecutable(extension.id, tool.id, null))}
							>
								Use PATH
							</Btn>
						{/if}
					</div>
					{#if tool.installUrl && tool.detected && !tool.detected.found}
						<p class="muted install" title={tool.installUrl}>Install it from {tool.installUrl}</p>
					{/if}
				{/each}
			</div>
		{/if}

		{#if extension.settings.some((s) => s.type !== 'executable')}
			<div class="group" aria-label="Settings">
				{#each extension.settings.filter((s) => s.type !== 'executable') as setting (setting.key)}
					<SettingField {setting} disabled={busy} onchange={(value) => void saveSetting(setting.key, value)} />
				{/each}
			</div>
		{/if}
	{/if}

	{#if extension.enabled && extension.state !== 'failed'}
		<ExtensionPanels location="global" only={extension.id} auto={false} />
	{/if}

	<details class="diagnostics">
		<summary>Diagnostics</summary>
		<dl>
			<dt>Identity</dt>
			<dd class="mono">{extension.id}</dd>
			<dt>Publisher</dt>
			<dd>{extension.publisher}</dd>
			{#if extension.license}<dt>Licence</dt><dd>{extension.license}</dd>{/if}
			{#if extension.diagnostics.program}<dt>Program</dt><dd class="mono">{extension.diagnostics.program}</dd>{/if}
			{#if !extension.compatibility.compatible}
				<dt>Why it cannot run</dt>
				<dd>{extension.compatibility.reasons.join(' ')}</dd>
			{/if}
		</dl>
		{#if extension.diagnostics.toolRuns.length}
			<p class="muted">Recent program runs</p>
			<ul class="runs">
				{#each extension.diagnostics.toolRuns as run, i (i)}
					<li class="mono">
						{run.tool} {run.args.join(' ')} → {run.cancelled ? 'cancelled' : run.timedOut ? 'timed out' : `exit ${run.exitCode}`}
						({Math.round(run.durationMs / 100) / 10}s)
					</li>
				{/each}
			</ul>
		{/if}
		{#if extension.diagnostics.logs.length}
			<p class="muted">Log</p>
			<pre>{extension.diagnostics.logs.join('\n')}</pre>
		{/if}
		{#if extension.diagnostics.stderr}
			<p class="muted">Error output</p>
			<pre>{extension.diagnostics.stderr}</pre>
		{/if}
	</details>
</article>

<style>
	.card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		border-radius: var(--r-panel);
		background: var(--surface);
		border: 1px solid var(--line);
	}

	.head,
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		flex-wrap: wrap;
	}

	.identity {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.name {
		font-weight: 600;
	}

	.version,
	.muted,
	.description {
		color: var(--muted);
	}

	.description,
	.reason {
		margin: 0;
	}

	.reason.error,
	.tool-state.error {
		color: var(--danger);
	}

	.badge {
		font-size: var(--fs-secondary);
		padding: 0 6px;
		border-radius: var(--r-pill);
		background: var(--sunken);
		color: var(--muted);
	}

	.badge.official {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.state {
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.state-active {
		color: var(--ok);
	}

	.state-failed,
	.state-incompatible {
		color: var(--danger);
	}

	.buttons {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.switch,
	.permission {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding-top: 6px;
		border-top: 1px solid var(--line);
	}

	.tool {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.tool-name {
		font-weight: 550;
	}

	.tool-state {
		flex: 1;
		color: var(--muted);
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.install {
		margin: 0;
		overflow-wrap: anywhere;
	}

	.diagnostics summary {
		cursor: pointer;
		color: var(--muted);
	}

	dl {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 2px 10px;
		margin: 6px 0;
	}

	dt {
		color: var(--muted);
	}

	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}

	pre,
	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	pre {
		margin: 0;
		max-height: 180px;
		overflow: auto;
		padding: 6px 8px;
		background: var(--sunken);
		border-radius: var(--r-field);
		white-space: pre-wrap;
	}

	.runs {
		margin: 0;
		padding-left: 16px;
	}
</style>
