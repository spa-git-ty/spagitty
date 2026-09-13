<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { PROVIDER_LABELS } from '../describe';
	import type { AgentProvider, AgentStatus } from '../types';

	/**
	 * What the Farm screen is before there is a farm (FEAT-073).
	 *
	 * It was four lines in a centred box — a heading, a sentence, and a button
	 * that sent you to the Settings pane to find the field you actually needed.
	 * That is the worst possible first screen for the feature the product is
	 * named after: it explains nothing, it asks for a navigation before it asks
	 * for anything useful, and it says nothing about whether this machine can
	 * run a farm at all.
	 *
	 * So this page answers the three questions somebody standing in front of it
	 * has, in the order they have them:
	 *
	 *   1. *What is this?* — the paragraph, and the four steps. A farm is not a
	 *      chat window: it is a goal, tasks cut from it, one worktree and branch
	 *      per task, and a merge you make. Somebody who reads the steps knows
	 *      what will happen to their repository before they start it.
	 *   2. *How do I start?* — the goal field, right here, with the button
	 *      beside it. Starting a farm needs one sentence and no navigation.
	 *   3. *Will it work?* — the readiness rows. Whether an agent was found,
	 *      whether this repository has rules for one, and whether anything
	 *      checks the work. Each row is a fact plus, where there is one, the
	 *      action that changes it.
	 *
	 * Everything on it is a prop. The screen owns the calls; this owns the
	 * layout and the words, which is what makes it testable without a backend.
	 */
	interface Props {
		/** Agents that could take a task right now. */
		ready: AgentStatus[];
		/** Providers looked for on `PATH` and not found. */
		undetected: AgentProvider[];
		/** Files the prompt policy is read from — `AGENTS.md` and friends. */
		policySources: string[];
		/** How many verification commands are configured. */
		verificationCount: number;
		busy?: boolean;
		onstart: (title: string, description: string) => void;
		ondetect: () => void;
		onwritePolicy: () => void;
		onsettings: () => void;
	}

	let {
		ready,
		undetected,
		policySources,
		verificationCount,
		busy = false,
		onstart,
		ondetect,
		onwritePolicy,
		onsettings
	}: Props = $props();

	let title = $state('');
	let description = $state('');

	const canStart = $derived(!busy && title.trim().length > 0);

	/**
	 * The steps, as data.
	 *
	 * Four, and no more: this is the whole loop, and a fifth step would be a
	 * detail of one of these rather than a thing that happens.
	 */
	const STEPS = [
		{
			title: 'Set the goal',
			detail: 'One sentence about what should be true when the farm is finished.'
		},
		{
			title: 'Cut it into tasks',
			detail:
				'Write them yourself, or ask an agent to plan and then edit what it proposes. ' +
				'A task can depend on another, and the ones that can run do.'
		},
		{
			title: 'Agents work in parallel',
			detail:
				'Each task gets a branch and a worktree of its own, so nothing an agent does ' +
				'reaches your working copy. Up to four run at once.'
		},
		{
			title: 'You verify and merge',
			detail:
				'Verification commands run in the task’s worktree before it can be accepted. ' +
				'Reviewing and merging stay yours.'
		}
	];

	/** The names of the agents that could start work, for the readiness row. */
	const readyNames = $derived(ready.map((agent) => agent.definition.displayName).join(', '));

	const missingNames = $derived(
		undetected.map((provider) => PROVIDER_LABELS[provider]).join(', ')
	);
</script>

