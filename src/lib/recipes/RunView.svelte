<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { blockOf, groupOf } from './catalogue';
	import { choiceLabel, levelWords, mainChoice, runShare, runStatus, took } from './model';
	import { recipes } from './store.svelte';
	import type { Choice, RunView } from './types';

	/**
	 * A recipe's run (FEAT-133): the steps with their state and time, the
	 * commands Spagitty ran and what they printed, the stop as the one tinted
	 * card with its choices, and the restore point.
	 *
	 * Every command here was run by Spagitty. An agent's lines are the ones
	 * in its colour: a message it wrote, or what it said went wrong.
	 */
	interface Props {
		view: RunView;
		/** A choice that leaves the screen: Merger, or Conflicts. */
		ongo: (choice: Choice) => void;
	}

	let { view, ongo }: Props = $props();

	const run = $derived(view.run);
	const stop = $derived(run.stop);
	const over = $derived(run.outcome !== null);
	const lines = $derived(recipes.lines);
	const main = $derived(stop ? mainChoice(stop) : null);
	const agent = $derived(
		run.recipe.agentHelp ? (recipes.helpers.find((each) => each.id === run.recipe.agent)?.name ?? null) : null
	);

	let output = $state<HTMLDivElement | null>(null);

	/** The newest line is the one to read: the output follows it down. */
	$effect(() => {
		void lines.length;
		void tick().then(() => {
			if (output) output.scrollTop = output.scrollHeight;
		});
	});

	async function choose(choice: Choice) {
		if (choice === 'openConflicts') {
			ongo(choice);
			return;
		}
		const taken = await recipes.answer(choice);
		if (taken && choice === 'openMerger') ongo(choice);
	}
</script>

