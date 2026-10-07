<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import { ACTION_LOOK, orphanSquash } from './plan';
	import { ACTIONS, rebase } from './store.svelte';
	import type { RebaseAction } from '$lib/types';

	/**
	 * The plan (FEAT-106): every commit that moves, in the order it is
	 * replayed, each with what happens to it. Rows move by drag or by
	 * Alt+↑ / Alt+↓; a reword takes its new message here, because git's editor
	 * has no terminal to open in.
	 */
	interface Props {
		/** Nothing can be edited while git is replaying or stopped. */
		locked?: boolean;
	}

	let { locked = false }: Props = $props();

	let dragging = $state<string | null>(null);
	let over = $state<string | null>(null);

	const risky = $derived(new Set(rebase.preview?.rows.filter((r) => r.mayConflict).map((r) => r.id) ?? []));

	function keydown(id: string, event: KeyboardEvent) {
		if (locked || !(event.altKey || event.metaKey)) return;
		if (event.key === 'ArrowUp') {
			event.preventDefault();
			void rebase.move(id, -1);
		} else if (event.key === 'ArrowDown') {
			event.preventDefault();
			void rebase.move(id, 1);
		}
	}

	function drop(target: string) {
		const moving = dragging;
		dragging = null;
		over = null;
		if (locked || moving === null || moving === target) return;
		const from = rebase.plan.findIndex((entry) => entry.id === moving);
		const to = rebase.plan.findIndex((entry) => entry.id === target);
		if (from !== -1 && to !== -1) void rebase.move(moving, to - from);
	}

	function choose(id: string, action: RebaseAction) {
		if (!locked) void rebase.setAction(id, action);
	}
</script>

<ol class="plan" aria-label="Plan">
	{#each rebase.rows as { row, action }, index (row.id)}
		{@const look = ACTION_LOOK[action]}
		{@const orphan = action === 'squash' && orphanSquash(rebase.plan, row.id)}
		<li
			class="row tone-{look.tone}"
			class:dropped={action === 'drop'}
			class:focused={row.id === rebase.focused}
			class:over={over === row.id && dragging !== row.id}
			class:dragging={dragging === row.id}
			draggable={!locked}
			ondragstart={() => (dragging = row.id)}
			ondragover={(event) => {
				event.preventDefault();
				over = row.id;
			}}
			ondragleave={() => {
				if (over === row.id) over = null;
			}}
			ondrop={() => drop(row.id)}
			ondragend={() => {
				dragging = null;
				over = null;
			}}
		>
			<span class="grip" aria-hidden="true" title="Drag to reorder, or Alt+↑ / Alt+↓">⠿</span>
			<span class="step" aria-hidden="true">{index + 1}</span>
			<button
				class="what"
				onclick={() => rebase.focus(row.id)}
				onkeydown={(event) => keydown(row.id, event)}
				title={row.summary}
			>
				<span class="line">
					<span class="sha mono">{row.short}</span>
					<span class="summary">{row.summary}</span>
				</span>
				<span class="meta">
					<span class="note">{row.authorName}</span>
					{#if action === 'squash'}
						<span class="tag tone-squash" class:bad={orphan}>
							{orphan ? 'nothing above to fold into' : 'folds into the one above'}
						</span>
					{/if}
					{#if risky.has(row.id) && action !== 'drop'}
						<span class="tag risky">may conflict</span>
					{/if}
				</span>
			</button>
			<span class="actions" role="group" aria-label="What happens to {row.short}">
				{#each ACTIONS as candidate (candidate)}
					{@const option = ACTION_LOOK[candidate]}
					<button
						class="action tone-{option.tone}"
						aria-pressed={candidate === action}
						title={option.hint}
						disabled={locked}
						onclick={() => choose(row.id, candidate)}
					>
						{option.label}
					</button>
				{/each}
			</span>
			{#if action === 'reword'}
				<label class="message">
					<Icon name="edit" size="0.95em" />
					<input
						class="reword"
						value={rebase.messageOf(row.id)}
						placeholder={row.summary}
						aria-label="New message for {row.short}"
						spellcheck="true"
						disabled={locked}
						oninput={(event) => rebase.setMessage(row.id, event.currentTarget.value)}
					/>
				</label>
			{/if}
		</li>
	{/each}
</ol>

<style>
	.plan {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.row {
		--tone: var(--ok);
		display: grid;
		grid-template-columns: auto auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 4px 10px;
		padding: 8px 10px 8px 6px;
		border-radius: 14px;
		border: 1px solid var(--pane-edge);
		border-left: 3px solid var(--tone);
		background: color-mix(in srgb, var(--surface-2) 70%, transparent);
	}

	.row.focused {
		border-color: color-mix(in srgb, var(--accent) 55%, transparent);
		border-left-color: var(--tone);
	}

	.row.over {
		box-shadow: 0 -2px 0 var(--accent);
	}

	.row.dragging {
		opacity: 0.5;
	}

	.row.dropped .summary,
	.row.dropped .sha {
		text-decoration: line-through;
		color: var(--muted);
	}

	.tone-pick {
		--tone: var(--ok);
	}

	.tone-reword {
		--tone: var(--lane-4);
	}

	.tone-squash {
		--tone: var(--side-b);
	}

	.tone-drop {
		--tone: var(--danger);
	}

	.grip {
		color: var(--muted);
		cursor: grab;
		padding: 0 2px;
	}

	.step {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		font-size: var(--fs-mono);
		font-weight: 650;
		color: var(--tone);
		background: color-mix(in srgb, var(--tone) 16%, transparent);
	}

	.what {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		text-align: left;
		padding: 0;
	}

	.line,
	.meta {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.meta {
		flex-wrap: wrap;
	}

	.sha {
		color: var(--muted);
		font-size: var(--fs-mono);
		flex: none;
	}

	.summary {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 550;
	}

	.tag {
		font-size: var(--fs-mono);
		padding: 1px 8px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--tone) 45%, transparent);
		color: var(--tone);
		white-space: nowrap;
	}

	.tag.bad,
	.tag.risky {
		--tone: var(--danger);
	}

	.actions {
		display: flex;
		padding: 2px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
		border: 1px solid var(--pane-edge);
	}

	.action {
		height: 24px;
		padding: 0 10px;
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		color: var(--muted);
		white-space: nowrap;
	}

	.action:hover:not(:disabled) {
		background: var(--hover);
		color: var(--ink);
	}

	.action[aria-pressed='true'] {
		background: color-mix(in srgb, var(--tone) 18%, var(--surface-2));
		color: var(--tone);
		font-weight: 650;
		box-shadow: var(--shadow-1);
	}

	.action:disabled {
		cursor: not-allowed;
	}

	.message {
		grid-column: 3 / -1;
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--lane-4);
	}

	.reword {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-secondary);
		border: 1px solid color-mix(in srgb, var(--lane-4) 45%, var(--pane-edge));
		background: var(--sunken);
		color: var(--ink);
		border-radius: 8px;
		padding: 5px 10px;
	}

	.reword:focus {
		outline: none;
		border-color: var(--lane-4);
	}
</style>
