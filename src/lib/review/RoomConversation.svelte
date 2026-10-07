<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { markdownText } from '$lib/ui/markdown';
	import { room } from './room.svelte';
	import { whereOfDraft } from './drafts';
	import { whereOf } from './threads';

	/**
	 * The Conversation card (FEAT-091): every thread on the pull request,
	 * open or resolved, each one a way back to the line it is about.
	 *
	 * Under them, what you have written and not sent (FEAT-093) — each a way
	 * to its line too — and at the foot the box for the pull request as a
	 * whole, which goes out with Finish review.
	 */

	interface Props {
		/** Put the card away; the room offers it back at its edge (BUG-054). */
		onhide?: () => void;
	}

	let { onhide }: Props = $props();

	const open = $derived(room.threads.filter((thread) => !thread.resolved));
	const resolved = $derived(room.threads.filter((thread) => thread.resolved));
	const shown = $derived(room.panel === 'open' ? open : resolved);

	function replies(count: number): string {
		return count === 0 ? 'no replies' : count === 1 ? '1 reply' : `${count} replies`;
	}
</script>

<aside class="conversation" aria-label="Conversation">
	<div class="head">
		<span class="title">Conversation</span>
		<Chip active={room.panel === 'open'} onclick={() => room.setPanel('open')}>Open {open.length}</Chip>
		<Chip active={room.panel === 'resolved'} onclick={() => room.setPanel('resolved')}>
			Resolved {resolved.length}
		</Chip>
		{#if onhide}
			<button class="hide" onclick={onhide} title="Hide the conversation" aria-label="Hide the conversation">
				<Icon name="chevron-right" size="0.95em" weight={2.2} />
			</button>
		{/if}
	</div>
	<div class="list">
		{#if room.commentsError}
			<p class="note error" title={room.commentsError}>The threads could not be read.</p>
		{:else if shown.length === 0}
			<p class="note">{room.panel === 'open' ? 'No open threads.' : 'No resolved threads.'}</p>
		{/if}
		{#each shown as thread (thread.id)}
			{@const first = thread.comments[0]}
			<button class="card item" disabled={thread.line === null} onclick={() => room.jumpTo(thread)}>
				<span class="top">
					<Chip><span class="mono">{whereOf(thread)}</span></Chip>
					<span class="grow"></span>
					<span class="note small">{replies(thread.comments.length - 1)}</span>
				</span>
				<span class="note"><span class="who">{first.author}</span> · {relativeTime(first.createdAt)}</span>
				<span class="body">{markdownText(first.body)}</span>
			</button>
		{/each}
		{#if room.currentDrafts.length > 0}
			<span class="note mine">Your pending · {room.currentDrafts.length}</span>
			{#each room.currentDrafts as draft (draft.id)}
				<button class="draft" onclick={() => room.jumpToDraft(draft)}>
					<Chip><span class="mono">{whereOfDraft(draft)}</span></Chip>
					<span class="body">{markdownText(draft.body)}</span>
				</button>
			{/each}
		{/if}
		{#if room.olderDrafts.length > 0}
			<span class="note mine">Written before the last push · {room.olderDrafts.length}</span>
			{#each room.olderDrafts as draft (draft.id)}
				<div class="draft older">
					<span class="top">
						<Chip><span class="mono">{whereOfDraft(draft)}</span></Chip>
						<span class="grow"></span>
						<button class="note delete" onclick={() => room.removeDraft(draft.id)}>Delete</button>
					</span>
					<span class="body">{markdownText(draft.body)}</span>
				</div>
			{/each}
		{/if}
	</div>
	<div class="whole">
		<label for="pr-comment" class="note">On the whole pull request</label>
		<textarea
			id="pr-comment"
			rows="2"
			placeholder="Overall thoughts…"
			value={room.body}
			oninput={(event) => room.setBody(event.currentTarget.value)}
		></textarea>
	</div>
</aside>

<style>
	/* An inset card, like the commit detail on Graph. */
	.conversation {
		width: var(--room-conversation-w);
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
		margin: 0 0 10px;
		padding: 12px 10px 0;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		min-width: 0;
		overflow: hidden;
	}

	/* Wraps rather than spilling the chips past the card's edge when the card
	   is narrow (BUG-054). */
	.head {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
		padding: 4px 4px 10px;
		min-width: 0;
	}

	.title {
		flex: 1 1 auto;
		min-width: 0;
		font-size: var(--fs-ui);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.hide {
		width: 24px;
		height: 24px;
		border-radius: 8px;
		display: grid;
		place-items: center;
		color: var(--muted);
		flex: none;
	}

	.hide:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.list {
		flex: 1;
		overflow: auto;
		padding: 0 4px 12px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.list > p {
		margin: 4px;
	}

	.error {
		color: var(--danger);
	}

	.item {
		text-align: left;
		padding: 11px 12px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.mine {
		padding: 8px 4px 0;
		color: var(--warn);
	}

	/* Written here and not sent: warm, as under its line. */
	.draft {
		text-align: left;
		padding: 11px 12px;
		border-radius: var(--r-panel);
		background: var(--warn-soft);
		border: 1px solid color-mix(in srgb, var(--warn) 35%, transparent);
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 6px;
	}

	.draft.older {
		align-items: stretch;
		opacity: 0.8;
	}

	.delete {
		text-decoration: underline;
	}

	.whole {
		padding: 8px 4px 14px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.whole textarea {
		border: 1px solid var(--pane-edge);
		background: var(--sunken);
		border-radius: var(--r-field);
		padding: 8px 10px;
		font-family: var(--read-font);
		font-size: var(--fs-secondary);
		resize: vertical;
	}

	.top {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.grow {
		flex: 1;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.who {
		color: var(--ink);
		font-weight: 600;
	}

	.body {
		font-family: var(--read-font);
		font-size: var(--fs-secondary);
		line-height: 1.6;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
		overflow-wrap: anywhere;
	}
</style>
