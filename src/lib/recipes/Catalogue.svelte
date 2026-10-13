<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { CATALOGUE, GROUPS } from './catalogue';
	import type { BlockId } from './types';

	/**
	 * The blocks, by group, each with the key that adds it (FEAT-133).
	 *
	 * A click or its key puts a block at the end of the stack. These eighteen
	 * are all there are: what is not here cannot be in a recipe.
	 *
	 * Neutral cards, with a small muted square for the group and nothing
	 * louder. The author asked for a tool for professionals, not a toy.
	 */
	interface Props {
		onadd: (block: BlockId) => void;
	}

	let { onadd }: Props = $props();
</script>

<aside class="catalogue" aria-label="Blocks">
	{#each GROUPS as group (group.id)}
		<section aria-label={group.label}>
			<h3 class="group">
				<span class="square" style:background-color={group.tone} aria-hidden="true"></span>
				{group.label}
			</h3>
			<div class="blocks">
				{#each CATALOGUE.filter((block) => block.group === group.id) as block (block.id)}
					<button class="block" title="{block.what} ({block.key})" onclick={() => onadd(block.id)}>
						<span class="square" style:background-color={group.tone} aria-hidden="true"></span>
						<span>{block.label}</span>
						<kbd class="mono">{block.key}</kbd>
					</button>
				{/each}
			</div>
		</section>
	{/each}
</aside>

<style>
	.catalogue {
		flex: 0 0 244px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: auto;
	}

	.group {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0 0 6px;
		font-size: var(--fs-secondary);
		font-weight: 400;
		color: var(--muted);
	}

	.square {
		flex: none;
		width: 9px;
		height: 9px;
		border-radius: 2px;
		opacity: 0.75;
	}

	.blocks {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.block {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 5px 8px 5px 9px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
		background: var(--surface-2);
		white-space: nowrap;
	}

	.block:hover {
		background: var(--hover);
	}

	kbd {
		min-width: 18px;
		padding: 1px 4px;
		border-radius: 4px;
		background: var(--sunken);
		color: var(--muted);
		font-size: var(--fs-mono);
		text-align: center;
	}
</style>
