<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { parse, type Block, type Inline } from './markdown';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * Extension markdown, drawn as text (FEAT-096).
	 *
	 * Every leaf is a text node; nothing here or anywhere in the extension UI
	 * hands a string to the browser as markup. A link does not navigate: choosing it copies the address,
	 * which is the one action that cannot surprise anybody, and its title shows
	 * where it goes before it is chosen.
	 */
	interface Props {
		source: string;
		/** Draw blocks without their margins, for a one-line summary. */
		compact?: boolean;
	}

	let { source, compact = false }: Props = $props();
	const blocks = $derived(parse(source));

	async function copy(href: string) {
		try {
			await navigator.clipboard.writeText(href);
			notice.ok('Link copied', href);
		} catch {
			notice.failed('Could not copy the link', href);
		}
	}
</script>

{#snippet inline(nodes: Inline[])}
	{#each nodes as node, index (index)}
		{#if node.t === 'text'}{node.v}{:else if node.t === 'code'}<code>{node.v}</code
			>{:else if node.t === 'strong'}<strong>{@render inline(node.c)}</strong
			>{:else if node.t === 'em'}<em>{@render inline(node.c)}</em>{:else if node.t === 'link'}<button
				type="button"
				class="link"
				title={`Copy ${node.href}`}
				onclick={() => copy(node.href)}>{@render inline(node.c)}</button
			>{/if}
	{/each}
{/snippet}

{#snippet blockList(list: Block[])}
	{#each list as block, index (index)}
		{#if block.t === 'p'}
			<p>{@render inline(block.c)}</p>
		{:else if block.t === 'h'}
			<p class="heading level-{block.level}">{@render inline(block.c)}</p>
		{:else if block.t === 'pre'}
			<pre>{block.v}</pre>
		{:else if block.t === 'list'}
			{#if block.ordered}
				<ol>
					{#each block.items as item, i (i)}<li>{@render inline(item)}</li>{/each}
				</ol>
			{:else}
				<ul>
					{#each block.items as item, i (i)}<li>{@render inline(item)}</li>{/each}
				</ul>
			{/if}
		{:else if block.t === 'quote'}
			<blockquote>{@render blockList(block.c)}</blockquote>
		{/if}
	{/each}
{/snippet}

<div class="markdown" class:compact>
	{@render blockList(blocks)}
</div>

<style>
	.markdown {
		display: flex;
		flex-direction: column;
		gap: 6px;
		overflow-wrap: anywhere;
		min-width: 0;
	}

	.markdown.compact {
		gap: 2px;
	}

	p,
	ul,
	ol,
	blockquote {
		margin: 0;
	}

	ul,
	ol {
		padding-left: 18px;
	}

	.heading {
		font-weight: 600;
	}

	blockquote {
		padding-left: 8px;
		border-left: 2px solid var(--line);
		color: var(--muted);
	}

	pre,
	code {
		font-family: var(--font-mono);
		font-size: var(--fs-code);
	}

	pre {
		margin: 0;
		padding: 6px 8px;
		background: var(--sunken);
		border-radius: var(--r-field);
		overflow-x: auto;
		white-space: pre;
	}

	code {
		background: var(--sunken);
		border-radius: 4px;
		padding: 0 3px;
	}

	.link {
		all: unset;
		color: var(--accent);
		text-decoration: underline;
		cursor: pointer;
	}

	.link:focus-visible {
		outline: 2px solid var(--ring);
		outline-offset: 1px;
		border-radius: 2px;
	}
</style>
