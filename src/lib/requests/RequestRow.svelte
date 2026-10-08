<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import { CHECK_LABELS, REVIEW_LABELS, requests } from '$lib/requests/store.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import type { PullRequest } from '$lib/types';

	/**
	 * One pull request.
	 *
	 * The vocabulary is host-agnostic throughout: "pull request", never a
	 * brand. The hosting service is a detail, not the language.
	 */
	interface Props {
		request: PullRequest;
		/** Rows waiting on somebody else render dashed. */
		waiting?: boolean;
	}

	let { request, waiting = false }: Props = $props();

	const open = $derived(request.id === requests.openId);
</script>

<li class="row" class:waiting class:open>
	<button class="body" onclick={() => requests.openWorkspace(request.id)}>
		<span class="number mono note">#{request.number}</span>
		<span class="title" title={request.title}>{request.title}</span>

		<span class="chips">
			{#if request.draft}<Chip>draft</Chip>{/if}
			<Chip active={request.review === 'changesRequested'}>
				{REVIEW_LABELS[request.review]}
			</Chip>
			{#if request.checks}
				<Chip active={request.checks === 'failing'}>{CHECK_LABELS[request.checks]}</Chip>
			{/if}
			{#if request.mergeable === false}<Chip active>conflicts</Chip>{/if}
		</span>

		<span class="who note">{request.authorName}</span>
		<span class="when note">{relativeTime(request.updated)}</span>
	</button>

	{#if request.needsYou && request.needsYouBecause}
		<div class="because note">{request.needsYouBecause}</div>
	{/if}
</li>

<style>
	.row {
		list-style: none;
		background-color: var(--surface-veil);
		border: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		border-radius: var(--r-panel);
		box-shadow: none;
		padding: 5px 9px;
		transition:
			background-color var(--t-fast) var(--ease),
			border-color var(--t-fast) var(--ease);
	}

	.row:hover {
		background-color: var(--hover);
	}

	.row.waiting {
		background: none;
		box-shadow: none;
		color: var(--muted);
	}

	.row.waiting:hover {
		background-color: var(--hover);
	}

	/* The one being read. */
	.row.open {
		background: var(--selection);
		border-color: color-mix(in srgb, var(--accent) 45%, var(--soft));
		box-shadow: none;
	}

	.body {
		width: 100%;
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		padding: 0;
	}

	.number {
		flex: none;
	}

	.title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.chips {
		flex: none;
		display: flex;
		gap: 4px;
	}

	.who,
	.when {
		flex: none;
		white-space: nowrap;
	}

	.because {
		margin-top: 2px;
	}
</style>
