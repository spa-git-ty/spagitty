<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Markdown from '$lib/ui/Markdown.svelte';

	/**
	 * A pull request's description, drawn as its host draws it (FEAT-094):
	 * tables, task lists, `<details>`, code in colour. It was a hand-rolled
	 * subset that showed tables as rows of pipes and made a live link of
	 * `javascript:`; `renderMarkdown` keeps only what is safe in this window.
	 */
	interface Props {
		markdown: string;
	}

	let { markdown }: Props = $props();
</script>

<div class="markdown-view">
	{#if !markdown.trim()}
		<div class="empty-doc note">
			<p>No description or changelog provided for this pull request.</p>
		</div>
	{:else}
		<article class="doc-content">
			<Markdown source={markdown} />
		</article>
	{/if}
</div>

<style>
	.markdown-view {
		flex: 1;
		min-width: 0;
		height: 100%;
		overflow-y: auto;
		padding: 24px 32px;
		background: var(--bg);
		box-sizing: border-box;
	}

	.empty-doc {
		display: grid;
		place-items: center;
		min-height: 200px;
		color: var(--muted);
	}

	.doc-content {
		max-width: 820px;
		margin: 0 auto;
	}
</style>
