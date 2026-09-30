<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { fileHistory } from '$lib/history/store.svelte';
	import FileHistoryView from '$lib/history/FileHistoryView.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';

	let inputPath = $state('');

	onMount(() => {
		const paramPath = page.url.searchParams.get('path');
		if (paramPath && repo.info) {
			inputPath = paramPath;
			void fileHistory.inspect(paramPath);
		}
	});

	function inspectFile(): void {
		if (inputPath.trim()) {
			void fileHistory.inspect(inputPath.trim());
		}
	}
</script>

<div class="history-page">
	{#if !fileHistory.path}
		<div class="empty-prompt">
			<div class="prompt-box">
				<h2 class="title">File History & Blame</h2>
				<form
					class="input-form"
					onsubmit={(e) => {
						e.preventDefault();
						inspectFile();
					}}
				>
					<input
						type="text"
						class="field mono"
						placeholder="path/to/file.ext"
						bind:value={inputPath}
					/>
					<Btn primary onclick={inspectFile}>Inspect File</Btn>
				</form>
			</div>
		</div>
	{:else}
		<FileHistoryView />
	{/if}
</div>

<style>
	/* The pane's width, not the prompt's (FEAT-084): without `flex: 1` the page
	   shrank to its card and the card sat at the pane's left edge. */
	.history-page {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}

	.empty-prompt {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding: 32px;
	}

	/* A card in the ornaments' glass, as the graph's inspector is (FEAT-084). */
	.prompt-box {
		background: var(--glass-thick);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-panel);
		padding: 24px;
		max-width: 480px;
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
		color: var(--ink);
	}


	.input-form {
		display: flex;
		gap: 8px;
		margin-top: 8px;
	}

	/* The well every other field is; only its size is this screen's. */
	.field {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-secondary);
	}

	.mono {
		font-family: var(--font-mono);
	}
</style>
