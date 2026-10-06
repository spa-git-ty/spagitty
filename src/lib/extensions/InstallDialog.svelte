<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Modal from './Modal.svelte';
	import type { InstallPreview } from './types';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * What a package is before it is installed: its identity as it states it,
	 * the targets it carries programs for, what it asks to do, its files and
	 * their checksums, and the trust model said plainly. Nothing in it has
	 * been run.
	 */
	interface Props {
		preview: InstallPreview;
		busy?: boolean;
		oninstall: () => void;
		oncancel: () => void;
	}

	let { preview, busy = false, oninstall, oncancel }: Props = $props();
	const verb = $derived(preview.replaces ? `Update to ${preview.version}` : 'Install');
	const size = (bytes: number) =>
		bytes < 1024 ? `${bytes} B` : bytes < 1048576 ? `${Math.round(bytes / 1024)} KB` : `${(bytes / 1048576).toFixed(1)} MB`;
</script>

<Modal title={preview.replaces ? `Update ${preview.name}` : `Install ${preview.name}`} wide onclose={oncancel}>
	<dl class="facts">
		<dt>Identity</dt>
		<dd class="mono">{preview.id}</dd>
		<dt>Version</dt>
		<dd>{preview.version}{#if preview.replaces} (replaces {preview.replaces}){/if}</dd>
		<dt>Publisher</dt>
		<dd>{preview.publisher} <span class="muted">— as the package says; not verified</span></dd>
		{#if preview.license}<dt>Licence</dt><dd>{preview.license}</dd>{/if}
		<dt>Programs for</dt>
		<dd class="mono">{preview.targets.join(', ')}</dd>
		<dt>Checksum</dt>
		<dd class="mono digest" title="SHA-256 of the package file">{preview.digest}</dd>
	</dl>

	{#if preview.description}<p class="muted">{preview.description}</p>{/if}

	{#if !preview.compatibility.compatible}
		<p class="error">{preview.compatibility.reasons.join(' ')}</p>
	{/if}

	{#if preview.capabilities.length}
		<div class="list" aria-label="What it asks to do">
			{#each preview.capabilities as capability (capability.capability)}
				<div class="item">
					<span>{capability.description}</span>
					<span class="muted">
						{capability.required ? 'required' : 'optional'}{#if preview.addedCapabilities.includes(capability.capability)}
							· new in this version{/if}
					</span>
				</div>
			{/each}
		</div>
	{/if}

	{#each preview.warnings as warning (warning)}<p class="warn">{warning}</p>{/each}

	<p class="trust">{preview.trust}</p>

	<details>
		<summary>{preview.files.length} files</summary>
		<ul class="files">
			{#each preview.files as file (file.path)}
				<li class="mono" title={file.sha256}>
					{file.path} <span class="muted">{size(file.size)}{file.executable ? ' · program' : ''}</span>
				</li>
			{/each}
		</ul>
	</details>

	{#snippet actions()}
		<Btn onclick={oncancel}>Cancel</Btn>
		<Btn primary disabled={busy || !preview.compatibility.compatible} onclick={oninstall}>{verb}</Btn>
	{/snippet}
</Modal>

<style>
	.facts {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 2px 12px;
		margin: 0;
	}

	dt,
	.muted {
		color: var(--muted);
	}

	dd {
		margin: 0;
		overflow-wrap: anywhere;
	}

	.digest {
		font-size: calc(var(--fs-mono) - 1px);
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.item {
		display: flex;
		justify-content: space-between;
		gap: 12px;
	}

	.trust {
		margin: 0;
		padding: 8px;
		border-radius: var(--r-field);
		background: var(--warn-soft);
	}

	.error {
		color: var(--danger);
		margin: 0;
	}

	.warn {
		color: var(--warn);
		margin: 0;
	}

	p {
		margin: 0;
	}

	.files {
		margin: 4px 0 0;
		padding-left: 16px;
		max-height: 160px;
		overflow: auto;
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}
</style>
