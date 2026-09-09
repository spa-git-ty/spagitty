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
				<p class="desc">
					Inspect the commit evolution and line-by-line attribution of any file in the repository.
				</p>
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
	.history-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg);
		overflow: hidden;
	}

	.empty-prompt {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding: 32px;
	}

	.prompt-box {
		background: var(--panel);
		border: 1px solid var(--line);
		border-radius: var(--r-field, 6px);
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

	.desc {
		margin: 0;
		font-size: var(--fs-secondary);
		color: var(--muted);
		line-height: 1.4;
	}

	.input-form {
		display: flex;
		gap: 8px;
		margin-top: 8px;
	}

	.field {
		flex: 1;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 4px;
		padding: 8px 10px;
		font-size: var(--fs-secondary);
		color: var(--ink);
		outline: none;
	}

	.field:focus {
		border-color: var(--accent);
	}

	.mono {
		font-family: var(--font-mono);
	}
</style>
