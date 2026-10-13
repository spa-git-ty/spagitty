<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import NavRail from '$lib/chrome/NavRail.svelte';
	import { delight } from '$lib/delight/store.svelte';
	import {
		GROUP_LABELS,
		hiddenItems,
		inOrder,
		isHidden,
		moveBy,
		moveOnto,
		NAV_ITEMS,
		railChoice,
		RAIL_DEFAULT,
		RULE_LABELS,
		type NavGroup,
		type NavItem,
		type RailSettings,
		type RailWhere,
		type ScreenCode
	} from '$lib/nav';
	import { alt, shortcut } from '$lib/platform';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { settings } from './store.svelte';

	/**
	 * Pin or hide what is on the rail (FEAT-134).
	 *
	 * 3.0 adds four rows to a rail TASK-056 had already filled, and only the
	 * person using it knows which rows they never touch. Every screen has a row
	 * here, in its group, and is pinned or hidden; hidden screens go under the
	 * rail's More button or to the palette only. Nothing is hidden until
	 * somebody hides it.
	 *
	 * Three kinds of row do not offer the choice, and each says why in place of
	 * it: Farm and Settings read *Always* — Settings has to stay reachable to
	 * undo a hide — and Conflicts and Badges show the rule they already keep.
	 *
	 * The preview is the rail itself, drawn from what is chosen here, so there
	 * is no second drawing to fall out of step with it.
	 */
	const rail = $derived(settings.rail);

	/** Badges is only offered while the delight layer is on (TASK-045). */
	const items = $derived(
		inOrder(NAV_ITEMS, rail.order).filter((item) => !item.delight || delight.on)
	);

	/** Two columns, as the day goes: the work on the left, the rest beside it. */
	const COLUMNS: NavGroup[][] = [
		['farm', 'work'],
		['tools', 'app']
	];

	function members(group: NavGroup): NavItem[] {
		return items.filter((item) => item.group === group);
	}

	/** `farm` has no heading on the rail, being one row. Here every card has one. */
	function nameOf(group: NavGroup): string {
		return GROUP_LABELS[group] ?? members(group)[0]?.label ?? '';
	}

	const hidden = $derived(
		hiddenItems(NAV_ITEMS, { pathname: '', conflicts: null, delight: delight.on, rail })
	);
	/** What is on the rail whatever the day brings: not the two that keep a rule. */
	const pinned = $derived(
		items.filter((item) => railChoice(item) !== 'rule' && !isHidden(item, rail)).length
	);

	const isDefault = $derived(
		rail.hidden.length === 0 && rail.order.length === 0 && rail.where === RAIL_DEFAULT.where
	);

	function write(next: Partial<RailSettings>) {
		void settings.choose('rail', { ...rail, ...next });
	}

	function setHidden(code: ScreenCode, hide: boolean) {
		if (rail.hidden.includes(code) === hide) return;
		write({
			hidden: hide ? [...rail.hidden, code] : rail.hidden.filter((other) => other !== code)
		});
	}

	function setWhere(where: RailWhere) {
		if (rail.where !== where) write({ where });
	}

	function reorder(order: ScreenCode[]) {
		if (order !== rail.order) write({ order });
	}

	let dragging = $state<ScreenCode | null>(null);
	let over = $state<ScreenCode | null>(null);

	/** A row can be dropped on only by a row of its own group. */
	function accepts(target: NavItem): boolean {
		const moving = NAV_ITEMS.find((item) => item.code === dragging);
		return moving !== undefined && moving.code !== target.code && moving.group === target.group;
	}

	function drop(target: NavItem) {
		const moving = dragging;
		dragging = null;
		over = null;
		if (moving === null || moving === target.code) return;
		reorder(moveOnto(moving, target.code, rail.order));
	}

	function keydown(item: NavItem, event: KeyboardEvent) {
		if (!event.altKey) return;
		if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
		event.preventDefault();
		reorder(moveBy(item.code, event.key === 'ArrowUp' ? -1 : 1, rail.order));
	}
</script>

