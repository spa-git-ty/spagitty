<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { fileKind } from './kind';

	/**
	 * A file's kind as a small coloured tag (FEAT-149): `TS` on blue, `{}` on
	 * amber, a grey sheet for what is not recognised. A folder is a folder, in
	 * the muted ink the tree names folders in.
	 */
	interface Props {
		path: string;
		folder?: boolean;
		open?: boolean;
	}

	let { path, folder = false, open = false }: Props = $props();

	const kind = $derived(fileKind(path));
</script>

{#if folder}
	<svg class="file-glyph folder" class:open viewBox="0 0 16 16" aria-hidden="true">
		{#if open}
			<path d="M1.5 4.5a1 1 0 0 1 1-1h3.4l1.4 1.5h5.7a1 1 0 0 1 1 1v.5H4.2a1 1 0 0 0-.95.68L1.5 12.5z" />
			<path class="lid" d="M3.3 7.2a1 1 0 0 1 .95-.7h10.3a.6.6 0 0 1 .57.8l-1.7 5.1a1 1 0 0 1-.95.6H1.9z" />
		{:else}
			<path d="M1.5 4a1 1 0 0 1 1-1h3.4l1.4 1.5h6.2a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z" />
		{/if}
	</svg>
{:else}
	<span
		class="file-glyph tag"
		class:grey={kind.hue === null}
		class:long={kind.tag.length > 2}
		style:--hue={kind.hue ?? 0}
		data-tag={kind.tag}
		title={kind.label}
		aria-hidden="true"
	></span>
{/if}

<style>
	.file-glyph {
		flex: none;
		width: 18px;
		height: 14px;
	}

	.folder {
		fill: color-mix(in srgb, var(--muted) 55%, transparent);
	}

	.folder .lid {
		fill: var(--muted);
	}

	/*
	 * The tint and the letters are one hue at two strengths, mixed into the
	 * theme's own ink and ground, so a tag sits as quietly in light as in dark.
	 */
	.tag {
		display: inline-grid;
		place-items: center;
		border-radius: 3px;
		font-family: var(--font-mono);
		font-size: calc(var(--fs-mono) * 0.66);
		font-weight: 700;
		line-height: 1;
		letter-spacing: -0.02em;
		color: color-mix(in oklch, oklch(68% 0.16 var(--hue)) 80%, var(--ink));
		background: color-mix(in oklch, oklch(68% 0.16 var(--hue)) 20%, transparent);
		box-shadow: inset 0 0 0 1px color-mix(in oklch, oklch(68% 0.16 var(--hue)) 35%, transparent);
	}

	/* Drawn, not written: the letters are not part of the row's text, so a
	   name copied or read from the row is only the name. */
	.tag::before {
		content: attr(data-tag);
	}

	.tag.long {
		font-size: calc(var(--fs-mono) * 0.58);
		letter-spacing: -0.04em;
	}

	.tag.grey {
		color: var(--muted);
		background: var(--soft);
		box-shadow: inset 0 0 0 1px var(--pane-edge);
	}
</style>
