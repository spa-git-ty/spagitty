<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { NEED_FILTERS, age, counts, filtered, styleOfNeed, summary, type Need, type NeedFilter } from './needs';

	/**
	 * Needs you (FEAT-140): the heart of Home. One row per thing waiting on the
	 * person now, most urgent first; the button does what the kind says.
	 */
	interface Props {
		rows: Need[];
		now: number;
		onact: (need: Need) => void;
	}

	let { rows, now, onact }: Props = $props();

	let filter = $state<NeedFilter>('all');
	const shown = $derived(filtered(rows, filter));
	const sizes = $derived(counts(rows));
</script>

<section class="card home-card needs" aria-labelledby="home-needs">
	<div class="home-card-head">
		<div class="title">
			<h2 id="home-needs">Needs you</h2>
			<span class="note">{summary(rows, now)}</span>
		</div>
		<div class="chips" role="group" aria-label="Show">
			{#each NEED_FILTERS as chip (chip.id)}
				<Chip active={filter === chip.id} onclick={() => (filter = chip.id)}>
					{chip.label}{chip.id === 'all' ? '' : ` ${sizes[chip.id]}`}
				</Chip>
			{/each}
		</div>
	</div>

	{#each shown as need (need.key)}
		{@const style = styleOfNeed(need.kind)}
		<div class="need home-row" style:--kind={style.colour}>
			<span class="tile"><Icon name={style.icon} size="1.15em" /></span>
			<div class="text">
				<div class="line">
					<span class="kind">{style.label}</span>
					<span class="what home-ellipsis">{need.title}</span>
				</div>
				<div class="meta home-ellipsis">{need.meta}</div>
			</div>
			<span class="age">{age(need.since, now)}</span>
			<Btn primary={style.primary} quiet onclick={() => onact(need)}>{style.action}</Btn>
		</div>
	{:else}
		<p class="clear home-row">Nothing is waiting on you{filter === 'all' ? '' : ' here'}.</p>
	{/each}
</section>

<style>
	.needs {
		flex: 2 1 560px;
		padding-bottom: 10px;
	}

	.title {
		display: flex;
		align-items: baseline;
		gap: 10px;
	}

	.title h2 {
		font-size: calc(var(--fs-ui) * 1.15);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.need {
		display: grid;
		grid-template-columns: 40px minmax(0, 1fr) auto auto;
		align-items: center;
		column-gap: 14px;
		padding: 12px 0;
	}

	.tile {
		width: 40px;
		height: 40px;
		display: grid;
		place-items: center;
		border-radius: 12px;
		color: var(--kind);
		background: color-mix(in srgb, var(--kind) 14%, transparent);
	}

	.text {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.line {
		display: flex;
		align-items: baseline;
		gap: 10px;
		min-width: 0;
	}

	.kind {
		flex: none;
		color: var(--kind);
		font-size: var(--fs-mono);
		font-weight: 600;
		white-space: nowrap;
	}

	.what {
		font-size: var(--fs-ui);
		font-weight: 500;
	}

	.meta,
	.age {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.age {
		font-size: var(--fs-mono);
	}

	.clear {
		margin: 0;
		padding: 18px 0;
		color: var(--muted);
	}

	@media (max-width: 700px) {
		.need {
			grid-template-columns: 40px minmax(0, 1fr) auto;
		}
		.age {
			display: none;
		}
	}
</style>
