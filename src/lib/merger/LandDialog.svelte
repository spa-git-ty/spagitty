<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { relativeTime } from '$lib/format';
	import { plural, type Strategy, type SummaryRow } from './plan';
	import SideBadge from './SideBadge.svelte';
	import { merger } from './store.svelte';

	/**
	 * The commit dialog (FEAT-101) and, once it has landed, the done state.
	 *
	 * The dialog is the one place Merger writes from: it lists every conflict
	 * with what was chosen for it, takes the message, and names the strategy on
	 * its button. A rebase has no message box — each replayed commit keeps its
	 * own — and a fast-forward writes no commit at all.
	 */
	interface Props {
		/** Each conflict and what was chosen, or none for a clean merge. */
		rows?: SummaryRow[];
		/** Write it. */
		oncommit: () => void;
		/** Open the graph, from the done state. */
		ongraph: () => void;
	}

	let { rows = [], oncommit, ongraph }: Props = $props();

	const who = $derived(merger.roles);
	const strategy = $derived(merger.strategy);
	const tone = $derived(who ? (who.isNew ? 'new' : who.onto) : 'a');
	const done = $derived(merger.phase === 'done' ? merger.landed : null);

	const LABELS: Record<Strategy, string> = {
		merge: 'Create merge commit',
		squash: 'Commit the squash',
		rebase: 'Finish the rebase',
		ff: 'Fast-forward'
	};

	let dialog = $state<HTMLDivElement | null>(null);

	onMount(() => {
		const first = dialog?.querySelector<HTMLElement>('textarea, button.primary-action, .btn.primary');
		first?.focus();
	});

	function keydown(event: KeyboardEvent) {
		if (event.key !== 'Escape') return;
		event.stopPropagation();
		if (merger.phase === 'commit') merger.back();
	}

	function doneText(): string {
		if (!done) return '';
		const { landed, source } = done;
		switch (done.strategy) {
			case 'merge':
				return `One merge commit, ${landed.short}. ${source} is unchanged; delete it from Branches when you no longer need it.`;
			case 'squash':
				return `One squashed commit, ${landed.short}. ${source}’s own commits are still on that branch.`;
			case 'rebase':
				return `${plural(landed.written, 'commit')} replayed with new hashes, ending at ${landed.short}. ${source} still has the originals.`;
			case 'ff':
				return `${landed.target} moved forward to ${landed.short}. No new commit.`;
		}
	}
</script>

<div class="veil">
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		bind:this={dialog}
		class="dialog ornament"
		role="dialog"
		aria-modal="true"
		aria-label={done ? 'Merged' : 'Land the merge'}
		tabindex="-1"
		onkeydown={keydown}
	>
		{#if done}
			<span class="tick" aria-hidden="true"><Icon name="check" size="1.4em" weight={2.4} /></span>
			<h2 class="title">
				<span class="mono tone-{tone}">{done.landed.target}</span> now includes {done.source}
			</h2>
			<span class="note">{doneText()}</span>
			<div class="buttons start">
				<Btn primary onclick={ongraph}>Open in Graph</Btn>
				<Btn onclick={() => merger.again()}>Merge another</Btn>
			</div>
		{:else if who}
			<h2 class="title">Ready to land on <span class="mono tone-{tone}">{who.targetName}</span></h2>
			{#if rows.length > 0}
				<ul class="rows">
					{#each rows as row (row.where)}
						<li>
							<span class="where mono">{row.where}</span>
							<span class="chip" class:mine={row.mine}>
								{#each row.badges as badge, index (index)}<SideBadge side={badge} small />{/each}
								{row.label}
							</span>
							{#if row.who && row.who !== 'you'}<span class="note">{row.who}</span>{/if}
						</li>
					{/each}
				</ul>
			{:else if merger.forecast}
				{@const source = merger.forecast[who.source]}
				<span class="note">
					No conflicts. {plural(source.ahead, 'commit')} from {who.sourceName}, the newest {relativeTime(source.time)}.
				</span>
			{/if}
			{#if strategy === 'merge' || strategy === 'squash'}
				<div class="message">
					<label for="merger-message" class="note">Commit message</label>
					<textarea
						id="merger-message"
						class="mono"
						rows="3"
						value={merger.message}
						spellcheck="false"
						oninput={(event) => merger.setMessage((event.currentTarget as HTMLTextAreaElement).value)}
					></textarea>
				</div>
			{:else if strategy === 'rebase'}
				<span class="note">Each replayed commit keeps its own message.</span>
			{:else}
				<span class="note">{who.targetName} moves forward to {who.sourceName}. No commit is written.</span>
			{/if}
			{#if merger.landError}<p class="note error" role="alert">{merger.landError}</p>{/if}
			<div class="buttons">
				<Btn disabled={merger.landing} onclick={() => merger.back()}>Back</Btn>
				<Btn primary busy={merger.landing} onclick={oncommit}>
					{merger.landing ? 'Writing…' : strategy === 'ff' ? `${LABELS.ff} ${who.targetName}` : LABELS[strategy]}
				</Btn>
			</div>
		{/if}
	</div>
</div>

<style>
	.veil {
		position: absolute;
		inset: 0;
		z-index: 5;
		display: grid;
		place-items: center;
		background: color-mix(in srgb, var(--umbra) 38%, transparent);
	}

	.dialog {
		width: 540px;
		max-width: calc(100% - 32px);
		max-height: calc(100% - 32px);
		overflow: auto;
		border-radius: 24px;
		padding: 22px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		outline: none;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.title .mono {
		font-size: inherit;
	}

	.tone-a {
		color: var(--side-a);
	}

	.tone-b {
		color: var(--side-b);
	}

	.tone-new {
		color: var(--ok);
	}

	.tick {
		width: 44px;
		height: 44px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		background: color-mix(in srgb, var(--ok) 12%, transparent);
		color: var(--ok);
	}

	.rows {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.rows li {
		display: flex;
		align-items: center;
		gap: 10px;
		font-size: var(--fs-secondary);
	}

	.where {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 3px 10px;
		font-size: var(--fs-mono);
		white-space: nowrap;
		background-color: var(--surface-veil);
	}

	.chip.mine {
		color: var(--side-mine);
	}

	.message {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	textarea {
		font-size: var(--fs-secondary);
		border: 1px solid var(--pane-edge);
		background: var(--sunken);
		color: var(--ink);
		border-radius: 10px;
		padding: 8px 10px;
		resize: vertical;
	}

	.buttons {
		display: flex;
		gap: 8px;
		justify-content: flex-end;
	}

	.buttons.start {
		justify-content: flex-start;
	}

	.error {
		color: var(--danger);
		margin: 0;
	}
</style>
