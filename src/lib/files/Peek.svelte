<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { fullDate, relativeTime } from '$lib/format';
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';
	import { graph } from '$lib/graph/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { files, pullOf } from './store.svelte';

	/**
	 * Why a line is the way it is (FEAT-131).
	 *
	 * Opened under the line, in the flow of the file, never over it: the code
	 * around the line is what the answer is about, and a card covering it
	 * would hide the question.
	 *
	 * It says who and when, what the commit was for, and what the line was
	 * before — the line that commit's change took away in this place. Then
	 * three ways on: the commit on the Graph, the blame as it stood before
	 * this commit, and the line's whole life in Investigate.
	 */
	interface Props {
		/** The line peeked at, 0-based. */
		line: number;
	}

	let { line }: Props = $props();

	const commit = $derived(files.commitOf(line));
	const pull = $derived(commit ? pullOf(commit.summary) : null);
	const refused = $derived(files.blame?.refused ?? null);

	const REFUSED = {
		binary: 'A binary file has no lines to blame',
		tooLarge: 'This file is too large to blame',
		notAFile: 'There is no file here to blame',
		empty: 'An empty file has no lines to blame'
	} as const;

	function showCommit() {
		if (!commit) return;
		graph.want(commit.id);
		void goto('/');
	}

	function investigate() {
		const file = files.file;
		if (!file) return;
		const query = new URLSearchParams({
			kind: 'line',
			path: file.path,
			line: String(line + 1),
			rev: file.revision
		});
		void goto(`/investigate?${query}`);
	}
</script>

<div class="peek" class:agent={!!commit?.agent} role="group" aria-label="Line {line + 1}">
	{#if commit}
		<div class="who">
			{#if commit.agent}
				<span class="mark"><Icon name="agent" size={14} weight={2.2} /></span>
				<span class="by">{commit.agent}</span>
				<span class="note" title={fullDate(commit.time)}>
					for {commit.authorName} · {relativeTime(commit.time)}{#if pull}&nbsp;· #{pull}{/if}
				</span>
			{:else}
				<AuthorAvatar email={commit.authorEmail} name={commit.authorName} />
				<span class="by">{commit.authorName}</span>
				<span class="note" title={fullDate(commit.time)}>
					{relativeTime(commit.time)}{#if pull}&nbsp;· #{pull}{/if}
				</span>
			{/if}
			<span class="gap"></span>
			<span class="mono muted">{commit.short}</span>
		</div>

		<span class="summary">{commit.summary}</span>

		{#if files.was?.trim()}
			<div class="was mono" title="This line before {commit.short}">
				<span>was</span>
				<s>{files.was.trim()}</s>
			</div>
		{/if}

		<div class="actions">
			<Btn onclick={showCommit} title="Show {commit.short} on the Graph">Commit</Btn>
			<Btn
				disabled={!commit.previous}
				title={commit.previous
					? `Blame this file as it was before ${commit.short}`
					: `${commit.short} added this file`}
				onclick={() => files.blameBefore(line)}>Blame before this</Btn
			>
			<Btn onclick={investigate} title="Every commit that changed this line">Investigate line</Btn>
		</div>
	{:else if files.blameError}
		<span class="note error">{files.blameError}</span>
	{:else if refused}
		<span class="note">{REFUSED[refused]}</span>
	{:else}
		<Loader size="inline" label="Reading blame…" />
	{/if}
</div>

<style>
	.peek {
		--avatar-d: 18px;
		margin: 6px 16px 8px;
		padding: 14px 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		background: var(--surface);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		line-height: 1.4;
		white-space: normal;
	}

	/* An agent's line: the agent's own colour, as everywhere since 2.0. */
	.peek.agent {
		border-color: var(--agent-edge);
		background: color-mix(in srgb, var(--agent) 8%, var(--surface));
	}

	.who {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}

	.by {
		font-weight: 600;
	}

	.agent .mark,
	.agent .by {
		color: var(--agent);
	}

	.mark {
		display: inline-flex;
	}

	.gap {
		flex: 1;
	}

	.summary {
		font-size: var(--fs-ui);
	}

	.was {
		display: flex;
		gap: 12px;
		padding: 6px 10px;
		border-radius: var(--r-field);
		background: var(--sunken);
		color: var(--muted);
		white-space: pre;
		overflow: hidden;
	}

	.was s {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.error {
		color: var(--danger);
	}
</style>
