<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { renderMarkdown } from './markdown';

	/**
	 * Markdown drawn as the host draws it (FEAT-094): a pull request's
	 * description, a comment. `renderMarkdown` keeps only what is safe to put
	 * in this window; this draws it.
	 *
	 * `compact` is for comments, which sit in a thread among others: tighter
	 * spacing, no rules under headings.
	 */
	interface Props {
		source: string;
		compact?: boolean;
	}

	let { source, compact = false }: Props = $props();

	const html = $derived(renderMarkdown(source));
</script>

<!-- Safe to draw: rebuilt from an allow-list in renderMarkdown. -->
<div class="md" class:compact>{@html html}</div>

<style>
	.md {
		color: var(--ink);
		font-family: var(--read-font, var(--font-ui));
		font-size: var(--read-size, var(--fs-ui));
		line-height: 1.6;
		overflow-wrap: anywhere;
		min-width: 0;
	}

	.md > :global(:first-child) {
		margin-top: 0;
	}

	.md > :global(:last-child) {
		margin-bottom: 0;
	}

	.md :global(p) {
		margin: 0 0 12px;
	}

	.md :global(h1),
	.md :global(h2) {
		font-weight: 650;
		margin: 20px 0 10px;
		padding-bottom: 6px;
		border-bottom: 1px solid var(--line);
	}

	.md :global(h1) {
		font-size: 1.4em;
	}

	.md :global(h2) {
		font-size: 1.2em;
	}

	.md :global(h3),
	.md :global(h4),
	.md :global(h5),
	.md :global(h6) {
		font-size: 1em;
		font-weight: 650;
		margin: 16px 0 8px;
	}

	.md :global(a) {
		color: var(--accent);
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	.md :global(code) {
		font-family: var(--code-font, var(--font-mono));
		font-size: 0.88em;
		padding: 1px 5px;
		background: var(--sunken);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-field);
	}

	.md :global(.code-block) {
		position: relative;
		margin: 12px 0;
		background: var(--sunken);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.md :global(.code-lang) {
		position: absolute;
		top: 6px;
		right: 10px;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.md :global(pre) {
		margin: 0;
		padding: 12px 14px;
		overflow-x: auto;
		font-family: var(--code-font, var(--font-mono));
		font-size: var(--fs-code, var(--fs-mono));
		line-height: 1.5;
		tab-size: 4;
	}

	.md :global(pre code) {
		padding: 0;
		background: none;
		border: none;
		font-size: inherit;
	}

	.md :global(blockquote) {
		margin: 12px 0;
		padding: 2px 14px;
		border-left: 3px solid var(--pane-edge);
		color: var(--muted);
	}

	.md :global(ul),
	.md :global(ol) {
		margin: 0 0 12px;
		padding-left: 24px;
	}

	.md :global(li) {
		margin: 3px 0;
	}

	.md :global(li > ul),
	.md :global(li > ol) {
		margin: 3px 0 0;
	}

	/* A task list: the box where the bullet was. */
	.md :global(li:has(> input[type='checkbox'])) {
		list-style: none;
		margin-left: -20px;
	}

	.md :global(input[type='checkbox']) {
		margin: 0 6px 0 0;
		vertical-align: -1px;
	}

	/* A table scrolls on its own rather than widening what holds it. */
	.md :global(table) {
		display: block;
		max-width: 100%;
		overflow-x: auto;
		margin: 12px 0;
		border-collapse: collapse;
		font-size: 0.94em;
	}

	.md :global(th),
	.md :global(td) {
		padding: 5px 10px;
		border: 1px solid var(--pane-edge);
		text-align: left;
		vertical-align: top;
		/* A cell widens the table, which scrolls, rather than break a version in two. */
		overflow-wrap: normal;
	}

	.md :global(td code),
	.md :global(th code) {
		white-space: nowrap;
	}

	.md :global(th) {
		font-weight: 650;
		background: color-mix(in srgb, var(--ink) 6%, transparent);
	}

	.md :global(tr:nth-child(2n) td) {
		background: color-mix(in srgb, var(--ink) 3%, transparent);
	}

	.md :global(details) {
		margin: 10px 0;
		padding: 6px 12px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	.md :global(summary) {
		cursor: pointer;
		font-weight: 600;
	}

	.md :global(details[open] > summary) {
		margin-bottom: 8px;
	}

	.md :global(hr) {
		border: none;
		border-top: 1px solid var(--line);
		margin: 18px 0;
	}

	.md :global(kbd) {
		font-family: var(--font-mono);
		font-size: 0.85em;
		padding: 1px 5px;
		border: 1px solid var(--pane-edge);
		border-bottom-width: 2px;
		border-radius: 4px;
	}

	.md.compact :global(p) {
		margin-bottom: 8px;
	}

	.md.compact :global(h1),
	.md.compact :global(h2) {
		font-size: 1.05em;
		border-bottom: none;
		padding-bottom: 0;
		margin: 10px 0 6px;
	}
</style>
