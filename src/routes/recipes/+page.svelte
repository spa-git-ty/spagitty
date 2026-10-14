<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import * as api from '$lib/api';
	import { agents } from '$lib/agents/store.svelte';
	import BranchMenu from '$lib/branches/BranchMenu.svelte';
	import type { BranchOption } from '$lib/branches/pick';
	import { merger } from '$lib/merger/store.svelte';
	import { isMac } from '$lib/platform';
	import AgentPanel from '$lib/recipes/AgentPanel.svelte';
	import Catalogue from '$lib/recipes/Catalogue.svelte';
	import { keyAction, keyHints, typing } from '$lib/recipes/keys';
	import { findingsAt, MAX_STEPS } from '$lib/recipes/model';
	import RunView from '$lib/recipes/RunView.svelte';
	import StepCard from '$lib/recipes/StepCard.svelte';
	import { recipes } from '$lib/recipes/store.svelte';
	import type { Choice } from '$lib/recipes/types';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * Recipes (1V, FEAT-133): git steps in blocks.
	 *
	 * A recipe is a stack of blocks from a closed catalogue — stash, pull,
	 * merge, pop, commit, push — each with a few parameters chosen from
	 * lists. The validator says, under each block, what is wrong with the
	 * stack as the repository is now. *Run* saves a restore point and runs
	 * the steps one at a time; Spagitty runs every command itself.
	 *
	 * An agent helps when its switch is on: it proposes the blocks from a
	 * sentence, writes a message left to it, and explains a failure. Off, no
	 * agent is called and the run is the git commands and nothing else.
	 */
	const info = $derived(repo.info);
	const draft = $derived(recipes.draft);
	const check = $derived(recipes.check);
	const findings = $derived(check?.findings ?? []);
	const here = $derived(check?.world.current ?? info?.head.branch ?? null);

	let branchMenu = $state<{
		x: number;
		y: number;
		anchor: HTMLElement;
		value: string;
		options: BranchOption[];
	} | null>(null);
	/**
	 * What the branch chosen is given to. Kept apart from the menu: the menu
	 * closes first and picks after, and by then it is no longer on screen.
	 */
	let picking: (name: string) => void = () => {};
	let pasting = $state(false);
	let pasted = $state('');

	/** This repository's recipes: on arrival, and when another is opened. */
	$effect(() => {
		const path = info?.path ?? null;
		void repo.generation;
		untrack(() => {
			if (!path || !api.inTauri()) return;
			void agents.load(path);
			void recipes.load(path);
		});
	});

	/** HEAD moved, or the tree changed: what the validator said may not hold. */
	$effect(() => {
		void info?.head.id;
		void info?.head.branch;
		untrack(() => {
			if (recipes.loaded && !recipes.run) void recipes.checkNow();
		});
	});

	/** The branches a step can name: the repository's, and the ones made above it. */
	function branchesFor(index: number): BranchOption[] {
		const made = (draft?.steps ?? [])
			.slice(0, index)
			.flatMap((step) => (step.block === 'newBranch' && step.params.name ? [step.params.name] : []));
		const names = [...new Set([...(check?.world.branches ?? []), ...made])];
		return names.map((name) => ({ name, group: 'local' as const }));
	}

	function openBranches(index: number, anchor: HTMLElement, value: string, pick: (name: string) => void) {
		if (branchMenu?.anchor === anchor) {
			branchMenu = null;
			return;
		}
		const box = anchor.getBoundingClientRect();
		picking = pick;
		branchMenu = { x: box.left, y: box.bottom + 4, anchor, value, options: branchesFor(index) };
	}

	function onkey(event: KeyboardEvent) {
		if (!draft || recipes.run || typing(event.target) || branchMenu) return;
		const action = keyAction(
			{ key: event.key, ctrl: event.ctrlKey, meta: event.metaKey, alt: event.altKey, shift: event.shiftKey },
			isMac()
		);
		if (!action) return;
		event.preventDefault();
		const at = recipes.selected;
		switch (action.kind) {
			case 'add':
				if (draft.steps.length < MAX_STEPS) recipes.add(action.block);
				break;
			case 'down':
				recipes.select(Math.min(at + 1, draft.steps.length - 1));
				break;
			case 'up':
				recipes.select(Math.max(at - 1, 0));
				break;
			case 'moveDown':
				if (at >= 0) recipes.move(at, 1);
				break;
			case 'moveUp':
				if (at >= 0) recipes.move(at, -1);
				break;
			case 'remove':
				if (at >= 0) recipes.remove(at);
				break;
			case 'run':
				if (check && check.errors === 0 && check.level !== 'suggest' && draft.steps.length > 0) {
					void recipes.start();
				}
				break;
			case 'undo':
				recipes.undo();
				break;
		}
	}

	async function copy(text: string, what: string) {
		try {
			await navigator.clipboard.writeText(text);
			notice.ok(`${what} copied`);
		} catch (cause) {
			notice.failed(`Could not copy ${what.toLowerCase()}`, cause);
		}
	}

	async function importPasted() {
		if (await recipes.importFrom(pasted)) {
			pasting = false;
			pasted = '';
		}
	}

	/** A choice at a stop that leaves this screen. */
	function go(choice: Choice) {
		const stop = recipes.run?.run.stop;
		if (choice === 'openConflicts') {
			void goto('/conflicts');
		} else if (choice === 'openMerger' && stop?.source) {
			// The merge the recipe asked for, in Merger: the other branch into
			// the one the run is on.
			const onto = recipes.run?.run.recipe.steps
				.slice(0, stop.step)
				.reduce(
					(current, step) =>
						step.block === 'switch' ? step.params.branch : step.block === 'newBranch' ? step.params.name : current,
					recipes.run?.run.branch ?? ''
				);
			merger.pickA(stop.source);
			if (onto) merger.pickB(onto);
			void goto('/merge');
		}
	}
