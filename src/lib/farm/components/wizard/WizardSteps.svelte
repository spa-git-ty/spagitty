<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import { STEPS, type Step } from '../../wizard';

	/**
	 * The wizard's four steps (FEAT-141), in the header where the phase tracker
	 * is once the farm exists. Done steps are ticked in `--ok`, the current one
	 * ringed in the accent.
	 */
	let { current }: { current: Step } = $props();
</script>

<ol class="steps" aria-label="Starting a farm">
	{#each STEPS as label, i (label)}
		{@const n = i + 1}
		<li class:done={n < current} class:current={n === current} aria-current={n === current ? 'step' : undefined}>
			{#if i > 0}<span class="bar" class:lit={n <= current} aria-hidden="true"></span>{/if}
			<span class="marker">{#if n < current}<Icon name="check" size="0.9em" weight={2.6} />{:else}{n}{/if}</span>
			<span class="label">{label}</span>
		</li>
	{/each}
</ol>

<style>
	.steps {
		display: flex;
		align-items: center;
		gap: 10px;
		list-style: none;
		margin: 0;
		padding: 0;
		font-size: var(--fs-secondary);
	}

	li {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--muted);
		white-space: nowrap;
	}

	.bar {
		display: block;
		width: 28px;
		height: 2px;
		border-radius: 1px;
		background: var(--soft);
	}

	.bar.lit {
		background: var(--ok);
	}

	/*
	 * `marker`, not `dot`: farm.css's `.farm-journey .dot` is the 7px status
	 * dot, and it won over this one, so each step's number spilled out under a
	 * dot too small to hold it (BUG-093).
	 */
	.marker {
		width: 28px;
		height: 28px;
		box-sizing: border-box;
		display: grid;
		place-items: center;
		border-radius: 50%;
		border: 1.5px solid color-mix(in srgb, var(--ink) 30%, transparent);
		font-size: var(--fs-mono);
		font-weight: 600;
	}

	.done {
		color: var(--ink);
	}

	.done .marker {
		color: var(--ok);
		border: 2px solid var(--ok);
	}

	.current {
		color: var(--ink);
		font-weight: 600;
	}

	.current .marker {
		color: var(--accent);
		border: 2px solid var(--accent);
		box-shadow: 0 0 0 5px color-mix(in srgb, var(--accent) 18%, transparent);
	}

	@media (max-width: 1100px) {
		.label {
			display: none;
		}
		.current .label {
			display: inline;
		}
	}
</style>
