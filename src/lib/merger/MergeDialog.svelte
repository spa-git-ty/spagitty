<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
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
	 *
	 * Done, a merge somebody chose here says where it left them — the branch
	 * that received it is checked out — and asks the one thing that is left:
	 * push it now, or not now (FEAT-123). *Not now* rather than *Cancel*: the
	 * merge is made, and nothing here takes it back. Where the push goes is the
	 * button's title; a branch that was already checked out is not remarked on.
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
		aria-label={done ? 'Merged' : 'Make the merge'}
		tabindex="-1"
		onkeydown={keydown}
	>
		{#if done}
			<span class="tick" aria-hidden="true"><Icon name="check" size="1.4em" weight={2.4} /></span>
			<h2 class="title">
				<span class="mono tone-{tone}">{done.landed.target}</span> now includes {done.source}
			</h2>
			<span class="note">{doneText()}</span>
			{#if merger.origin?.kind === 'farmTask'}
				{#if merger.farmError}
					<p class="note error" role="alert">{merger.farmError}</p>
					<div class="buttons start">
						<Btn primary onclick={() => merger.finishFarmTask()}>Tell the Farm again</Btn>
					</div>
				{:else}
					<span class="note">The Farm has been told, and the task is finished.</span>
				{/if}
			{/if}
			{#if merger.origin?.kind === 'pullRequest' && merger.origin.local}
				<span class="note">
					Push it to update the pull request, then go back to merge the pull request.
				</span>
				{#if merger.pushError}<p class="note error" role="alert">{merger.pushError}</p>{/if}
				<div class="buttons start">
					<Btn
						primary={!merger.pushed}
						busy={merger.pushing}
						disabled={merger.pushed}
						onclick={() => merger.pushPullRequestHead()}
					>
						{merger.pushed ? 'Pushed' : `Push ${merger.origin.local.branch}`}
					</Btn>
					<Btn primary={merger.pushed} onclick={() => merger.backToPullRequest()}>
						Back to the pull request
					</Btn>
				</div>
			{:else if merger.origin === null}
				{@const target = done.landed.target}
				{@const out = merger.checkout}
				{#if out?.state === 'switching'}
					<span class="note line"><Loader size="inline" label="Working" />Checking out {target}…</span>
				{:else if out?.state === 'done'}
					<span class="note"><span class="mono">{target}</span> is checked out.</span>
				{:else if out?.state === 'elsewhere'}
					<span class="note" title="A branch can be checked out in one worktree at a time. It is up to date there.">
						<span class="mono">{target}</span> is checked out in <span class="mono">{out.path}</span>, not here.
					</span>
				{:else if out?.state === 'failed'}
					<p class="note error" role="alert">
						Merged, but {target} could not be checked out: {out.error}
					</p>
				{/if}
				{#if merger.pushed && merger.pushTo}
					<span class="note">Pushed to <span class="mono">{merger.pushTo.to}</span>.</span>
				{:else if merger.nowhereToPush}
					<span class="note">No remote to push to.</span>
				{/if}
				{#if merger.pushError}<p class="note error" role="alert">{merger.pushError}</p>{/if}
				<div class="buttons start">
					{#if (merger.pushTo || merger.pushReading) && !merger.pushed}
						<Btn
							primary
							busy={merger.pushing}
							disabled={!merger.pushTo}
							title={merger.pushTo ? `Push ${target} to ${merger.pushTo.to}` : undefined}
							onclick={() => merger.pushLanded()}
						>
							{merger.pushing ? 'Pushing…' : 'Push now'}
						</Btn>
						<Btn disabled={merger.pushing} onclick={() => merger.again()}>Not now</Btn>
						{#if out?.state === 'failed'}
							<Btn onclick={() => merger.checkOutLanded()}>Check out {target}</Btn>
						{/if}
						<span class="grow"></span>
						<Btn onclick={ongraph}>Open in Graph</Btn>
					{:else}
						<Btn primary onclick={ongraph}>Open in Graph</Btn>
						{#if out?.state === 'failed'}
							<Btn onclick={() => merger.checkOutLanded()}>Check out {target}</Btn>
						{/if}
						<Btn onclick={() => merger.again()}>Merge another</Btn>
					{/if}
				</div>
			{:else}
				<div class="buttons start">
					<Btn primary onclick={ongraph}>Open in Graph</Btn>
					<Btn onclick={() => merger.again()}>Merge another</Btn>
				</div>
			{/if}
		{:else if who}
			<h2 class="title">Ready to merge into <span class="mono tone-{tone}">{who.targetName}</span></h2>
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
		flex-wrap: wrap;
	}

	.grow {
		flex: 1;
	}

	.line {
		display: inline-flex;
		align-items: center;
		gap: 7px;
	}

	.error {
		color: var(--danger);
		margin: 0;
	}
</style>
