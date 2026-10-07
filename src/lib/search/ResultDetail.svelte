<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { clockTime, relativeTime } from '$lib/format';
	import { search } from '$lib/search/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';

	/**
	 * The opened commit, beside the results rather than instead of them.
	 *
	 * "Opening a commit" here means reading it — message, people, and the files
	 * it touched. Its hunks are a different question and a different screen,
	 * which is what `Alt+Enter` is for.
	 */
	interface Props {
		ondiff?: (id: string) => void;
	}

	let { ondiff }: Props = $props();

	const detail = $derived(search.detail);
</script>

<section class="detail">
	{#if search.detailError}
		<p class="note error state">{search.detailError}</p>
	{:else if !detail}
		<p class="note state">Open a result to read it.</p>
	{:else}
		<header class="top">
			<span class="sha mono">{detail.short}</span>
			<Btn onclick={() => ondiff?.(detail.id)}>Open full diff<Icon name="chevron-right" size="0.95em" weight={2.2} /></Btn>
		</header>

		<h2 class="summary">{detail.summary}</h2>
		{#if detail.body}<p class="body note">{detail.body}</p>{/if}

		<div class="person">
			<AuthorAvatar email={detail.authorEmail} name={detail.authorName} />
			<span class="who">
				<span class="name">{detail.authorName}</span>
				<span class="note small">{detail.authorEmail}</span>
			</span>
			<span class="when note small" title={clockTime(detail.authorTime)}>{relativeTime(detail.authorTime)}</span>
		</div>

		<div class="files note small">
			{detail.files.length}
			{detail.files.length === 1 ? 'file' : 'files'}
		</div>
		<ul class="filelist mono">
			{#each detail.files as file (file.path)}
				<li title={file.path}>{file.path}</li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	.detail {
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.state {
		margin: 0;
	}

	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}

	.sha {
		font-size: var(--fs-mono);
		color: var(--muted);
		padding: 2px 8px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
	}

	.summary {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: 600;
		line-height: 1.4;
	}

	.body {
		margin: 0;
		white-space: pre-wrap;
		line-height: 1.5;
	}

	.person {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: 12px;
		background: color-mix(in srgb, var(--sunken) 60%, transparent);
		min-width: 0;
	}

	.who {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.who > span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.name {
		font-weight: 550;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.when {
		flex: none;
	}

	.files {
		margin-top: 2px;
	}

	.filelist {
		margin: 0;
		padding: 0;
		list-style: none;
		font-size: var(--fs-secondary);
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	/* The tail of a path identifies the file, so the head takes the ellipsis —
	   the same rule every other file list here follows. */
	.filelist li {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		direction: rtl;
		text-align: left;
		padding: 2px 6px;
		border-radius: 6px;
	}

	.filelist li:hover {
		background: var(--hover);
	}
</style>
