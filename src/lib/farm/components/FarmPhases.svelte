<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { PHASES } from '../describe';
	import Icon from '$lib/ui/Icon.svelte';
	let { current, subtitles = [] }: { current: number; subtitles?: string[] } = $props();
</script>

<ol class="farm-phases" aria-label="Farm phases">
	{#each PHASES as name, i}
		<li
			class:done={i < current}
			class:current={i === current}
			aria-current={i === current ? 'step' : undefined}
		>
			<span class="phase-dot"
				>{#if i < current}<Icon name="check" />{:else}{i + 1}{/if}</span
			>
			<span
				>{name}{#if subtitles[i]}<small>{subtitles[i]}</small>{/if}</span
			>
		</li>
	{/each}
</ol>

<style>
	.farm-phases {
		display: flex;
		align-items: center;
		gap: 8px;
		list-style: none;
		margin: 0;
		padding: 0;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}
	li {
		display: flex;
		align-items: center;
		gap: 7px;
		position: relative;
		white-space: nowrap;
	}
	li + li:before {
		content: '';
		width: 22px;
		height: 2px;
		margin-right: 2px;
		background: var(--soft);
	}
	li.done + li:before {
		background: color-mix(in srgb, var(--ok) 55%, transparent);
	}
	.phase-dot {
		width: 24px;
		height: 24px;
		display: grid;
		place-items: center;
		border: 1px solid var(--soft);
		border-radius: 50%;
		font-size: var(--fs-mono);
	}
	.done .phase-dot {
		color: var(--ok);
		border-color: var(--ok);
		background: color-mix(in srgb, var(--ok) 10%, transparent);
	}
	.current {
		color: var(--ink);
		font-weight: 600;
	}
	.current .phase-dot {
		color: var(--accent);
		border-color: var(--accent);
		background: var(--accent-soft);
		animation: breathe 3s ease-in-out infinite;
	}
	/* Scoped: the Loader's own `breathe` is not visible from here. */
	@keyframes breathe {
		0%,
		100% {
			box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 0%, transparent);
		}
		50% {
			box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
		}
	}
	small {
		display: block;
		max-width: 110px;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--muted);
		font-weight: 400;
	}
	@media (max-width: 1100px) {
		small {
			display: none;
		}
		li + li:before {
			width: 10px;
		}
		.farm-phases {
			gap: 5px;
		}
	}
	@media (max-width: 750px) {
		li > span:last-child {
			display: none;
		}
	}
</style>
