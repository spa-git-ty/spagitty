<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import { agentName } from '../../describe';
	import { farmStore } from '../../store.svelte';
	import * as api from '../../api';
	import type { FarmSettings, PastFarm, Setup, TaskDraft } from '../../types';
	import {
		back,
		canContinue,
		initial,
		next,
		rememberedLine,
		rulesOf,
		skippable,
		skipToStart,
		soFar,
		sourcesOf,
		startLabel,
		startingPoint,
		wayOf,
		type Step,
		type WizardState
	} from '../../wizard';
	import SoFar from './SoFar.svelte';
	import StepCrew from './StepCrew.svelte';
	import StepFrom from './StepFrom.svelte';
	import StepGoal from './StepGoal.svelte';
	import StepStart from './StepStart.svelte';

	/**
	 * Starting a farm, one step at a time (FEAT-141). Replaces Setup.
	 *
	 * Only the goal is asked for. The crew and rules come back as the last farm
	 * in this repository had them; the issues, briefs and branch are optional.
	 * Starting creates the farm, configures it, remembers its crew and rules for
	 * next time, and then makes the tasks the way step 4 says — in that order,
	 * stopping at the first step that fails.
	 */
	interface Props {
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		/** A goal carried over from Wrap up's `Start a farm with it`. */
		title?: string;
		description?: string;
		/** The step on screen, for the header's steps. */
		step?: Step;
		/** Offer *A slice per task* (FEAT-143). */
		workspaces?: boolean;
		oncreated: () => void;
	}

	let {
		busy,
		act,
		title = '',
		description = '',
		step = $bindable(1),
		workspaces = false,
		oncreated
	}: Props = $props();

	let wizard = $state<WizardState>(untrack(() => initial(title, description)));
	let remembered = $state<Setup | null>(null);
	let rules = $state<FarmSettings>(rulesOf(null));
	let past = $state<PastFarm[]>([]);
	let rulesKey = $state(0);

	const planners = $derived(
		farmStore.usable.filter((a) => a.definition.capabilities.includes('planning')).map((a) => a.definition)
	);
	let planner = $state<string | null>(null);
	$effect(() => {
		if (!planner || !planners.some((p) => p.id === planner)) planner = planners[0]?.id ?? null;
	});

	const hasPolicy = $derived(farmStore.policy.sources.some((s) => s.path.endsWith('AGENTS.md')));
	const crew = $derived(farmStore.usable.map((a) => a.definition));
	const plannerName = $derived(planner ? agentName(planner, farmStore.agents) : null);
	const way = $derived(wayOf(wizard));
	const rows = $derived(
		soFar(wizard, { crew: crew.length, rules, remembered: remembered !== null, planner: plannerName })
	);

	$effect(() => {
		step = wizard.step;
	});

	onMount(() => {
		void (async () => {
			try {
				remembered = (await api.remembered()) ?? null;
				if (remembered) {
					rules = rulesOf(remembered);
					rulesKey++;
				}
			} catch {
				// No farm service yet, or nothing remembered: the defaults stand.
			}
			try {
				past = (await api.past()) ?? [];
			} catch {
				past = [];
			}
		})();
	});

	function go(nextState: WizardState) {
		wizard = nextState;
	}

	async function pick(farm: PastFarm) {
		if (farm.status === 'completed') {
			const from = startingPoint(farm);
			wizard = { ...wizard, title: from.title, notes: from.notes };
			rules = from.rules;
			rulesKey++;
			return;
		}
		if (await act('Could not resume the farm', () => api.resume(farm.id))) oncreated();
	}

	/** One draft per picked issue, in the issue's words, for Plan review. */
	function draftOf(issue: WizardState['issues'][number]): TaskDraft {
		const link = issue.url ? `\n\n${issue.url}` : '';
		return {
			title: issue.title,
			description: `${issue.body.trim()}${link}`.trim(),
			kind: 'general',
			priority: 'normal',
			dependsOn: [],
			allowedPaths: [],
			acceptanceCriteria: [`Issue #${issue.number} is fixed.`],
			verification: [],
			verificationOverrides: false,
			assignedAgent: null,
			ready: false
		};
	}

	async function start() {
		if (!wizard.title.trim()) return;
		const chosen = way;
		const created = await act('Could not start the farm', async () => {
			await api.create(wizard.title.trim(), wizard.notes.trim(), sourcesOf(wizard));
			await api.configure({ ...rules, agents: farmStore.usable.map((a) => a.definition.id) });
			// Remembered when the farm starts, not when it finishes: the next
			// farm's question is about this one's crew and rules.
			await Promise.resolve(api.remember()).catch(() => null);
			if (chosen === 'plan') await api.plan(planner);
			if (chosen === 'perIssue') for (const issue of wizard.issues) await api.addTask(draftOf(issue));
		});
		if (created) oncreated();
	}

	function primary() {
		if (wizard.step === 4) void start();
		else go(next(wizard));
	}

	const QUESTIONS: Record<Step, string> = {
		1: 'What should be true when it’s done?',
		2: 'Anything to start from?',
		3: 'Same crew and rules as last time?',
		4: 'How should the tasks be made?'
	};

	const question = $derived(wizard.step === 3 && !remembered ? 'Who works on it, and by which rules?' : QUESTIONS[wizard.step]);
	const BACK: Record<Step, string> = { 1: '', 2: '← Goal', 3: '← Start from', 4: '← Crew & rules' };
