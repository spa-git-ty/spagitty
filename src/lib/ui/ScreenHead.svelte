<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * The head of a list screen (BUG-056): the title, the repository it is
	 * about in the same mono pill on every screen that names one, a line of
	 * detail, and the actions at the right. Pull requests and Review drew their
	 * own and did not line up.
	 */
	interface Props {
		title: string;
		/** `owner/name`, when the screen is about one repository. */
		repository?: string | null;
		detail?: Snippet;
		actions?: Snippet;
	}

	let { title, repository = null, detail, actions }: Props = $props();
</script>

<header class="head">
	<div class="left">
		<span class="title">{title}</span>
		{#if repository}<span class="repository mono">{repository}</span>{/if}
		{@render detail?.()}
	</div>
	<div class="right">{@render actions?.()}</div>
</header>

<style>
	.head {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		min-height: 52px;
		box-sizing: border-box;
		padding: 10px 16px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.left {
		flex: 1;
	}

	.right {
		flex: none;
	}

	.title {
		font-size: var(--fs-title);
		flex: none;
	}

	.repository {
		font-size: var(--fs-mono);
		color: var(--muted);
		padding: 2px 8px;
		border-radius: 6px;
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
