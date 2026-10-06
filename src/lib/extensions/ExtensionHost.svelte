<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import Markdown from './Markdown.svelte';
	import Modal from './Modal.svelte';
	import ReviewScopeDialog from './ReviewScopeDialog.svelte';
	import { extensions } from './store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import { repo } from '$lib/repo.svelte';
	import { inTauri } from '$lib/api';

	/**
	 * The extension host's place in the shell, mounted once (FEAT-096): it
	 * subscribes to the host, keeps the open repository in step, and draws the
	 * two dialogs that can be asked for from anywhere — a review's scope, and a
	 * confirmation the backend needs before it posts something on the person's
	 * behalf.
	 */
	const confirmation = $derived(extensions.confirmations[0] ?? null);
	const path = $derived(repo.info?.path ?? null);

	$effect(() => {
		void extensions.setRepository(path);
	});

	$effect(() => {
		extensions.setWorkingChanges((repo.counts.working ?? 0) + (repo.counts.staged ?? 0) > 0);
	});

	onMount(() => {
		if (inTauri()) void extensions.start();
	});
	onDestroy(() => extensions.stop());
</script>

{#if extensions.draft}
	<ReviewScopeDialog draft={extensions.draft} />
{/if}

{#if confirmation}
	<Modal title={confirmation.title} wide onclose={() => extensions.answer(confirmation.id, false)}>
		<p class="target">{confirmation.target}</p>
		<div class="body"><Markdown source={confirmation.body} /></div>
		<details>
			<summary>Exact text</summary>
			<pre>{confirmation.body}</pre>
		</details>
		{#snippet actions()}
			<Btn onclick={() => extensions.answer(confirmation.id, false)}>Don't post</Btn>
			<Btn primary onclick={() => extensions.answer(confirmation.id, true)}>Post comment</Btn>
		{/snippet}
	</Modal>
{/if}

<style>
	.target {
		margin: 0;
		font-family: var(--font-mono);
		color: var(--muted);
	}

	.body {
		padding: 8px;
		border-radius: var(--r-field);
		background: var(--surface);
		border: 1px solid var(--line);
	}

	pre {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	details summary {
		cursor: pointer;
		color: var(--muted);
	}
</style>
