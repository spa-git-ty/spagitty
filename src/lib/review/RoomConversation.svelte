<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import Chip from '$lib/ui/Chip.svelte';
	import { room } from './room.svelte';
	import { whereOf } from './threads';

	/**
	 * The Conversation card (FEAT-091): every thread on the pull request,
	 * open or resolved, each one a way back to the line it is about.
	 */

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
				<span class="body">{first.body}</span>
			</button>
		{/each}
	</div>
</aside>

<style>
	/* An inset card, like the commit detail on Graph. */
	.conversation {
		width: 300px;
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
	}

	.head {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 4px 10px;
	}

	.title {
		flex: 1;
		font-size: var(--fs-ui);
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
