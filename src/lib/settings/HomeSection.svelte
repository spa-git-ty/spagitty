<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import {
		HOME_DEFAULT,
		inHomeOrder,
		moveCardBy,
		moveCardOnto,
		shownCards,
		type HomeLayout
	} from '$lib/home/layout';
	import { alt } from '$lib/platform';
	import Btn from '$lib/ui/Btn.svelte';
	import { sortable, type SortState } from '$lib/ui/sortable';
	import { settings } from './store.svelte';

	/**
	 * Arrange Home's cards (FEAT-148).
	 *
	 * The rail's section for Home: every card is a row, shown or hidden, moved
	 * by its grip or with Alt and the arrows. The preview is the order Home
	 * draws them in, the wide card taking two columns as it does there.
	 */
	const layout = $derived(settings.home);
	const cards = $derived(inHomeOrder(layout.order));
	const shown = $derived(shownCards(layout));
	const isDefault = $derived(layout.hidden.length === 0 && layout.order.length === 0);

	function write(next: Partial<HomeLayout>) {
		void settings.choose('home', { ...layout, ...next });
	}

	function setHidden(id: string, hide: boolean) {
		if (layout.hidden.includes(id) === hide) return;
		write({ hidden: hide ? [...layout.hidden, id] : layout.hidden.filter((other) => other !== id) });
	}

	function reorder(order: string[]) {
		if (order !== layout.order) write({ order });
	}

	let dragging = $state<string | null>(null);
	let over = $state<string | null>(null);

	const sorting = $derived({
		onmove: (from: string, onto: string) => reorder(moveCardOnto(from, onto, layout.order)),
		onstate: (state: SortState) => {
			dragging = state.dragging;
			over = state.over;
		}
	});

	function keydown(id: string, event: KeyboardEvent) {
		if (!event.altKey) return;
		if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
		event.preventDefault();
		reorder(moveCardBy(id, event.key === 'ArrowUp' ? -1 : 1, layout.order));
	}
</script>

<section class="section" aria-labelledby="home-heading">
	<div class="main">
		<header class="top">
			<h2 class="heading" id="home-heading">Home</h2>
			<span class="note small">{shown.length} shown · {layout.hidden.length} hidden</span>
			<Btn disabled={isDefault} onclick={() => write(HOME_DEFAULT)}>Reset</Btn>
		</header>

		<section class="card" aria-label="Home cards" use:sortable={sorting}>
			{#each cards as card (card.id)}
				{@const off = layout.hidden.includes(card.id)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="row"
					class:off
					class:over={over === card.id}
					class:dragging={dragging === card.id}
					data-card={card.id}
					data-sort-key={card.id}
					onkeydown={(event) => keydown(card.id, event)}
				>
					<button
						class="grip"
						data-sort-grip
						aria-label="Move {card.label}"
						title="Drag to reorder, or {alt()}↑ / {alt()}↓"
					>
						<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
							<circle cx="9" cy="6" r="1.6" /><circle cx="15" cy="6" r="1.6" />
							<circle cx="9" cy="12" r="1.6" /><circle cx="15" cy="12" r="1.6" />
							<circle cx="9" cy="18" r="1.6" /><circle cx="15" cy="18" r="1.6" />
						</svg>
					</button>
					<span class="label">{card.label}</span>
					{#if card.wide}<span class="note small">wide</span>{/if}
					<span class="gap"></span>
					<div class="segmented" role="radiogroup" aria-label="{card.label} on Home">
						<button role="radio" aria-checked={!off} class:on={!off} onclick={() => setHidden(card.id, false)}
							>Show</button
						>
						<button role="radio" aria-checked={off} class:on={off} onclick={() => setHidden(card.id, true)}
							>Hide</button
						>
					</div>
				</div>
			{/each}
		</section>

		<span class="note small hints">drag or <kbd class="kbd">{alt()}↑↓</kbd> to reorder</span>
	</div>

	<aside class="preview" aria-label="Preview">
		<span class="note small">Preview</span>
		<div class="stage">
			{#each shown as card (card.id)}
				<span class="tile" class:wide={card.wide}>{card.label}</span>
			{:else}
				<span class="note small">Home shows only its heading.</span>
			{/each}
		</div>
	</aside>
</section>

<style>
	.section {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: 14px;
	}

	.main {
		flex: 1;
		min-width: min(100%, 420px);
		max-width: 640px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.top {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.heading {
		flex: 1;
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	.row {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 38px;
		padding: 0 8px 0 4px;
		border-radius: var(--r-button);
	}

	.row:hover {
		background: var(--hover);
	}

	.row.off > :not(.segmented) {
		opacity: 0.55;
	}

	.row.dragging {
		opacity: 0.5;
	}

	.row.over {
		box-shadow: inset 0 2px 0 var(--accent);
	}

	.grip {
		flex: none;
		width: 16px;
		height: 24px;
		display: grid;
		place-items: center;
		color: var(--muted);
		border-radius: var(--r-row);
		cursor: grab;
		touch-action: none;
	}

	.label {
		font-size: var(--fs-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.gap {
		flex: 1;
	}

	.segmented {
		flex: none;
		display: inline-flex;
		gap: 2px;
		padding: 2px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-pill);
		background: var(--sunken);
	}

	.segmented button {
		height: 26px;
		padding: 0 10px;
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.segmented button.on {
		background: var(--surface-2);
		box-shadow: var(--shadow-1);
		color: var(--ink);
		font-weight: 600;
	}

	.hints {
		color: var(--muted);
	}

	.kbd {
		padding: 0 5px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-row);
		background: var(--sunken);
		font-family: var(--font-mono);
		font-size: calc(var(--fs-mono) * 0.875);
	}

	.preview {
		flex: none;
		width: 300px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-panel);
	}

	/* Home's grid at three columns, dense, as Home itself packs it. */
	.stage {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		grid-auto-flow: row dense;
		gap: 6px;
	}

	.tile {
		min-height: 34px;
		padding: 6px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-row);
		background: var(--sunken);
		font-size: calc(var(--fs-mono) * 0.875);
		color: var(--muted);
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.tile.wide {
		grid-column: span 2;
		grid-row: span 2;
	}
</style>
