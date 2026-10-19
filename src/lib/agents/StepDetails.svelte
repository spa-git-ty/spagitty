<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { fileAttrs } from '$lib/files/open';
	import Markdown from '$lib/ui/Markdown.svelte';
	import { stepTime } from './levels';
	import { KIND_LABELS, blocksOf, lineCount } from './record';
	import type { Step } from './types';

	/**
	 * One step, in full (FEAT-118): what Spagitty sent the agent, and what came
	 * back, each part labelled as what it is. Replies are read as Markdown,
	 * commands and tool input are set as code, and a file that was read is a
	 * link to it. Hidden until the card's *Details* is opened.
	 */
	interface Props {
		step: Step;
		now: number;
		/** Go to a file the agent read, in the diff. Absent where there is no diff. */
		onfile?: (path: string) => void;
	}

	let { step, now, onfile }: Props = $props();

	const blocks = $derived(blocksOf(step));
	const prompt = $derived(step.prompt ?? null);
</script>

<div class="step-details" data-step={step.index}>
	<div class="title">
		<span class="label">{step.label}</span>
		<span class="note mono">{stepTime(step, now)}</span>
	</div>

	{#if prompt}
		<details class="block sent">
			<summary><span class="kind">Sent</span> What Spagitty asked · {lineCount(prompt)} lines</summary>
			<pre class="code mono">{prompt}</pre>
		</details>
	{:else}
		<p class="note">Nothing was recorded as sent for this step.</p>
	{/if}

	{#each blocks as block, i (i)}
		<div class="block {block.kind}">
			<span class="kind">{KIND_LABELS[block.kind]}</span>
			{#if block.kind === 'reply'}
				<div class="reply"><Markdown source={block.text} /></div>
			{:else if block.kind === 'command'}
				<pre class="code mono">$ {block.text}</pre>
			{:else if block.kind === 'read' && block.path}
				{#if onfile}
					<button class="file mono" title="Show {block.path} in the diff" onclick={() => onfile?.(block.path!)}>
						{block.path}
					</button>
				{:else}
					<span class="mono" {...fileAttrs(block.path, 'HEAD', undefined, { click: true })}>{block.path}</span>
				{/if}
			{:else if block.kind === 'tool'}
				<code class="inline mono">{block.text}</code>
			{:else}
				<span class="text">{block.text}</span>
			{/if}
		</div>
	{:else}
		<p class="note">Nothing came back in this step yet.</p>
	{/each}

	{#if step.sent.length}
		<div class="block sentfiles">
			<span class="kind">Sent</span>
			<span class="text">Files whose content went to the model: <span class="mono">{step.sent.join(', ')}</span></span>
		</div>
	{/if}
	{#each step.checks as check (check.command)}
		<div class="block {check.passed ? 'result' : 'error'}">
			<span class="kind">Check</span>
			<span class="text mono">{check.command} · {check.passed ? 'passed' : 'failed'}</span>
		</div>
	{/each}
</div>

<style>
	.step-details {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}

	.title {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		font-weight: 600;
	}

	.block {
		display: grid;
		grid-template-columns: 64px minmax(0, 1fr);
		gap: 8px;
		align-items: start;
		font-size: var(--fs-secondary);
	}

	.kind {
		color: var(--muted);
		font-size: var(--fs-mono);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding-top: 2px;
	}

	.block.error .text,
	.block.error .kind {
		color: var(--danger);
	}

	.block.result .text {
		color: var(--muted);
	}

	.sent summary {
		grid-column: 1 / -1;
		cursor: pointer;
	}

	.sent {
		display: block;
	}

	.sent summary .kind {
		margin-right: 6px;
	}

	.code {
		margin: 4px 0 0;
		padding: 8px 10px;
		border-radius: 8px;
		background: var(--sunken);
		white-space: pre-wrap;
		word-break: break-word;
		max-height: 280px;
		overflow: auto;
		font-size: var(--fs-mono);
	}

	.inline {
		padding: 1px 6px;
		border-radius: 6px;
		background: var(--sunken);
		word-break: break-word;
	}

	.file {
		justify-self: start;
		padding: 0;
		border: 0;
		background: none;
		color: var(--accent);
		text-align: left;
		cursor: pointer;
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	.reply {
		min-width: 0;
	}
</style>