<section class="starter">
	<header class="hero">
		<span class="mark" aria-hidden="true"><Icon name="farm" size="1.9em" /></span>
		<h1 class="title">Run an agent farm in this repository</h1>
		<p class="lede">
			A farm is a goal, the tasks it was broken into, and the coding agents working them —
			each on its own branch, in its own worktree, side by side. Spagitty runs them and shows
			you what they did in the history you already read here. It is not a model and it does
			not contain one: the agents are the ones already installed on this machine.
		</p>
	</header>

	<div class="goal">
		<label class="field">
			<span class="note">What is this farm for?</span>
			<input
				class="goal-title"
				bind:value={title}
				placeholder="e.g. Add dark mode to the settings screen"
				onkeydown={(event) => {
					if (event.key === 'Enter' && canStart) onstart(title.trim(), description.trim());
				}}
			/>
		</label>
		<label class="field">
			<span class="note">Anything the agents should know (optional)</span>
			<textarea
				bind:value={description}
				rows="2"
				placeholder="Constraints, conventions, files to leave alone"
			></textarea>
		</label>
		<div class="actions">
			<Btn primary disabled={!canStart} onclick={() => onstart(title.trim(), description.trim())}>
				Start the farm
			</Btn>
			<span class="note">Nothing runs until you say so. All of this is editable later.</span>
		</div>
	</div>

	<ol class="steps">
		{#each STEPS as step, index (step.title)}
			<li class="step">
				<span class="ordinal" aria-hidden="true">{index + 1}</span>
				<span class="step-body">
					<span class="step-title">{step.title}</span>
					<span class="note">{step.detail}</span>
				</span>
			</li>
		{/each}
	</ol>

	<div class="ready">
		<h2 class="heading">Before you start</h2>

		<!--
			Each row is a fact about this machine or this repository, painted by
			what it means: a dot that is green when the thing is in place and
			amber when it is not. None of them blocks starting a farm — a farm
			with no agent is still a plan, and saying so is more useful than
			disabling the button.
		-->
		<div class="check" data-state={ready.length > 0 ? 'ok' : 'warn'}>
			<span class="dot" aria-hidden="true"></span>
			<span class="what">Agents</span>
			<span class="note fact">
				{#if ready.length > 0}
					{ready.length} ready — {readyNames}
				{:else if undetected.length > 0}
					None found on <span class="mono">PATH</span>. Looked for {missingNames}.
				{:else}
					Nothing detected yet.
				{/if}
			</span>
			<Btn disabled={busy} onclick={ondetect}>Look again</Btn>
		</div>

		<div class="check" data-state={policySources.length > 0 ? 'ok' : 'warn'}>
			<span class="dot" aria-hidden="true"></span>
			<span class="what">Repository rules</span>
			<span class="note fact">
				{#if policySources.length > 0}
					Read from {policySources.join(', ')} and attached to every prompt.
				{:else}
					No rules file, so agents follow whatever conventions they find in the code.
				{/if}
			</span>
			{#if policySources.length === 0}
				<Btn disabled={busy} onclick={onwritePolicy}>Write AGENTS.md</Btn>
			{/if}
		</div>

		<div class="check" data-state={verificationCount > 0 ? 'ok' : 'warn'}>
			<span class="dot" aria-hidden="true"></span>
			<span class="what">Verification</span>
			<span class="note fact">
				{#if verificationCount > 0}
					{verificationCount}
					{verificationCount === 1 ? 'command runs' : 'commands run'} against every task's worktree.
				{:else}
					Nothing checks a task, so work reaches review having been checked by nobody.
				{/if}
			</span>
			<Btn disabled={busy} onclick={onsettings}>Settings</Btn>
		</div>
	</div>
</section>

<style>
	/*
	 * One column, centred, with a readable measure.
	 *
	 * The screen this replaces was vertically centred in the pane, which put
	 * its heading in a different place depending on the window's height. This
	 * starts at the top and stays there.
	 */
	.starter {
		display: flex;
		flex-direction: column;
		gap: 18px;
		width: 100%;
		max-width: 62rem;
		margin: 0 auto;
		padding: 4px 4px 24px;
	}

	.hero {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	/* The screen's own glyph, in the accent, at the size a page title takes.
	   It is the rail's farm icon — the same shape in both places. */
	.mark {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.6em;
		height: 2.6em;
		border-radius: var(--r-panel);
		color: var(--accent);
		background-color: var(--accent-soft);
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 620;
		letter-spacing: -0.01em;
	}

	.lede {
		margin: 0;
		max-width: 62ch;
		color: var(--muted);
		line-height: 1.5;
	}

	/* The one thing to do on this screen, so it is a surface rather than two
	   loose fields under a paragraph. */
	.goal {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px 16px;
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
		border-radius: var(--r-panel);
		background-color: var(--surface-veil);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.field input,
	.field textarea {
		width: 100%;
		padding: 7px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		background-color: var(--bg);
		color: var(--ink);
		font: inherit;
		font-size: var(--fs-secondary);
	}

	.goal-title {
		font-size: var(--fs-ui);
	}

	.field textarea {
		resize: vertical;
	}

	.field input:focus-visible,
	.field textarea:focus-visible {
		outline: 2px solid var(--ring);
		outline-offset: 1px;
		border-color: var(--accent);
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}

	/*
	 * The loop, numbered.
	 *
	 * A grid rather than a list with markers: the ordinal is a token in the
	 * accent and the browser's own numbering cannot be painted.
	 */
	.steps {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12.5rem, 1fr));
		gap: 10px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.step {
		display: flex;
		align-items: flex-start;
		gap: 9px;
		padding: 10px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
		background-color: var(--surface-veil);
	}

	.ordinal {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.6em;
		height: 1.6em;
		border-radius: 50%;
		color: var(--accent);
		background-color: var(--accent-soft);
		font-size: var(--fs-secondary);
		font-weight: 650;
		font-variant-numeric: tabular-nums;
	}

	.step-body {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.step-title {
		font-weight: 600;
	}

	.ready {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.heading {
		margin: 0 0 2px;
		font-size: var(--fs-ui);
		font-weight: 600;
	}

	.check {
		display: flex;
		align-items: center;
		gap: 9px;
		padding: 8px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-row);
		background-color: var(--stripe);
	}

	.dot {
		flex: none;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background-color: var(--muted);
	}

	.check[data-state='ok'] .dot {
		background-color: var(--ok);
	}

	.check[data-state='warn'] .dot {
		background-color: var(--warn);
	}

	.what {
		flex: none;
		width: 9rem;
		font-weight: 550;
		font-size: var(--fs-secondary);
	}

	/* The fact takes the row's spare width so the action stays at the far
	   right, in the same place on all three rows. */
	.fact {
		flex: 1;
		min-width: 0;
	}

	/* Narrow: the rows stack rather than squeezing the fact to two words. */
	@media (max-width: 720px) {
		.check {
			flex-wrap: wrap;
		}

		.what {
			width: auto;
		}

		.fact {
			flex: 1 1 100%;
			order: 3;
		}
	}
</style>
