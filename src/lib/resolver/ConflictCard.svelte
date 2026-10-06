<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Token } from '$lib/diff/highlight';
	import Icon from '$lib/ui/Icon.svelte';
	import SideBadge from '$lib/merger/SideBadge.svelte';
	import {
		badgesOf,
		chosenLines,
		editSeed,
		kept,
		lineOf,
		offered,
		pickAll,
		placeholder,
		statusLabel,
		toggled,
		why,
		type Choice,
		type Mode,
		type Names,
		type Numbered,
		type Region,
		type RegionLayout,
		type ResolverFile,
		type SideKey
	} from './model';
	import { paint } from './paint';

	/**
	 * One conflict, in three columns (FEAT-102): A | Result | B, always in that
	 * order whatever the direction. Each side shows its own lines by its own
	 * numbers, with the context around them dimmed; the Result shows what
	 * will land, each line badged with where it came from, numbered as the
	 * file will be. Under it, every way out.
	 */
	interface Props {
		file: ResolverFile;
		region: Region;
		place: RegionLayout;
		choice: Choice | null;
		/** Its number across every file, from 1. */
		number: number;
		names: Names;
		language: string;
		showBase: boolean;
		baseShort: string | null;
		current: boolean;
		onchoose: (choice: Choice | null) => void;
		onfocus: () => void;
	}

	let { file, region, place, choice, number, names, language, showBase, baseShort, current, onchoose, onfocus }: Props =
		$props();

	const lines = $derived(chosenLines(region, choice));
	const editing = $derived(choice?.mode === 'edit' ? choice : null);
	const picking = $derived(choice?.mode === 'pick' ? choice : null);
	const name = $derived(file.path.slice(file.path.lastIndexOf('/') + 1));

	/** Each column's lines, painted as one block so open strings carry. */
	function painted(before: Numbered[], block: string[], after: Numbered[]): Token[][] {
		return paint([...before.map((l) => l.text), ...block, ...after.map((l) => l.text)], language);
	}
	const aTokens = $derived(painted(place.aBefore, region.a, place.aAfter));
	const bTokens = $derived(painted(place.bBefore, region.b, place.bAfter));
	const resultTokens = $derived(painted(place.before, (lines ?? []).map((l) => l.text), place.after));
	const baseTokens = $derived(paint(region.base ?? [], language));

	const CHOICES: { mode: Mode; label: (n: Names) => string; aria: (n: Names) => string }[] = [
		{ mode: 'a', label: (n) => `Take ${n.a}`, aria: (n) => `Take ${n.a}` },
		{ mode: 'b', label: (n) => `Take ${n.b}`, aria: (n) => `Take ${n.b}` },
		{ mode: 'ab', label: () => 'Both', aria: (n) => `Both, ${n.a} first` },
		{ mode: 'ba', label: () => 'Both', aria: (n) => `Both, ${n.b} first` },
		{ mode: 'pick', label: () => 'Pick lines', aria: () => 'Pick lines from each side' },
		{ mode: 'edit', label: () => 'Edit by hand', aria: () => 'Edit the result by hand' }
	];
	const available = $derived(CHOICES.filter((entry) => offered(file).includes(entry.mode)));

	function pick(mode: Mode) {
		if (choice?.mode === mode) return;
		if (mode === 'pick') onchoose(pickAll(region));
		else if (mode === 'edit') onchoose(editSeed(region, choice));
		else onchoose({ mode } as Choice);
	}

	function tick(side: SideKey, i: number) {
		if (picking) onchoose(toggled(picking, side, i));
	}

	function sideNote(side: SideKey): string | null {
		const exists = side === 'a' ? file.aExists : file.bExists;
		if (!exists) return `Deleted on ${names[side]}`;
		if (file.opaque) return 'Binary, or too large to show';
		return null;
	}
</script>

<article
	class="card conflict"
	class:current
	class:unresolved={!choice}
	id="conflict-{file.path}-{region.index}"
	aria-label="Conflict {number}"