</script>

<div class="wizard">
	<div class="layout">
		<section class="question" aria-labelledby="wizard-question">
			<div class="heading">
				<span class="step-of">Step {wizard.step} of 4{wizard.step === 2 ? ' · optional' : ''}</span>
				<h2 id="wizard-question">{question}</h2>
				{#if wizard.step === 2}
					<p class="lead">The planner reads all of it, with the goal.</p>
				{:else if wizard.step === 3}
					<p class="lead">{rememberedLine(remembered)}</p>
				{:else if wizard.step === 4}
					<p class="lead">Nothing runs until you accept the tasks.</p>
				{/if}
			</div>

			{#if wizard.step === 1}
				<StepGoal bind:wizard {past} {busy} onenter={primary} onpick={pick} />
			{:else if wizard.step === 2}
				<StepFrom bind:wizard {busy} />
			{:else if wizard.step === 3}
				{#key rulesKey}
					<StepCrew bind:rules {busy} {act} {workspaces} />
				{/key}
			{:else}
				<StepStart bind:wizard {planners} bind:planner {hasPolicy} {busy} />
			{/if}
		</section>

		<div class="side">
			<SoFar {rows} {crew} />
		</div>
	</div>

	<footer class="bar ornament">
		{#if wizard.step === 1}
			<span class="hint">One line is enough. <span class="key mono">Enter</span> to go on.</span>
		{:else}
			<button class="back" disabled={busy} onclick={() => go(back(wizard))}>{BACK[wizard.step]}</button>
		{/if}
		<div class="actions">
			{#if wizard.step === 1}
				<Btn disabled={busy || !canContinue(wizard)} onclick={() => go(skipToStart(wizard))}>Skip to start</Btn>
			{:else if skippable(wizard.step)}
				<Btn disabled={busy} onclick={() => go(next(wizard))}>Skip</Btn>
			{/if}
			<Btn
				primary
				busy={busy && wizard.step === 4}
				disabled={busy || !canContinue(wizard) || (wizard.step === 4 && way === 'plan' && !planner)}
				onclick={primary}
			>
				{wizard.step === 4 ? startLabel(way, plannerName, wizard.issues.length) : 'Continue'}
			</Btn>
		</div>
	</footer>
</div>

<svelte:window
	onkeydown={(event) => {
		// Enter continues, except where Enter means something else: a new
		// line in the notes, a choice in a list, a button already focused.
		const target = event.target as HTMLElement | null;
		if (event.key !== 'Enter' || event.isComposing || busy) return;
		if (target?.closest('textarea, select, button, [contenteditable], #farm-goal')) return;
		if (target?.closest('input') && wizard.step !== 1) return;
		if (wizard.step === 4) return;
		event.preventDefault();
		primary();
	}}
/>

<style>
	.wizard {
		display: flex;
		flex-direction: column;
		gap: 22px;
		min-height: 100%;
	}

	.layout {
		flex: 1;
		display: flex;
		flex-wrap: wrap;
		gap: 32px;
		align-items: flex-start;
		justify-content: center;
	}

	.question {
		flex: 999 1 560px;
		min-width: 0;
		max-width: 780px;
		display: flex;
		flex-direction: column;
		gap: 20px;
		padding-top: 18px;
	}

	.heading {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.step-of {
		color: var(--accent);
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	h2 {
		margin: 0;
		font-size: calc(var(--fs-title) * 1.9);
		line-height: 1.15;
		font-weight: 600;
		letter-spacing: -0.015em;
	}

	.lead {
		margin: 0;
		color: var(--muted);
		font-size: calc(var(--fs-ui) * 0.96);
	}

	.side {
		flex: 1 1 280px;
		max-width: 340px;
		padding-top: 18px;
	}

	.bar {
		position: sticky;
		bottom: 4px;
		z-index: 3;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 12px 14px 12px 20px;
		border-radius: var(--r-ornament);
	}

	.hint {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.key {
		padding: 2px 6px;
		border-radius: 6px;
		background: var(--soft);
	}

	.back {
		height: 44px;
		padding: 0 6px;
		border: 0;
		background: none;
		color: var(--muted);
		font: inherit;
		cursor: pointer;
	}

	.back:hover:not(:disabled) {
		color: var(--ink);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
	}
</style>
