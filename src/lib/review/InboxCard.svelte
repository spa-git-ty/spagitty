<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import type { PullRequest } from '$lib/types';
	import { chipsOf, progressOf, sizeLabel, sizeOf, type GroupId } from './inbox';
	import type { ReviewRecord } from './record';
	import Icon from '$lib/ui/Icon.svelte';
	import { cardLine } from '$lib/agents/levels';
	import type { Assignment } from '$lib/agents/types';

	/**
	 * One pull request in the Review inbox (FEAT-087): what it is, how big,
	 * and how far you got.
	 */
	interface Props {
		pr: PullRequest;
		record: ReviewRecord | null;
		group: GroupId;
		selected: boolean;
		/** Shown on rows from "All my repos", which do not share a repository. */
		showRepository: boolean;
		onselect: () => void;
		onopen: () => void;
		/** The agent assigned to it, so it can be followed from the list (2.0). */
		agent?: Assignment | null;
	}

	let { pr, record, group, selected, showRepository, onselect, onopen, agent = null }: Props = $props();

	const size = $derived(sizeOf(pr));
	const sizes = $derived(sizeLabel(pr));
	const progress = $derived(progressOf(pr, record));
	const chips = $derived(chipsOf(pr, record, group));
</script>

<!--
	A click chooses it for the preview; a double click, or Enter on a chosen
	one, opens it. The preview's button is the plain way in.
-->
<button
	class="card row"
	class:selected
	aria-pressed={selected}
	onclick={onselect}
	ondblclick={onopen}
	onkeydown={(event) => {
		if (event.key === 'Enter' && selected) {
			event.preventDefault();
			onopen();
		}
	}}
>
	<span class="what">
		<span class="headline">
			<span class="number mono">#{pr.number}</span>
			<span class="title">{pr.title}</span>
		</span>
		<span class="meta note">
			<span><span class="who">{pr.authorName}</span> · {relativeTime(pr.updated)}</span>
			{#if showRepository && pr.repository}
				<span class="tag mono">{pr.repository}</span>
			{/if}
			<span class="tag mono">{pr.sourceBranch}</span>
			{#if chips.conflict}<span class="tag resolve">{chips.conflict}</span>{/if}
			{#if chips.threads}<span class="tag on">{chips.threads}</span>{/if}
			{#if agent}
				<span class="agent" title={agent.sentence}><Icon name="agent" size="0.95em" weight={2} />{cardLine(agent)}</span>
			{/if}
		</span>
	</span>

	<span class="size" title="How big it is">
		<span class="bars" aria-hidden="true">
			{#each [1, 2, 3] as step (step)}
				<span class="bar" class:lit={size >= step}></span>
			{/each}
		</span>
		<span class="note small">
			{sizes.files}{#if sizes.lines}&nbsp;· {sizes.lines}{/if}
		</span>
	</span>

	<span class="progress">
		<span class="track" aria-hidden="true">
			<span class="fill" class:changed={progress.changed} style:width="{progress.share * 100}%"
			></span>
		</span>
		<span class="note small">{progress.label}</span>
	</span>
</button>

<style>
	.agent {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--agent);
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 18px;
		width: 100%;
		padding: 14px 16px;
		text-align: left;
		color: inherit;
	}

	.row.selected {
		border-color: color-mix(in srgb, var(--accent) 55%, transparent);
	}

	.what {
		flex: 1 1 360px;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 7px;
	}

	.headline {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.number {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.title {
		font-family: var(--read-font);
		font-size: calc(var(--fs-ui) * 1.06);
		line-height: 1.4;
		min-width: 0;
	}

	.meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
	}

	.who {
		color: var(--ink);
	}

	/* The chips are labels here, not controls: the whole card is the button. */
	.tag {
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 2px 9px;
		font-size: var(--fs-mono);
		background-color: var(--surface-veil);
		white-space: nowrap;
	}

	.tag.on {
		border-color: color-mix(in srgb, var(--accent) 62%, transparent);
		color: var(--accent);
		background-color: color-mix(in srgb, var(--accent) 14%, var(--surface));
	}

	/* A conflict fix is the sky lane's colour everywhere it appears. */
	.tag.resolve {
		color: var(--lane-5);
		border-color: color-mix(in srgb, var(--lane-5) 50%, transparent);
		background-color: color-mix(in srgb, var(--lane-5) 11%, transparent);
	}

	.size {
		width: 110px;
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.bars {
		display: flex;
		gap: 3px;
	}

	.bar {
		flex: 1;
		height: 6px;
		border-radius: var(--r-pill);
		background: var(--soft);
	}

	.bar.lit {
		background: var(--ink);
	}

	.progress {
		width: 120px;
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.track {
		height: 6px;
		border-radius: var(--r-pill);
		background: var(--soft);
		overflow: hidden;
	}

	.fill {
		display: block;
		height: 100%;
		background: var(--ok);
	}

	.fill.changed {
		background: var(--warn);
	}

	.small {
		font-size: var(--fs-mono);
	}
</style>
