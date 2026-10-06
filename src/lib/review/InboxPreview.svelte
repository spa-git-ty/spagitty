<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import PRMarkdown from '$lib/requests/PRMarkdown.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import type { PullRequest } from '$lib/types';
	import { continueLabel, factsOf, progressOf } from './inbox';
	import type { ReviewRecord } from './record';

	/**
	 * The chosen pull request, before it is opened (FEAT-087): what it says
	 * about itself, and what is worth knowing first.
	 */
	interface Props {
		pr: PullRequest;
		record: ReviewRecord | null;
		/** Set when the row belongs to a repository with no clone here. */
		notHere: string | null;
		opening: boolean;
		onopen: () => void;
		/** Absent where the pull request cannot be checked out here. */
		onworktree?: () => void;
	}

	let { pr, record, notHere, opening, onopen, onworktree }: Props = $props();

	const facts = $derived(factsOf(pr, record));
	const progress = $derived(progressOf(pr, record));
</script>

<aside class="preview" aria-label="Pull request preview">
	<span class="where note mono">
		#{pr.number} · {pr.sourceBranch} → {pr.targetBranch}
	</span>
	<span class="title">{pr.title}</span>

	{#if pr.body.trim()}
		<div class="body"><PRMarkdown markdown={pr.body} /></div>
	{/if}

	{#if facts.length > 0}
		<div class="facts">
			<span class="note">Before you start</span>
			{#each facts as fact (fact.text)}
				<span class="fact"><span class="dot {fact.tone}"></span>{fact.text}</span>
			{/each}
		</div>
	{/if}

	<span class="grow"></span>

	{#if notHere}
		<span class="note">No clone of {notHere} here</span>
	{/if}
	<Btn primary quiet disabled={opening} onclick={onopen}>{continueLabel(progress)}</Btn>
	{#if onworktree}
		<Btn disabled={opening} onclick={onworktree}>Open in worktree</Btn>
	{/if}
</aside>

<style>
	/* The inset card, as on Graph: inside the pane, its own corner and edge. */
	.preview {
		width: 330px;
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 16px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		box-shadow: var(--ornament-shadow);
		overflow: auto;
		min-height: 0;
	}

	.title {
		font-family: var(--read-font);
		font-size: calc(var(--fs-title) * 0.94);
		line-height: 1.4;
	}

	.body {
		font-family: var(--read-font);
		line-height: 1.7;
		letter-spacing: var(--code-ls);
		max-height: 40%;
		overflow: auto;
	}

	.facts {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.fact {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex: none;
		background: var(--muted);
	}

	.dot.resolve {
		background: var(--lane-5);
	}

	.dot.accent {
		background: var(--accent);
	}

	.dot.ok {
		background: var(--ok);
	}

	.dot.danger {
		background: var(--danger);
	}

	.dot.warn {
		background: var(--warn);
	}

	.grow {
		flex: 1;
	}

	.preview :global(.btn) {
		justify-content: center;
		padding: 8px 14px;
	}
</style>
