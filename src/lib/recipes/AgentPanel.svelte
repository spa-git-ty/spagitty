<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { LEVELS, levelWords, MAX_NOTE, MAX_SENTENCE, runLabel } from './model';
	import { recipes } from './store.svelte';
	import type { Level } from './types';

	/**
	 * How a recipe is run, and whether an agent helps (FEAT-133).
	 *
	 * *Agent help* is a switch, as the author asked on 2026-10-13. On, a
	 * sentence is written and the agent proposes the blocks; the note and the
	 * brief are there, and a step's message can be left to it. Off, those are
	 * disabled, no agent is called, and what is shown is the git commands the
	 * run is, as a script.
	 *
	 * Either way Spagitty runs every command itself, a step at a time, after a
	 * restore point is made.
	 */
	const draft = $derived(recipes.draft);
	const check = $derived(recipes.check);
	const on = $derived(draft?.agentHelp ?? false);
	const level = $derived(check?.level ?? draft?.level ?? 'stepByStep');
	const errors = $derived(check?.errors ?? 0);
	const steps = $derived(draft?.steps.length ?? 0);
	const readOnly = $derived(level === 'suggest');
	const helper = $derived(recipes.helpers.find((each) => each.id === draft?.agent) ?? null);
</script>

{#if draft}
	<aside class="panel" aria-label="How it runs">
		<div class="row">
			<span class="label" title="An agent proposes the blocks from a sentence, writes a message left to it, and explains a failure. It never runs a command.">
				Agent help
			</span>
			<Chip active={on} onclick={() => recipes.setAgentHelp(!on)}>{on ? 'on' : 'off'}</Chip>
		</div>

		{#if on}
			<div class="row agent">
				<span class="hex" aria-hidden="true"><Icon name="agent" size={16} /></span>
				<select
					aria-label="Agent"
					value={draft.agent ?? ''}
					onchange={(event) => recipes.setAgent(event.currentTarget.value || null)}
				>
					<option value="">Choose an agent</option>
					{#each recipes.helpers as each (each.id)}
						<option value={each.id}>{each.name}{each.detail ? ` · ${each.detail}` : ''}</option>
					{/each}
				</select>
			</div>
			{#if recipes.helpers.length === 0}
				<p class="note small">No agent is set up. Add one in Settings › Agents.</p>
			{/if}
		{:else}
			<p class="note small">Off: no agent is called. The run is the git commands below and nothing else.</p>
		{/if}

		<div class="levels" role="radiogroup" aria-label="Level">
			{#each LEVELS as each (each.id)}
				<button
					role="radio"
					class:on={draft.level === each.id}
					aria-checked={draft.level === each.id}
					title={each.label}
					onclick={() => recipes.setLevel(each.id as Level)}>{each.short}</button
				>
			{/each}
		</div>
		<p class="note small">{levelWords(level).says}</p>

		<label class="field">
			<span class="head">
				<span>What should it do?</span>
				<span class="count mono" class:over={recipes.sentence.length > MAX_SENTENCE}
					>{recipes.sentence.length}/{MAX_SENTENCE}</span
				>
			</span>
			<textarea
				rows="3"
				disabled={!on}
				placeholder={on ? 'Stash my work, bring main in, put my work back and push' : 'Agent help is off'}
				value={recipes.sentence}
				oninput={(event) => recipes.setSentence(event.currentTarget.value)}
			></textarea>
		</label>
		<div class="row">
			<Btn
				disabled={!on || !helper || recipes.proposing || recipes.sentence.trim() === '' || recipes.sentence.length > MAX_SENTENCE}
				busy={recipes.proposing}
				title="The agent proposes the stack. Nothing runs until you press Run."
				onclick={() => recipes.propose()}>Propose blocks</Btn
			>
			{#if recipes.said}
				<span class="note small said">{recipes.said}</span>
			{/if}
		</div>

		<label class="field">
			<span class="head">
				<span>Note</span>
				<span class="count mono" class:over={draft.note.length > MAX_NOTE}>{draft.note.length}/{MAX_NOTE}</span>
			</span>
			<textarea
				rows="2"
				disabled={!on}
				placeholder={on ? 'One note for the agent' : 'Agent help is off'}
				value={draft.note}
				oninput={(event) => recipes.setNote(event.currentTarget.value)}
			></textarea>
		</label>

		<div class="head">
			<span>{on ? 'Brief' : 'Commands'}</span>
			{#if on}
				<span class="guide mono" title="The rules every agent is given first. They ship with Spagitty.">
					recipes.md v{recipes.guideVersion}
				</span>
			{/if}
		</div>
		<pre class="brief mono" aria-label={on ? 'The brief, as it is sent' : 'The commands, as a script'}>{(on
				? check?.brief
				: check?.script) ?? ''}</pre>

		<div class="run">
			<span class="note small">A restore point is made first</span>
			<Btn
				primary
				disabled={steps === 0 || errors > 0 || readOnly || recipes.starting || !check}
				busy={recipes.starting}
				title={readOnly ? 'Read-only runs nothing. Copy the script, or choose another level.' : undefined}
				onclick={() => recipes.start()}>{runLabel(steps, errors)}</Btn
			>
		</div>
	</aside>
{/if}

<style>
	.panel {
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

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.row > .label {
		flex: 1;
	}

	.agent select {
		flex: 1;
		min-width: 0;
	}

	.hex {
		display: grid;
		place-items: center;
		color: var(--agent);
	}

	.small {
		margin: 0;
		font-size: var(--fs-secondary);
	}

	.said {
		min-width: 0;
	}

	.levels {
		display: flex;
		padding: 3px;
		border-radius: var(--r-pill);
		background: var(--sunken);
	}

	.levels button {
		flex: 1;
		padding: 5px 6px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	.levels button.on {
		background: var(--surface-2);
		box-shadow: var(--shadow-1);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.count {
		font-size: var(--fs-mono);
	}

	.count.over {
		color: var(--danger);
	}

	textarea {
		resize: none;
	}

	.guide {
		padding: 1px 8px;
		border: 1px solid color-mix(in srgb, var(--agent) 50%, transparent);
		border-radius: var(--r-pill);
		color: var(--agent);
		font-size: var(--fs-mono);
	}

	.brief {
		flex: 1 1 120px;
		min-height: 120px;
		margin: 0;
		padding: 10px;
		border-radius: var(--r-field);
		background: var(--sunken);
		font-size: var(--fs-mono);
		white-space: pre-wrap;
		overflow: auto;
		user-select: text;
	}

	.run {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}
</style>