<div class="screen">
	<header class="head">
		<Btn
			disabled={!over}
			title={over ? 'Back to the builder' : 'The run is not over'}
			onclick={() => recipes.closeRun()}>‹ Recipes</Btn
		>
		<span class="name mono">{run.recipe.name}</span>
		<span class="status" class:waiting={!!stop} class:over role="status">
			<span class="dot" aria-hidden="true"></span>
			{runStatus(run, view.working)}
		</span>
		<span class="gap"></span>
		<span
			class="meter"
			role="progressbar"
			aria-label="Steps done"
			aria-valuemin="0"
			aria-valuemax={run.steps.length}
			aria-valuenow={Math.round(runShare(run) * run.steps.length)}
		>
			<span style:width="{runShare(run) * 100}%"></span>
		</span>
		<Btn disabled={over || !view.live} title="End the run before its next step" onclick={() => recipes.stop()}>Stop</Btn>
	</header>

	<div class="body">
		<aside class="steps" aria-label="Steps">
			<ol>
				{#each run.steps as step, index (index)}
					<li class={step.state} class:current={index === run.at && !over} aria-current={index === run.at && !over}>
						<span class="mark" aria-hidden="true">
							{#if step.state === 'done'}
								<Icon name="check" size={14} weight={2} />
							{:else if step.state === 'waiting'}
								!
							{:else if step.state === 'skipped'}
								–
							{:else}
								<span class="ring"></span>
							{/if}
						</span>
						<span class="card" title={step.command}>
							<span
								class="square"
								style:background-color={groupOf(run.recipe.steps[index].block).tone}
								aria-hidden="true"
							></span>
							<span class="what">
								<span class="label">{blockOf(run.recipe.steps[index].block).label}</span>
								<span class="note detail">{step.label}</span>
							</span>
						</span>
						<span class="time mono">{took(step.ms)}</span>
					</li>
				{/each}
			</ol>

			<span class="grow"></span>
			<div class="restore">
				<span class="note small">Restore point</span>
				<span class="mono ref">refs/spagitty/restore/{run.restore}</span>
				<span class="note small">HEAD · branch tips · stash · working tree</span>
				<Btn
					disabled={view.working || run.outcome === 'undone'}
					title="Put everything back as it was before the first step. What was pushed stays pushed."
					onclick={() => recipes.undoRun()}>Undo the whole run</Btn
				>
			</div>
		</aside>

		<section class="work" aria-label="Run">
			<div class="who">
				{#if agent}
					<span class="agent"><Icon name="agent" size={16} /> {agent}</span>
				{:else}
					<span>Git commands only</span>
				{/if}
				<span class="level">{levelWords(run.level).short}</span>
				<span class="gap"></span>
				<span class="note small">Spagitty runs every command · on {run.branch || 'a detached HEAD'}</span>
			</div>

			<div class="output mono" bind:this={output} role="log" aria-label="Commands and their output">
				{#each lines as line, index (index)}
					<div class="line {line.kind}">{line.text}</div>
				{/each}
			</div>

			{#if stop}
				<div class="stop" class:conflict={stop.kind === 'conflict'} role="alert">
					<div class="title">
						<strong>{stop.title}</strong>
						{#each stop.paths.slice(0, 3) as path (path)}
							<span class="path mono">{path}</span>
						{/each}
						{#if stop.paths.length > 3}
							<span class="note small">and {stop.paths.length - 3} more</span>
						{/if}
					</div>
					{#if stop.inMerger}
						<p>The merge was aborted here, to be made in Merger. Come back when it has been made.</p>
					{:else if stop.explained}
						<p class="explained">{stop.explained}</p>
					{:else if stop.kind === 'approve' || stop.kind === 'ask'}
						<p class="mono">{run.steps[stop.step]?.command || stop.text}</p>
					{:else if stop.kind !== 'conflict' && stop.text}
						<pre class="mono">{stop.text}</pre>
					{/if}
					{#if recipes.refused}
						<p class="refused">{recipes.refused}</p>
					{/if}
					<div class="choices">
						{#each stop.choices as choice (choice)}
							<Btn
								primary={choice === main}
								quiet
								disabled={recipes.answering || !view.live}
								onclick={() => choose(choice)}>{choiceLabel(choice, stop)}</Btn
							>
						{/each}
					</div>
				</div>
			{:else if over}
				<div class="stop done" role="status">
					<div class="title">
						<strong>
							{run.outcome === 'done'
								? 'Every step ran'
								: run.outcome === 'undone'
									? 'The run was undone'
									: 'The run was ended'}
						</strong>
					</div>
					{#if run.outcome === 'ended'}
						<p>The steps that ran are kept. The restore point is still there.</p>
					{/if}
					<div class="choices">
						<Btn primary quiet onclick={() => recipes.closeRun()}>Back to the recipe</Btn>
					</div>
				</div>
			{/if}
		</section>
	</div>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		flex: none;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px 10px;
	}

	.name {
		font-size: var(--fs-title);
	}

	.status {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	.status .dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--ok);
	}

	.status.waiting {
		border-color: color-mix(in srgb, var(--warn) 62%, transparent);
		color: var(--warn);
	}

	.status.waiting .dot {
		background: var(--warn);
	}

	.status.over .dot {
		background: var(--muted);
	}

	.gap,
	.grow {
		flex: 1;
	}

	.meter {
		width: 180px;
		height: 6px;
		border-radius: var(--r-pill);
		background: var(--sunken);
		overflow: hidden;
	}

	.meter span {
		display: block;
		height: 100%;
		border-radius: var(--r-pill);
		background: var(--ok);
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 12px 12px;
	}

	.steps {
		flex: 0 0 340px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: auto;
	}

	ol {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	li {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.mark {
		flex: none;
		display: grid;
		place-items: center;
		width: 18px;
		color: var(--muted);
		font-weight: 700;
	}

	li.done .mark {
		color: var(--ok);
	}

	li.waiting .mark {
		color: var(--warn);
	}

	.ring {
		width: 10px;
		height: 10px;
		border: 1.5px solid var(--muted);
		border-radius: 50%;
	}

	.card {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
		background: var(--surface-2);
	}

	li.notStarted .card,
	li.skipped .card {
		opacity: 0.55;
	}

	li.waiting .card,
	li.current.running .card {
		border-color: color-mix(in srgb, var(--warn) 70%, transparent);
	}

	.square {
		flex: none;
		width: 9px;
		height: 9px;
		border-radius: 2px;
		opacity: 0.75;
	}

	.what {
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.label {
		font-weight: 600;
	}

	.detail {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
	}

	.time {
		flex: none;
		min-width: 34px;
		text-align: right;
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.small {
		margin: 0;
		font-size: var(--fs-secondary);
	}

	.restore {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 4px;
		padding: 10px;
		border-radius: var(--r-field);
		background: var(--sunken);
	}

	.ref {
		font-size: var(--fs-mono);
		overflow-wrap: anywhere;
		user-select: text;
	}

	.work {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	.who {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.agent {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--agent);
		font-weight: 600;
	}

	.level {
		padding: 2px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
	}

	.output {
		flex: 1;
		min-height: 80px;
		padding: 12px;
		border-radius: var(--r-field);
		background: var(--bg);
		font-size: var(--fs-mono);
		overflow: auto;
		user-select: text;
	}

	.line {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		color: var(--muted);
	}

	.line.command {
		color: var(--ink);
	}

	.line.error {
		color: var(--danger);
	}

	.line.agent {
		color: var(--agent);
	}

	.stop {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px 14px;
		border: 1px solid color-mix(in srgb, var(--warn) 55%, transparent);
		border-radius: var(--r-button);
		background: color-mix(in srgb, var(--warn) 8%, var(--surface));
	}

	.stop.conflict {
		border-color: color-mix(in srgb, var(--agent) 55%, transparent);
		background: color-mix(in srgb, var(--agent) 8%, var(--surface));
	}

	.stop.done {
		border-color: var(--soft);
		background: var(--surface-2);
	}

	.title {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}

	.path {
		padding: 1px 8px;
		border-radius: var(--r-pill);
		background: var(--sunken);
		font-size: var(--fs-mono);
	}

	.stop p,
	.stop pre {
		margin: 0;
	}

	.stop pre {
		max-height: 140px;
		font-size: var(--fs-mono);
		white-space: pre-wrap;
		overflow: auto;
		user-select: text;
	}

	.refused {
		color: var(--danger);
	}

	.choices {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
</style>
