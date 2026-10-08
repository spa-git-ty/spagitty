<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Markdown from '$lib/ui/Markdown.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import type { PullRequest } from '$lib/types';
	import { continueLabel, factsOf, progressOf } from './inbox';
	import type { ReviewRecord } from './record';
	import Icon from '$lib/ui/Icon.svelte';
	import AssignPopover from '$lib/agents/AssignPopover.svelte';
	import type { Assigned } from '$lib/agents/types';

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
		oncheckout?: () => void;
		/**
		 * An agent can be assigned here (2.0): one is set up for reviews, the
		 * pull request is this repository's, and none is working on it. False
		 * with no agent set up, and then nothing about agents is drawn.
		 */
		assignable?: boolean;
		starting?: boolean;
		onassign?: (chosen: Assigned) => Promise<boolean>;
	}

	let {
		pr,
		record,
		notHere,
		opening,
		onopen,
		oncheckout,
		assignable = false,
		starting = false,
		onassign
	}: Props = $props();

	let assigning = $state(false);

	async function assign(chosen: Assigned) {
		if (await onassign?.(chosen)) assigning = false;
	}

	const facts = $derived(factsOf(pr, record));
	const progress = $derived(progressOf(pr, record));
</script>

<aside class="preview" aria-label="Pull request preview">
	<span class="where note mono">
		#{pr.number} · {pr.sourceBranch} → {pr.targetBranch}
	</span>
	<span class="title">{pr.title}</span>

	{#if pr.body.trim()}
		<div class="body"><Markdown source={pr.body} /></div>
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
	{#if assignable && onassign}
		<span class="assign-anchor">
			<Btn disabled={opening} onclick={() => (assigning = !assigning)}>
				<Icon name="agent" size="1em" />Assign an agent…
			</Btn>
			{#if assigning}
				<AssignPopover job="review" busy={starting} onassign={assign} oncancel={() => (assigning = false)} />
			{/if}
		</span>
	{/if}
	{#if oncheckout}
		<Btn disabled={opening} onclick={oncheckout}>Check out branch</Btn>
	{/if}
</aside>

<style>
	/* The inset card, as on Graph: inside the pane, its own corner and edge. */
	.preview {
		width: var(--review-preview-w);
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

	/* As much of the description as there is room for; it scrolls past that. */
	.body {
		flex: 0 1 auto;
		min-height: 96px;
		overflow: auto;
		padding-right: 4px;
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

	.assign-anchor {
		position: relative;
		display: flex;
		flex-direction: column;
	}

	.preview :global(.btn) {
		justify-content: center;
		padding: 8px 14px;
	}
</style>
