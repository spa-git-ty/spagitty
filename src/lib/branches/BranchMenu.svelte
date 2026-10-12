<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick } from 'svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { drawn, sections, type BranchOption } from './pick';

	/**
	 * The list a branch is chosen from, wherever one is chosen (FEAT-122): the
	 * toolbar's switcher, Merger's two cards and the Rebase screen's.
	 *
	 * It was the right-click menu with branches in it, and three things about
	 * that were wrong for a list of names. It could not be searched, in a
	 * repository where the branch wanted is one of two hundred. The branch
	 * checked out was the *faintest* row, because the only way a menu can say
	 * "you are here" is to disable the entry. And each row was a name pushed
	 * to the left edge and a note pushed to the right, so a column of them
	 * read as two ragged lists.
	 *
	 * So: a field to type in, focused on opening; the groups under their own
	 * headings with how many each holds; one column of names, the folder part
	 * quieter than the part that tells them apart; and the branch held now
	 * ticked, the one checked out marked `current`, both at full strength.
	 *
	 * The keys are a combobox's: the focus stays in the field, the arrows move
	 * through the rows that can be picked, Enter picks, Escape closes.
	 */
	interface Props {
		x: number;
		y: number;
		/** The control it was opened from. Its second click closes (BUG-018). */
		anchor?: HTMLElement | null;
		/** Named for screen readers. */
		label: string;
		options: BranchOption[];
		/** The name this control holds now. Ticked, and not picked again. */
		value?: string | null;
		/** The branch checked out here. Marked wherever it is listed. */
		current?: string | null;
		/** The names are still being read. */
		loading?: boolean;
		onpick: (name: string) => void;
		onclose: () => void;
	}

	let {
		x,
		y,
		anchor = null,
		label,
		options,
		value = null,
		current = null,
		loading = false,
		onpick,
		onclose
	}: Props = $props();

	const uid = $props.id();

	let element = $state<HTMLDivElement | null>(null);
	let field = $state<HTMLInputElement | null>(null);
	let query = $state('');
	/** Where it sits and how tall it is, measured once the names are in. */
	let placed = $state<{ left: number; top: number; height: number | null } | null>(null);
	/** The row the keys are on: an index into `rows`, or -1. */
	let cursor = $state(-1);

	const groups = $derived(sections(options, query));
	const rows = $derived(groups.flatMap((group) => group.rows));
	const found = $derived(groups.reduce((sum, group) => sum + group.found, 0));

	function pickable(option: BranchOption): boolean {
		return !option.reason && option.name !== value;
	}

	/**
	 * Keep it inside the window, and the size it opened at.
	 *
	 * Measured after mount, like `Menu`. Two things differ. Opened from a
	 * control near the bottom of the window — the toolbar is one — it goes
	 * above the control rather than over it. And its height is then held:
	 * the list gets shorter with every letter typed, and a field that moved
	 * under the fingers typing into it would be worse than the empty space.
	 *
	 * Measured again only when the names themselves arrive or change, never
	 * for a query.
	 */
	$effect(() => {
		void options.length;
		void loading;
		placed = null;
		(async () => {
			await tick();
			if (!element) return;
			// Its laid-out size, not its drawn one: it is still growing in when
			// this runs, and a rectangle read mid-growth is a tenth too small,
			// which is how a menu near the right edge ended up past it.
			const width = element.offsetWidth;
			const height = element.offsetHeight;
			const margin = 8;
			const over = anchor?.getBoundingClientRect() ?? null;
			let top = y;
			if (top + height > window.innerHeight - margin) {
				const above = over ? over.top - 4 - height : -1;
				top = above >= margin ? above : window.innerHeight - height - margin;
			}
			placed = {
				left: Math.max(margin, Math.min(x, window.innerWidth - width - margin)),
				top: Math.max(margin, top),
				height: height > 0 ? height : null
			};
			field?.focus();
			// Open where you are: the branch held may be far down a long list.
			const held = element.querySelector('.entry.chosen') as HTMLElement | null;
			held?.scrollIntoView?.({ block: 'nearest' });
		})();
	});

	/** Typing starts over at the best match; opening starts at the one held. */
	$effect(() => {
		const asked = query;
		const list = rows;
		cursor =
			asked.trim() === ''
				? list.findIndex((option) => option.name === value)
				: list.findIndex(pickable);
	});

	function reveal() {
		void tick().then(() => {
			const at = element?.querySelector('.entry.at') as HTMLElement | null;
			at?.scrollIntoView?.({ block: 'nearest' });
		});
	}

	/** Move over the rows that can be picked, wrapping at either end. */
	function step(delta: number) {
		const usable = rows.map((option, index) => ({ option, index })).filter((row) => pickable(row.option));
		if (usable.length === 0) return;
		const here = usable.findIndex((row) => row.index === cursor);
		let next: number;
		if (here === -1) {
			// From a row that cannot be picked — the one held — to its nearest
			// neighbour in the direction asked.
			const after = usable.findIndex((row) => row.index > cursor);
			next = delta > 0 ? (after === -1 ? 0 : after) : (after === -1 ? usable.length : after) - 1;
			if (next < 0) next = usable.length - 1;
		} else {
			next = (here + delta + usable.length) % usable.length;
		}
		cursor = usable[next].index;
		reveal();
	}

	/**
	 * Close, and hand the focus back to the control it was opened from.
	 *
	 * The control is read before closing: the caller usually closes by
	 * dropping the very state these props are read from.
	 */
	function close() {
		const back = anchor;
		onclose();
		back?.focus();
	}

	function choose(option: BranchOption) {
		if (option.reason) return;
		const changed = option.name !== value;
		const pick = onpick;
		// Close first: picking may open a dialog (see `Menu`).
		close();
		if (changed) pick(option.name);
	}

	function onkeydown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Escape':
				event.preventDefault();
				event.stopPropagation();
				close();
				return;
			case 'ArrowDown':
				event.preventDefault();
				step(1);
				return;
			case 'ArrowUp':
				event.preventDefault();
				step(-1);
				return;
			case 'Enter': {
				event.preventDefault();
				const at = rows[cursor];
				if (at && pickable(at)) choose(at);
				return;
			}
		}
	}

	/** Close when the focus is somewhere else, as `Menu` does. */
	function leftTheMenu(event: FocusEvent) {
		const next = event.relatedTarget as Node | null;
		if (next && element?.contains(next)) return;
		if (next && anchor?.contains(next)) return;
		onclose();
	}
