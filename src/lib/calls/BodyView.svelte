<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { formatJson, type Line } from './view/json';
	import { formatXml } from './view/xml';
	import type { Shape } from './types';

	/**
	 * A response body as lines (FEAT-160, proposal §5.4).
	 *
	 * JSON and XML are formatted and coloured unless `raw` is set or the body
	 * is over the colour limit (`plain`); either way only the lines near the
	 * viewport are drawn. Formatting is display only: what is copied or
	 * saved is the body as it arrived.
	 */

	interface Props {
		text: string;
		shape: Shape;
		raw: boolean;
		plain: boolean;
	}

	let { text, shape, raw, plain }: Props = $props();

	interface Row {
		index: number;
		line: Line;
	}

	const lines = $derived.by<Line[]>(() => {
		if (!raw && !plain) {
			if (shape === 'json') {
				const formatted = formatJson(text);
				if (formatted) return formatted;
			} else if (shape === 'xml' || shape === 'html') {
				const formatted = shape === 'xml' ? formatXml(text) : null;
				if (formatted) return formatted.lines;
			}
		}
		return text.split(/\r?\n/).map((line) => [{ text: line }]);
	});

	const rows = $derived<Row[]>(lines.map((line, index) => ({ index, line })));
</script>

<div class="body mono">
	<VirtualRows items={rows} key={(row) => String(row.index)} estimate={() => 20} label="Response body">
		{#snippet row(item: Row)}
			<div class="line">
				{#each item.line as piece, at (at)}<span
						class={piece.cls ? `tok-${piece.cls}` : undefined}
						title={piece.path}>{piece.text}</span
					>{/each}{#if item.line.every((piece) => piece.text === '')}&nbsp;{/if}
			</div>
		{/snippet}
	</VirtualRows>
</div>

<style>
	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		font-size: var(--fs-code);
		line-height: var(--code-lh);
	}

	.line {
		padding: 0 12px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>
