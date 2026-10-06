<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import * as api from './api';
	import ExtensionCard from './ExtensionCard.svelte';
	import InstallDialog from './InstallDialog.svelte';
	import { extensions } from './store.svelte';
	import type { InstallPreview } from './types';
	import Btn from '$lib/ui/Btn.svelte';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * Settings › Extensions (FEAT-096): what is installed, where it came from,
	 * and what it may do in the open repository.
	 */
	let preview = $state<InstallPreview | null>(null);
	let busy = $state(false);

	const listing = $derived(extensions.listing);
	const workdir = $derived(extensions.workdir);

	function describe(error: unknown): string {
		if (error && typeof error === 'object' && 'message' in error) return String((error as { message: unknown }).message);
		return String(error);
	}

	async function inspect(path: string) {
		busy = true;
		try {
			preview = await api.inspect(path, workdir);
		} catch (error) {
			notice.failed('The package was not installed', describe(error));
		} finally {
			busy = false;
		}
	}

	async function chooseFile() {
		const picked = await openDialog({
			multiple: false,
			directory: false,
			title: 'Choose an extension',
			filters: [{ name: 'Spagitty extension', extensions: ['spagitty-extension'] }]
		});
		if (typeof picked === 'string') await inspect(picked);
	}

	async function chooseFolder() {
		const picked = await openDialog({ multiple: false, directory: true, title: 'Choose the extension folder' });
		if (typeof picked !== 'string') return;
		try {
			const manifest = await api.attach(picked);
			notice.ok('Attached for development', manifest.id);
		} catch (error) {
			notice.failed('Could not attach that folder', describe(error));
		}
		await extensions.refresh();
	}

	async function install() {
		if (!preview) return;
		busy = true;
		try {
			const done = await api.install(preview.token);
			notice.ok(done.previous ? `Updated to ${done.version}` : 'Installed', done.id);
			preview = null;
		} catch (error) {
			notice.failed('The package was not installed', describe(error));
		} finally {
			busy = false;
			await extensions.refresh();
		}
	}

	onMount(() => {
		void extensions.refresh();
	});
</script>

<section class="section" aria-label="Extensions">
	<div class="row">
		<h2 class="heading" title={listing?.trust}>Extensions</h2>
		<div class="buttons">
			<Btn disabled={busy} onclick={chooseFile}>Install from file…</Btn>
			<Btn disabled={busy} onclick={chooseFolder} title="Use an extension folder you are developing">
				Attach folder…
			</Btn>
		</div>
	</div>

	{#if extensions.error}<p class="error">{extensions.error}</p>{/if}

	{#each listing?.extensions ?? [] as extension (extension.id)}
		<ExtensionCard {extension} {workdir} onupdate={inspect} />
	{:else}
		{#if listing}<p class="muted">None installed.</p>{/if}
	{/each}

	{#each listing?.problems ?? [] as problem (problem.source)}
		<p class="error" title={problem.source}>{problem.source}: {problem.reason}</p>
	{/each}
</section>

{#if preview}
	<InstallDialog {preview} {busy} oninstall={install} oncancel={() => (preview = null)} />
{/if}

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 12px;
		max-width: 720px;
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.buttons {
		display: flex;
		gap: 6px;
	}

	.muted {
		color: var(--muted);
		margin: 0;
	}

	.error {
		color: var(--danger);
		margin: 0;
		overflow-wrap: anywhere;
	}
</style>