</script>

<svelte:window onkeydown={onkey} />

{#if !info}
	<div class="screen">
		<EmptyState message="Open a repository to build a recipe for it." />
	</div>
{:else if recipes.run}
	<RunView view={recipes.run} ongo={go} />
{:else}
	<div class="screen">
		<header class="head">
			<span class="title">Recipes</span>
			<div class="chips" role="tablist" aria-label="Saved recipes">
				{#each recipes.saved as saved (saved.name)}
					<button
						role="tab"
						class="chip mono"
						class:on={saved.name === recipes.savedAs}
						aria-selected={saved.name === recipes.savedAs}
						onclick={() => recipes.open(saved.name)}>{saved.name}</button
					>
				{/each}
				<button class="chip add" aria-label="New recipe" title="New recipe" onclick={() => recipes.create()}>+</button>
			</div>
			<span class="gap"></span>
			{#if draft}
				<span class="note on">
					on
					<button
						class="mono branch"
						title={draft.branch
							? `This recipe only runs on ${draft.branch}. Press to let it run on any branch.`
							: 'This recipe runs on whichever branch is checked out. Press to keep it to this one.'}
						onclick={() => recipes.setBranch(draft.branch ? '' : (here ?? ''))}
						>{draft.branch || (here ? `${here} · any branch` : 'any branch')}</button
					>
				</span>
				<Btn title="Open a recipe from its exported JSON" onclick={() => (pasting = !pasting)}>Import</Btn>
				<Btn title="Copy this recipe as JSON, to keep or to send" onclick={() => copy(recipes.exported(), 'Recipe')}
					>Export</Btn
				>
				<Btn
					disabled={!check}
					title="The steps as a shell script with set -e"
					onclick={() => copy(check?.script ?? '', 'Script')}>Copy as script</Btn
				>
				<Btn danger disabled={!recipes.savedAs} onclick={() => recipes.removeSaved()}>Delete</Btn>
				<Btn primary quiet disabled={!recipes.dirty || draft.name.trim() === ''} onclick={() => recipes.save()}
					>Save</Btn
				>
			{/if}
		</header>

		{#if pasting}
			<div class="paste">
				<textarea
					class="mono"
					rows="4"
					aria-label="A recipe's exported JSON"
					placeholder="Paste a recipe's exported JSON"
					bind:value={pasted}
				></textarea>
				<Btn disabled={pasted.trim() === ''} onclick={importPasted}>Open it</Btn>
			</div>
		{/if}

		{#if recipes.error}
			<p class="note error" role="alert">{recipes.error}</p>
		{/if}

		{#if draft}
			<div class="body">
				<Catalogue onadd={(block) => draft.steps.length < MAX_STEPS && recipes.add(block)} />

				<section class="stack" aria-label="The recipe">
					<div class="start">
						<span class="play" aria-hidden="true">▶</span>
						<span>On Run</span>
						<input
							class="mono name"
							type="text"
							aria-label="The recipe's name"
							placeholder="name"
							spellcheck="false"
							autocomplete="off"
							value={draft.name}
							oninput={(event) => recipes.rename(event.currentTarget.value)}
						/>
					</div>
					{#each findingsAt(findings, null) as finding (finding.code)}
						<p class="finding" class:error={finding.severity === 'error'}>{finding.text}</p>
					{/each}

					<div class="steps">
						{#each draft.steps as step, index (index)}
							<StepCard
								{index}
								{step}
								command={check?.commands[index] ?? ''}
								findings={findingsAt(findings, index)}
								selected={index === recipes.selected}
								agentHelp={draft.agentHelp}
								world={check?.world ?? null}
								first={index === 0}
								onchange={(next) => recipes.set(index, next)}
								onremove={() => recipes.remove(index)}
								onup={() => recipes.move(index, -1)}
								onselect={() => recipes.select(index)}
								onbranch={(anchor, value, pick) => openBranches(index, anchor, value, pick)}
							/>
						{:else}
							<p class="note empty">
								No blocks yet. Press a block on the left, or its key{draft.agentHelp
									? ', or write what you want on the right and let the agent propose them'
									: ''}.
							</p>
						{/each}
					</div>

					<span class="grow"></span>
					<div class="hints" aria-label="Keys">
						{#each keyHints(isMac()) as hint (hint.what)}
							<span><kbd class="mono">{hint.keys}</kbd> {hint.what}</span>
						{/each}
					</div>
				</section>

				<AgentPanel />
			</div>
		{/if}
	</div>
{/if}

{#if branchMenu}
	<BranchMenu
		x={branchMenu.x}
		y={branchMenu.y}
		anchor={branchMenu.anchor}
		label="Choose a branch"
		options={branchMenu.options}
		value={branchMenu.value || null}
		current={here}
		onpick={(name) => picking(name)}
		onclose={() => (branchMenu = null)}
	/>
{/if}

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
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		padding: 12px 16px 10px;
	}

	.title {
		margin-right: 6px;
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		min-width: 0;
	}

	.chip {
		padding: 3px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		background-color: var(--surface-veil);
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.chip:hover {
		background-color: var(--hover);
	}

	.chip.on {
		border-color: color-mix(in srgb, var(--accent) 62%, transparent);
		background-color: color-mix(in srgb, var(--accent) 14%, var(--surface));
		color: var(--accent);
	}

	.chip.add {
		color: var(--muted);
	}

	.gap,
	.grow {
		flex: 1;
	}

	.on {
		white-space: nowrap;
	}

	.branch {
		padding: 2px 8px;
		border-radius: var(--r-field);
		font-size: var(--fs-mono);
	}

	.branch:hover {
		background: var(--hover);
	}

	.paste {
		display: flex;
		align-items: flex-end;
		gap: 8px;
		padding: 0 16px 10px;
	}

	.paste textarea {
		flex: 1;
		resize: none;
		font-size: var(--fs-mono);
	}

	.error {
		margin: 0;
		padding: 0 16px 8px;
		color: var(--danger);
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 12px 12px;
	}

	.stack {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 14px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: auto;
	}

	.start {
		align-self: flex-start;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: 1px dashed var(--soft);
		border-radius: var(--r-button);
		color: var(--muted);
	}

	.play {
		font-size: var(--fs-mono);
	}

	.name {
		width: 200px;
		padding: 3px 8px;
		font-size: var(--fs-mono);
	}

	.steps {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.finding {
		margin: 0;
		font-size: var(--fs-secondary);
		color: var(--warn);
	}

	.finding.error {
		color: var(--danger);
	}

	.empty {
		margin: 8px 0;
	}

	.hints {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		gap: 14px;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	kbd {
		padding: 1px 5px;
		border: 1px solid var(--soft);
		border-radius: 4px;
		font-size: var(--fs-mono);
	}
</style>