<section class="section" aria-labelledby="rail-heading">
	<div class="main">
		<header class="top">
			<h2 class="heading" id="rail-heading">Rail</h2>
			<span class="note small">{pinned} pinned · {hidden.length} hidden</span>
			<Btn disabled={isDefault} onclick={() => write(RAIL_DEFAULT)}>Reset</Btn>
		</header>

		<div class="columns">
			{#each COLUMNS as column (column[0])}
				<div class="column">
					{#each column as group (group)}
						{@const rows = members(group)}
						<section class="card" aria-label={nameOf(group)}>
							<span class="note small name">{nameOf(group)}</span>
							{#each rows as item (item.code)}
								{@const choice = railChoice(item)}
								{@const off = isHidden(item, rail)}
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="row"
									class:off
									class:over={over === item.code && accepts(item)}
									class:dragging={dragging === item.code}
									data-code={item.code}
									draggable={rows.length > 1}
									ondragstart={() => (dragging = item.code)}
									ondragover={(event) => {
										if (!accepts(item)) return;
										event.preventDefault();
										over = item.code;
									}}
									ondragleave={() => {
										if (over === item.code) over = null;
									}}
									ondrop={() => drop(item)}
									ondragend={() => {
										dragging = null;
										over = null;
									}}
									onkeydown={(event) => keydown(item, event)}
								>
									{#if rows.length > 1}
										<button
											class="grip"
											aria-label="Move {item.label}"
											title="Drag to reorder, or {alt()}↑ / {alt()}↓"
										>
											<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
												<circle cx="9" cy="6" r="1.6" /><circle cx="15" cy="6" r="1.6" />
												<circle cx="9" cy="12" r="1.6" /><circle cx="15" cy="12" r="1.6" />
												<circle cx="9" cy="18" r="1.6" /><circle cx="15" cy="18" r="1.6" />
											</svg>
										</button>
									{:else}
										<span class="grip" aria-hidden="true"></span>
									{/if}
									<Icon name={item.icon} size={17} />
									<span class="label">{item.label}</span>
									{#if item.since}<span class="new">{item.since}</span>{/if}
									<span class="code mono">{item.code}</span>
									<span class="gap"></span>
									{#if choice === 'always'}
										<span class="note small fixed"><Icon name="lock" size={12} weight={2} />Always</span>
									{:else if choice === 'rule'}
										<span class="note small">{RULE_LABELS[item.shows === 'open' ? 'open' : 'conflicts']}</span>
									{:else}
										<div class="segmented" role="radiogroup" aria-label="{item.label} on the rail">
											<button
												role="radio"
												aria-checked={!off}
												class:on={!off}
												onclick={() => setHidden(item.code, false)}>Pin</button
											>
											<button
												role="radio"
												aria-checked={off}
												class:on={off}
												onclick={() => setHidden(item.code, true)}>Hide</button
											>
										</div>
									{/if}
								</div>
							{/each}
						</section>
					{/each}
				</div>
			{/each}
		</div>

		<section class="where" aria-label="Hidden screens">
			<span class="note small">Hidden screens</span>
			<div class="segmented wide" role="radiogroup" aria-label="Where hidden screens go">
				<button
					role="radio"
					aria-checked={rail.where === 'more'}
					class:on={rail.where === 'more'}
					onclick={() => setWhere('more')}>Under ⋯ More</button
				>
				<button
					role="radio"
					aria-checked={rail.where === 'palette'}
					class:on={rail.where === 'palette'}
					onclick={() => setWhere('palette')}>Palette only</button
				>
			</div>
			<span class="note small hints">
				<span>drag or <kbd class="kbd">{alt()}↑↓</kbd> to reorder</span>
				<span><kbd class="kbd">{shortcut('P')}</kbd> reaches every screen</span>
			</span>
		</section>
	</div>

	<aside class="preview" aria-label="Preview">
		<span class="note small">Preview</span>
		<div class="stage">
			<NavRail {rail} preview />
			{#if rail.where === 'more' && hidden.length > 0}
				<ul class="under ornament" aria-label="Under More">
					{#each hidden as item (item.code)}
						<li><Icon name={item.icon} size={15} /><span>{item.label}</span></li>
					{/each}
				</ul>
			{/if}
		</div>
	</aside>
</section>

<style>
	/* Wraps: in a narrow window the preview goes under the lists rather than
	   squeezing them. */
	.section {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: 14px;
	}

	.main {
		flex: 1;
		min-width: min(100%, 560px);
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

	.columns {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 18px;
	}

	.column {
		display: flex;
		flex-direction: column;
		gap: 14px;
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

	.name {
		padding: 2px 6px 6px;
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

	/* A hidden row is still a row: it is only said more quietly. */
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
	}

	.label {
		font-size: var(--fs-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.code {
		color: var(--muted);
	}

	/* New in this version. The agent's colour is not borrowed for it: a muted
	   outline says "recent" without saying "an agent did this". */
	.new {
		flex: none;
		padding: 0 6px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		font-size: calc(var(--fs-mono) * 0.875);
		color: var(--muted);
	}

	.gap {
		flex: 1;
	}

	.fixed {
		display: inline-flex;
		align-items: center;
		gap: 5px;
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

	.segmented.wide {
		align-self: flex-start;
		padding: 3px;
	}

	.segmented.wide button {
		height: 30px;
		padding: 0 14px;
	}

	.where {
		display: flex;
		flex-direction: column;
		gap: 10px;
		max-width: 640px;
	}

	.hints {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
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
		width: 250px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-panel);
	}

	.stage {
		display: flex;
		align-items: flex-end;
		gap: 12px;
	}

	.under {
		flex: 1;
		min-width: 0;
		margin: 0 0 48px;
		padding: 6px;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 1px;
		border-radius: var(--r-panel);
	}

	.under li {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 30px;
		padding: 0 8px;
		font-size: var(--fs-mono);
		white-space: nowrap;
		overflow: hidden;
	}

	.under li :global(.icon) {
		color: var(--muted);
	}
</style>