</script>

<svelte:window
	onmousedown={(event) => {
		const target = event.target as Node;
		if (anchor?.contains(target)) return;
		if (element && !element.contains(target)) onclose();
	}}
	onresize={onclose}
/>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={element}
	class="menu branch-menu floating"
	class:measuring={placed === null}
	style="left: {placed?.left ?? x}px; top: {placed?.top ?? y}px;{placed?.height
		? ` height: ${placed.height}px;`
		: ''}"
	{onkeydown}
	onfocusout={leftTheMenu}
>
	<div class="find">
		<Icon name="search" size="1em" weight={2} />
		<input
			bind:this={field}
			bind:value={query}
			class="query"
			type="text"
			role="combobox"
			aria-label={label}
			aria-expanded="true"
			aria-controls="{uid}-list"
			aria-autocomplete="list"
			aria-activedescendant={cursor >= 0 ? `${uid}-${cursor}` : undefined}
			placeholder="Find a branch"
			spellcheck="false"
			autocomplete="off"
			autocapitalize="off"
		/>
		{#if query.trim() !== '' && options.length > 0}
			<span class="count note" aria-live="polite">{found} of {options.length}</span>
		{/if}
	</div>

	<!--
		The focus stays in the field: a press on a row must not take it, or the
		next letter typed would go nowhere.
	-->
	<div
		id="{uid}-list"
		class="list"
		role="listbox"
		aria-label={label}
		tabindex="-1"
		onmousedown={(event) => event.preventDefault()}
	>
		{#each groups as group (group.group)}
			<div class="group" role="group" aria-label={group.heading}>
				<div class="heading note">
					<span>{group.heading}</span>
					<span class="tally">{group.found}</span>
				</div>
				{#each group.rows as option (option.name)}
					{@const name = drawn(option.name, query)}
					{@const index = rows.indexOf(option)}
					<button
						id="{uid}-{index}"
						type="button"
						class="entry"
						class:at={index === cursor}
						class:chosen={option.name === value}
						class:off={!!option.reason}
						role="option"
						tabindex="-1"
						aria-selected={option.name === value}
						aria-disabled={option.reason ? 'true' : undefined}
						title={option.reason ? `${option.name} — ${option.reason}` : option.name}
						onmouseenter={() => {
							if (pickable(option)) cursor = index;
						}}
						onclick={() => choose(option)}
					>
						<span class="tick" aria-hidden="true">
							{#if option.name === value}<Icon name="check" size="0.95em" weight={2.4} />{/if}
						</span>
						<span class="name mono">
							{#if name.dir.length > 0}<span class="dir"
									>{#each name.dir as mark, i (i)}{#if mark.hit}<mark>{mark.text}</mark
											>{:else}{mark.text}{/if}{/each}</span
								>{/if}{#each name.leaf as mark, i (i)}{#if mark.hit}<mark>{mark.text}</mark
									>{:else}{mark.text}{/if}{/each}
						</span>
						{#if option.name === current}<span class="here">current</span>{/if}
						{#if option.reason}<span class="why note">{option.reason}</span>{/if}
					</button>
				{/each}
				{#if group.found > group.rows.length}
					<div class="more note">
						+ {group.found - group.rows.length} more. Type to find them.
					</div>
				{/if}
			</div>
		{:else}
			<div class="empty note" role="status">
				{#if loading}
					Reading branches…
				{:else if options.length === 0}
					No branches
				{:else}
					Nothing matches “{query.trim()}”
				{/if}
			</div>
		{/each}
	</div>
</div>

<style>
	/*
	 * The material is `.floating`, from `app.css`: what floats over the
	 * application shares one surface, and the blur is spent in one place.
	 */
	.menu {
		position: fixed;
		z-index: 50;
		width: min(460px, calc(100vw - 16px));
		max-height: min(440px, calc(100vh - 64px));
		display: flex;
		flex-direction: column;
		outline: none;
		overflow: hidden;
		transform-origin: top left;
		animation: pop-in var(--t-enter-liquid) var(--spring-liquid);
	}

	.measuring {
		visibility: hidden;
	}

	.find {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 9px 12px;
		border-bottom: 1px solid var(--pane-edge);
		color: var(--muted);
	}

	.find :global(svg) {
		flex: none;
	}

	.query {
		flex: 1;
		min-width: 0;
		border: 0;
		outline: none;
		background: transparent;
		color: var(--ink);
		font-size: var(--fs-secondary);
	}

	.query::placeholder {
		color: var(--muted);
	}

	.count {
		flex: none;
		font-size: var(--fs-mono);
		font-variant-numeric: tabular-nums;
	}

	.list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: 5px;
		outline: none;
	}

	/* The heading stays while its group scrolls under it, so a long list
	   always says which kind of name is being read. */
	.heading {
		position: sticky;
		top: -5px;
		z-index: 1;
		display: flex;
		justify-content: space-between;
		gap: 8px;
		padding: 7px 8px 4px;
		font-size: var(--fs-mono);
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		background-color: var(--surface-2);
	}

	.tally {
		font-variant-numeric: tabular-nums;
		letter-spacing: 0;
	}

	/*
	 * One column. The tick has its own fixed slot, so every name starts at
	 * the same place whether or not its row is the one held; what follows the
	 * name sits against it rather than against the far edge.
	 */
	.entry {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 8px;
		border-radius: var(--r-row);
		text-align: left;
		font-size: var(--fs-secondary);
		color: var(--ink);
	}

	.entry:hover:not(.off):not(.chosen) {
		background: var(--hover);
	}

	.entry.at:not(.off):not(.chosen) {
		background: var(--selection);
	}

	.tick {
		flex: none;
		width: 1em;
		display: inline-flex;
		justify-content: center;
		color: var(--accent);
	}

	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.dir {
		color: var(--muted);
	}

	/* What was typed, wherever it was found in the name. */
	mark {
		background: color-mix(in srgb, var(--accent) 24%, transparent);
		color: inherit;
		border-radius: 3px;
	}

	/* The one held: the accent, and heavier. Never fainter than its
	   neighbours, which is what it was. */
	.chosen {
		cursor: default;
		background: var(--accent-soft);
	}

	.chosen .name {
		font-weight: 650;
		color: var(--accent);
	}

	.chosen .dir {
		color: color-mix(in srgb, var(--accent) 70%, var(--muted));
	}

	.here {
		flex: none;
		padding: 1px 8px;
		border: 1px solid color-mix(in srgb, var(--accent) 55%, transparent);
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		font-weight: 600;
		color: var(--accent);
	}

	/* Listed, with its reason, and not to be picked. */
	.off {
		cursor: default;
	}

	.off .name {
		opacity: 0.5;
	}

	.why {
		flex: none;
		margin-left: auto;
		font-size: var(--fs-mono);
	}

	.more,
	.empty {
		padding: 6px 8px 6px calc(1em + 16px);
		font-size: var(--fs-mono);
	}

	.empty {
		padding: 14px 12px;
		text-align: center;
		font-size: var(--fs-secondary);
	}
</style>