>
	<header class="head">
		<button class="number" onclick={onfocus}>Conflict {number}</button>
		<span class="where mono">{name}:{lineOf(region, place)}</span>
		<span class="status" class:resolved={choice !== null}>{statusLabel(choice, names)}</span>
		<span class="why note">{why(region, file, names)}</span>
		{#if choice}
			<button class="chip reset" onclick={() => onchoose(null)}>
				<Icon name="undo" size="0.85em" weight={2.2} />Reset
			</button>
		{/if}
	</header>

	{#if showBase && !file.whole}
		<div class="base">
			<span class="note small">Base{baseShort ? ` · ${baseShort}` : ''} · how it looked before either branch</span>
			{#if region.base === null}
				<span class="note small">The markers do not carry the base.</span>
			{:else if region.base.length === 0}
				<span class="code muted">(nothing: both sides added lines here)</span>
			{:else}
				{#each baseTokens as tokens, i (i)}
					<span class="code base-line">{#each tokens as token, t (t)}<span class="tok-{token.type}">{token.text}</span>{/each}</span>
				{/each}
			{/if}
		</div>
	{/if}

	<div class="columns">
		{#snippet side(key: SideKey, before: Numbered[], block: string[], after: Numbered[], tokens: Token[][], start: number | null)}
			<div class="column code side-{key}">
				{#if sideNote(key)}
					<span class="note small pad">{sideNote(key)}</span>
				{:else}
					{#each before as line, i (i)}
						<div class="row context">
							<span class="mark"></span><span class="n">{line.n ?? ''}</span>
							<span class="text">{#each tokens[i] as token, t (t)}<span class="tok-{token.type}">{token.text}</span>{/each}</span>
						</div>
					{/each}
					{#each block as text, i (i)}
						{@const keep = kept(choice, key, i)}
						<div class="row own {key}" class:dropped={keep === false}>
							<span class="mark"></span>
							<span class="n">{start === null ? '' : start + i}</span>
							{#if picking}
								<input
									type="checkbox"
									class="tick {key}"
									checked={keep === true}
									aria-label="Keep {names[key]} line {start === null ? i + 1 : start + i}"
									onchange={() => tick(key, i)}
								/>
							{/if}
							<span class="text">{#if text === ''}&nbsp;{/if}{#each tokens[before.length + i] as token, t (t)}<span class="tok-{token.type}">{token.text}</span>{/each}</span>
						</div>
					{/each}
					{#each after as line, i (i)}
						<div class="row context">
							<span class="mark"></span><span class="n">{line.n ?? ''}</span>
							<span class="text">{#each tokens[before.length + block.length + i] as token, t (t)}<span class="tok-{token.type}">{token.text}</span>{/each}</span>
						</div>
					{/each}
				{/if}
			</div>
		{/snippet}

		{@render side('a', place.aBefore, region.a, place.aAfter, aTokens, region.aLine)}

		<div class="column code result" class:chosen={choice !== null}>
			{#each place.before as line, i (i)}
				<div class="row context">
					<span class="mark"></span><span class="n">{line.n ?? ''}</span>
					<span class="text">{#each resultTokens[i] as token, t (t)}<span class="tok-{token.type}">{token.text}</span>{/each}</span>
				</div>
			{/each}
			{#if editing}
				<div class="edit">
					<label class="note small mine-note" for="edit-{file.path}-{region.index}">Edit freely. This text is what lands.</label>
					<textarea
						id="edit-{file.path}-{region.index}"
						spellcheck="false"
						rows={Math.max(3, editing.text.split('\n').length + 1)}
						value={editing.text}
						oninput={(event) => onchoose({ mode: 'edit', text: (event.currentTarget as HTMLTextAreaElement).value })}
					></textarea>
				</div>
			{:else if !lines}
				<div class="choose">
					<span class="choose-title">Choose what lands here</span>
					<span class="note small">{placeholder(region, names)}</span>
				</div>
			{:else if lines.length === 0}
				<div class="empty note small">Nothing kept. This region will be empty.</div>
			{:else}
				{#each lines as line, i (i)}
					<div class="row own {line.from}">
						<span class="mark"></span>
						<span class="n">{place.resultStart + i}</span>
						<span class="text">{#if line.text === ''}&nbsp;{/if}{#each resultTokens[place.before.length + i] as token, t (t)}<span class="tok-{token.type}">{token.text}</span>{/each}</span>
						<SideBadge side={line.from} small />
					</div>
				{/each}
			{/if}
			{#each place.after as line, i (i)}
				<div class="row context">
					<span class="mark"></span><span class="n">{line.n ?? ''}</span>
					<span class="text">{#each resultTokens[place.before.length + (lines?.length ?? 0) + i] ?? [] as token, t (t)}<span class="tok-{token.type}">{token.text}</span>{/each}</span>
				</div>
			{/each}
		</div>

		{@render side('b', place.bBefore, region.b, place.bAfter, bTokens, region.bLine)}
	</div>

	<div class="choices" role="group" aria-label="Resolve conflict {number}">
		{#each available as entry (entry.mode)}
			<button
				class="chip choice"
				class:mine={entry.mode === 'edit'}
				aria-pressed={choice?.mode === entry.mode}
				aria-label={entry.aria(names)}
				title={entry.aria(names)}
				onclick={() => pick(entry.mode)}
			>
				{#each badgesOf({ mode: entry.mode } as Choice) as badge, i (i)}<SideBadge side={badge} />{/each}
				{entry.label(names)}
			</button>
		{/each}
	</div>
	{#if picking}
		<span class="note small center">Tick the lines to keep on either side. {names.a}'s lines come first; switch to Edit by hand to reorder or reword.</span>
	{/if}
	{#if region.aFrom || region.bFrom}
		<div class="from note small">
			<span>{region.aFrom ? `from ${region.aFrom}` : ''}</span>
			<span>{region.bFrom ? `from ${region.bFrom}` : ''}</span>
		</div>
	{/if}
</article>

<style>
	.conflict {
		padding: 10px 12px 12px;
		border-radius: 16px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		scroll-margin: 12px;
	}

	.conflict.unresolved {
		border-color: color-mix(in srgb, var(--danger) 35%, transparent);
	}

	.conflict.current {
		border: 1.5px solid var(--accent);
		box-shadow:
			0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent),
			var(--shadow-1);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}

	.number {
		font-weight: 650;
		font-size: var(--fs-secondary);
	}

	.where {
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.status {
		border: 1px solid color-mix(in srgb, var(--danger) 45%, transparent);
		border-radius: var(--r-pill);
		padding: 2px 10px;
		font-size: var(--fs-mono);
		color: var(--danger);
		font-weight: 600;
		white-space: nowrap;
	}

	.status.resolved {
		color: var(--ok);
		border-color: color-mix(in srgb, var(--ok) 45%, transparent);
		font-weight: 400;
	}

	.why {
		flex: 1 1 260px;
		min-width: 0;
		font-size: var(--fs-mono);
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 3px 10px;
		font-size: var(--fs-mono);
		white-space: nowrap;
		background-color: var(--surface-veil);
	}

	.chip:hover {
		background-color: var(--hover);
	}

	.base {
		border: 1px dashed var(--line);
		border-radius: 10px;
		padding: 6px 8px;
		background: color-mix(in srgb, var(--sunken) 60%, transparent);
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.base-line {
		white-space: pre-wrap;
		tab-size: 4;
		padding-left: 39px;
		opacity: 0.8;
	}

	.columns {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr) minmax(0, 1fr);
		gap: 10px;
		align-items: start;
	}

	.code {
		font-family: var(--code-font);
		font-size: var(--fs-code);
		line-height: var(--code-lh);
		letter-spacing: var(--code-ls);
	}

	.column {
		border-radius: 10px;
		background: color-mix(in srgb, var(--sunken) 45%, transparent);
		padding: 4px 0;
		overflow: hidden;
		min-width: 0;
	}

	.result.chosen {
		background: color-mix(in srgb, var(--ok) 5%, var(--surface-2));
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ok) 30%, transparent);
	}

	.row {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		padding: 0 8px 0 0;
	}

	.row.context {
		opacity: 0.72;
	}

	.mark {
		width: 3px;
		align-self: stretch;
		flex: none;
	}

	.n {
		width: 3ch;
		flex: none;
		text-align: right;
		color: var(--muted);
		opacity: 0.75;
		font-variant-numeric: tabular-nums;
	}

	.text {
		white-space: pre-wrap;
		word-break: break-word;
		tab-size: 4;
		flex: 1;
		min-width: 0;
	}

	.own.a {
		background: var(--side-a-soft);
	}

	.own.a .mark {
		background: var(--side-a);
	}

	.own.b {
		background: var(--side-b-soft);
	}

	.own.b .mark {
		background: var(--side-b);
	}

	.own.mine {
		background: var(--side-mine-soft);
	}

	.own.mine .mark {
		background: var(--side-mine);
	}

	/*
	 * A side's own colour, kept off its own rows (the author's choice,
	 * 2026-10-06): function names are A's blue, and numbers and attributes are
	 * B's amber, so on that side's tinted rows they step towards the ink and
	 * differ in lightness rather than vanishing into the tint.
	 */
	.own.a :global(.tok-fn) {
		color: color-mix(in srgb, var(--lane-5) 45%, var(--ink));
	}

	.own.b :global(.tok-number),
	.own.b :global(.tok-attr) {
		color: color-mix(in srgb, var(--lane-3) 45%, var(--ink));
	}

	.own.dropped {
		opacity: 0.4;
	}

	.tick {
		margin: 4px 0 0;
		width: 14px;
		height: 14px;
		flex: none;
	}

	.tick.a {
		accent-color: var(--side-a);
	}

	.tick.b {
		accent-color: var(--side-b);
	}

	.choose {
		margin: 4px 8px 4px 11px;
		padding: 12px;
		border-radius: 10px;
		border: 1.5px dashed color-mix(in srgb, var(--danger) 55%, transparent);
		background: color-mix(in srgb, var(--danger) 10%, transparent);
		display: flex;
		flex-direction: column;
		gap: 3px;
		font-family: var(--font-ui);
	}

	.choose-title {
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--danger);
	}

	.empty {
		margin: 4px 8px 4px 11px;
		padding: 8px 10px;
		border-radius: 8px;
		border: 1px dashed var(--line);
		font-family: var(--font-ui);
	}

	.edit {
		margin: 4px 8px 4px 11px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.mine-note {
		color: var(--side-mine);
		font-family: var(--font-ui);
	}

	/* The hand edit stays plain: what is typed is what lands. */
	textarea {
		font-family: var(--code-font);
		font-size: var(--fs-code);
		line-height: var(--code-lh);
		tab-size: 4;
		border: 1.5px solid var(--side-mine);
		background: var(--side-mine-soft);
		color: var(--ink);
		border-radius: 8px;
		padding: 6px 8px;
		resize: vertical;
		white-space: pre;
	}

	.choices {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		justify-content: center;
		align-items: center;
	}

	.choice {
		padding: 5px 12px 5px 7px;
		font-size: var(--fs-secondary);
		height: 32px;
	}

	.choice[aria-pressed='true'] {
		border-color: var(--accent);
		color: var(--ink);
		background: color-mix(in srgb, var(--accent) 12%, var(--surface));
		font-weight: 600;
	}

	.choice.mine[aria-pressed='true'] {
		border-color: var(--side-mine);
		color: var(--side-mine);
	}

	.center {
		text-align: center;
	}

	.from {
		display: flex;
		justify-content: space-between;
		gap: 10px;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.pad {
		display: block;
		padding: 6px 10px;
	}
</style>
