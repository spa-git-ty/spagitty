<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { FARM_COPY as C } from '../../describe';
	import type { PastFarm } from '../../types';
	import { pastLine, type WizardState } from '../../wizard';

	/**
	 * Step 1 (FEAT-141): what should be true when it's done. One large field,
	 * the notes for the agents, and the earlier farms here to pick up instead.
	 */
	interface Props {
		wizard: WizardState;
		past: PastFarm[];
		busy: boolean;
		onenter: () => void;
		onpick: (past: PastFarm) => void;
	}

	let { wizard = $bindable(), past, busy, onenter, onpick }: Props = $props();

	const HOW = ['Goal', 'Plan', 'Each task in its own worktree', 'Your checks', 'Another agent reviews'];
</script>

<label class="hidden-label" for="farm-goal">{C.goal}</label>
<!-- svelte-ignore a11y_autofocus -->
<input
	id="farm-goal"
	class="goal"
	autofocus
	placeholder="Gitea support for pull requests"
	bind:value={wizard.title}
	onkeydown={(event) => {
		if (event.key === 'Enter') {
			event.preventDefault();
			onenter();
		}
	}}
/>

<label class="notes">
	<span>{C.goalKnow} <span class="optional">· optional</span></span>
	<textarea rows="3" placeholder="Same Host trait as GitHub and GitLab. Do not touch the GitHub code path." bind:value={wizard.notes}></textarea>
</label>

{#if past.length}
	<div class="past">
		<span class="small">Or pick up a past farm</span>
		<div class="cards">
			{#each past.slice(0, 4) as farm (farm.id)}
				{@const line = pastLine(farm)}
				<button class="card past-card" disabled={busy} onclick={() => onpick(farm)}>
					<span class="top">
						<span style:color={line.colour}>{line.text}</span>
						<span class="muted">{new Date(farm.updatedMs).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}</span>
					</span>
					<span class="title">{farm.goal.title}</span>
					<span class="action">{line.action}</span>
				</button>
			{/each}
		</div>
	</div>
{/if}

<details class="how">
	<summary>How a farm runs</summary>
	<div class="chips">
		{#each HOW as step (step)}<span class="step">{step}</span><span aria-hidden="true">→</span>{/each}
		<span class="step merged">Merged</span>
	</div>
</details>

<style>
	.hidden-label {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
	}

	.goal {
		box-sizing: border-box;
		width: 100%;
		height: 68px;
		padding: 0 22px;
		border-radius: var(--r-floating);
		border: 1.5px solid var(--accent);
		box-shadow: 0 0 0 5px color-mix(in srgb, var(--accent) 14%, transparent);
		font-size: calc(var(--fs-title) * 1.15);
	}

	.notes {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.notes > span,
	.small {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.optional {
		opacity: 0.8;
	}

	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: 14px 18px;
		border-radius: var(--r-floating);
		line-height: 1.5;
		resize: vertical;
	}

	.past {
		display: flex;
		flex-direction: column;
		gap: 10px;
		margin-top: 6px;
	}

	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 12px;
	}

	.past-card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 16px 18px;
		border-radius: var(--r-floating);
		color: var(--ink);
		text-align: left;
		font: inherit;
		cursor: pointer;
	}

	.top {
		display: flex;
		justify-content: space-between;
		font-size: var(--fs-mono);
	}

	.title {
		font-size: var(--fs-ui);
		font-weight: 500;
	}

	.action {
		color: var(--accent);
		font-size: var(--fs-secondary);
	}

	.how {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.how summary {
		cursor: pointer;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin-top: 12px;
	}

	.step {
		padding: 6px 12px;
		border-radius: var(--r-pill);
		border: 1px solid var(--soft);
		color: var(--ink);
	}

	.step.merged {
		border-color: color-mix(in srgb, var(--ok) 50%, transparent);
		color: var(--ok);
	}
</style>
