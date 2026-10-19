<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick, untrack } from 'svelte';
	import ImageDiff from '$lib/diff/ImageDiff.svelte';
	import { fullDate } from '$lib/format';
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { shortAge, strengths } from './age';
	import { pieces } from './colour';
	import { optionKey, step, type Pending } from './keys';
	import Peek from './Peek.svelte';
	import SearchToggles from './SearchToggles.svelte';
	import { files, revisionLabel } from './store.svelte';

	/**
	 * One file at a revision, read and never written (FEAT-131).
	 *
	 * Tabs, a breadcrumb, the file with blame beside every line, and a status
	 * line. The file is a virtual list: only the rows near the window are
	 * drawn, their colour is asked for as they are about to be, and the blame
	 * arrives after the text has been painted.
	 *
	 * Everything is clickable, and everything can be done from the keyboard
	 * with the keys in `keys.ts`. None of them types into the file. `VIEW` in
	 * the status line is a fact, not a mode there is another of.
	 */
	interface Props {
		/** Open the tree at a folder: what a breadcrumb segment does. */
		onreveal?: (dir: string) => void;
	}

	let { onreveal }: Props = $props();

	/** A row's height before it is measured. One line of code at rest. */
	const ROW = 23;

	const file = $derived(files.file);
	const lines = $derived(files.lines);
	const content = $derived(file?.content ?? null);
	const blame = $derived(files.blameOn ? files.blame : null);
	const bars = $derived(blame ? strengths(blame.commits.map((commit) => commit.time)) : []);

	/**
	 * The rows: every line, and the peek under the line it is for. A line is
	 * its index; the peek is that index negated and shifted, so it has a key
	 * of its own and the list measures it apart from its line.
	 */
	const rows = $derived.by(() => {
		const out: number[] = new Array(lines.length);
		for (let i = 0; i < lines.length; i++) out[i] = i;
		const peek = files.peek;
		if (peek !== null && peek < lines.length) out.splice(peek + 1, 0, -(peek + 1));
		return out;
	});

	const parts = $derived((file?.path ?? '').split('/').filter(Boolean));
	const selection = $derived(files.selection);
	const percent = $derived(
		lines.length === 0 ? 0 : Math.round(((files.cursor.line + 1) / lines.length) * 100)
	);
	/** Room for the longest line number, and never less than three digits. */
	const digits = $derived(Math.max(3, String(lines.length).length));

	let list = $state<ReturnType<typeof VirtualRows<number>> | null>(null);
	let pane = $state<HTMLDivElement | null>(null);
	let field = $state<HTMLInputElement | null>(null);
	let pending: Pending = null;

	/**
	 * The rows the browser's selection runs between, kept drawn while it
	 * lasts (FEAT-152). Without them a selection dragged or scrolled past
	 * the drawn rows lost its far end, and only what was on screen copied.
	 */
	let held = $state<[number, number] | null>(null);

	function lineOf(node: Node | null): number | null {
		const element = node instanceof Element ? node : (node?.parentElement ?? null);
		const line = element?.closest<HTMLElement>('.line[data-line]');
		return line && pane?.contains(line) ? Number(line.dataset.line) - 1 : null;
	}

	$effect(() => {
		const onselectionchange = () => {
			const chosen = document.getSelection();
			if (!chosen || chosen.isCollapsed) {
				held = null;
				return;
			}
			const ends = [lineOf(chosen.anchorNode), lineOf(chosen.focusNode)].filter(
				(line): line is number => line !== null
			);
			// Neither end is in the file: the selection is somewhere else.
			if (ends.length === 0) {
				held = null;
				return;
			}
			// One end off the lines, on a spacer while dragging: keep what was held.
			const at = ends.map(rowOf).concat(ends.length === 1 && held ? held : []);
			held = [Math.min(...at), Math.max(...at)];
		};
		document.addEventListener('selectionchange', onselectionchange);
		return () => document.removeEventListener('selectionchange', onselectionchange);
	});

	function rowOf(line: number): number {
		const peek = files.peek;
		return peek !== null && peek < line ? line + 1 : line;
	}

	function follow(align: 'nearest' | 'center' = 'nearest') {
		list?.scrollToIndex(rowOf(files.cursor.line), align);
	}

	function focusFile() {
		(pane?.querySelector('.viewport') as HTMLElement | null)?.focus();
	}

	/** A new file takes the keys, and shows the line it was opened at. */
	$effect(() => {
		if (!file || content?.kind !== 'text') return;
		untrack(() => {
			void tick().then(() => {
				focusFile();
				if (files.cursor.line > 0) follow('center');
			});
		});
	});

	$effect(() => {
		if (files.searching) void tick().then(() => field?.focus());
	});

	/** Half a screen of lines, as `Ctrl+d` goes. */
	function half(): number {
		const height = pane?.clientHeight ?? 0;
		const row = (pane?.querySelector('.line') as HTMLElement | null)?.offsetHeight || ROW;
		return Math.max(1, Math.floor(height / row / 2));
	}

	async function copy() {
		const text = files.reference;
		if (!text) return;
		try {
			await navigator.clipboard.writeText(text);
			notice.ok('Copied', text);
		} catch (error) {
			notice.failed('Could not copy', error);
		}
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.target === field) return;
		const taken = step(pending, {
			key: event.key,
			ctrl: event.ctrlKey,
			meta: event.metaKey,
			alt: event.altKey
		});
		pending = taken.pending;
		if (!taken.handled) return;
		event.preventDefault();
		const action = taken.action;
		if (!action) return;

		switch (action.kind) {
			case 'move':
				files.moveBy(action.by);
				follow();
				return;
			case 'half':
				files.moveBy(action.by * half());
				follow('center');
				return;
			case 'top':
				files.moveTo(0);
				follow();
				return;
			case 'bottom':
				files.moveTo(lines.length - 1);
				follow();
				return;
			case 'search':
				files.startSearch(action.backward);
				return;
			case 'next':
				if (files.searchNext(action.reverse)) follow('center');
				return;
			case 'block':
				files.moveBlock(action.by);
				follow();
				return;
			case 'blame':
				files.toggleBlame();
				return;
			case 'peek':
				if (files.peek === files.cursor.line) files.closePeek();
				else files.openPeek(files.cursor.line);
				follow();
				return;
			case 'close':
				files.dismiss();
				return;
			case 'copy':
				void copy();
				return;
		}
	}

	function searchKey(event: KeyboardEvent) {
		const option = optionKey(event);
		if (option) {
			event.preventDefault();
			files.toggleOption(option);
		} else if (event.key === 'Enter') {
			event.preventDefault();
			const went = files.searchNext();
			files.endSearch();
			focusFile();
			if (went) follow('center');
		} else if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			files.endSearch();
			focusFile();
		}
	}

	/** A press on a line's text: the peek, unless it was a drag to select. */
	function pressLine(line: number, event: MouseEvent) {
		if (window.getSelection()?.isCollapsed === false) return;
		if (event.shiftKey) {
			files.moveTo(line, true);
			return;
		}
		if (files.peek === line) files.closePeek();
		else files.openPeek(line);
	}

	function bytes(size: number): string {
		if (size < 1024) return `${size} B`;
		if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
		return `${(size / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<section class="file" aria-label={file ? `${file.path} at ${files.rev}` : 'No file open'}>
	<div class="tabs" role="tablist" aria-label="Open files">
		{#each files.tabs as tab, index (tab.path + '@' + tab.rev)}
			{@const name = tab.path.slice(tab.path.lastIndexOf('/') + 1)}
			<div class="tab" class:on={index === files.active}>
				<button
					role="tab"
					class="open"
					aria-selected={index === files.active}
					title="{tab.path} at {tab.rev}"
					onclick={() => files.activate(index)}
					onauxclick={(event) => {
						if (event.button === 1) void files.close(index);
					}}
				>
					<span class="name">{name}</span>
					{#if tab.rev !== files.rev}<span class="at note">{revisionLabel(tab.rev)}</span>{/if}
				</button>
				<button class="shut" aria-label="Close {name}" title="Close" onclick={() => files.close(index)}>
					<Icon name="close" size={11} weight={2} />
				</button>
			</div>
		{/each}
	</div>

	{#if file && parts.length > 0}
		<nav class="crumbs mono" aria-label="Path">
			{#each parts as part, index (index)}
				{#if index > 0}<span class="sep" aria-hidden="true">›</span>{/if}
				{#if index < parts.length - 1}
					<button onclick={() => onreveal?.(parts.slice(0, index + 1).join('/'))}>{part}</button>
				{:else}
					<span class="leaf">{part}</span>
				{/if}
			{/each}
		</nav>
	{/if}

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="pane"
		class:blamed={files.blameOn}
		class:wrap={files.wrap}
		style:--digits={digits}
		bind:this={pane}
		{onkeydown}
	>
		{#if files.reading}
			<Loader />
		{:else if files.fileError}
			<p class="said note error">{files.fileError}</p>
		{:else if !file || !content}
			<p class="said note">Choose a file from the tree</p>
		{:else if content.kind === 'text'}
			{#if lines.length === 0}
				<p class="said note">This file is empty</p>
			{:else}
				<VirtualRows
					bind:this={list}
					items={rows}
					key={(row) => (row < 0 ? `p${-row - 1}` : `l${row}`)}
					estimate={(row) => (row < 0 ? 170 : ROW)}
					overscan={500}
					hold={held}
					label="{file.path}, {lines.length} lines"
					onrange={(start, end) => files.wantColour(start, end)}
				>
					{#snippet row(item: number)}
						{#if item < 0}
							<Peek line={-item - 1} />
						{:else}
							{@const commitIndex = blame ? blame.lines[item] : undefined}
							{@const commit = commitIndex === undefined ? null : (blame?.commits[commitIndex] ?? null)}
							{@const first = commit !== null && (item === 0 || blame?.lines[item - 1] !== commitIndex)}
							{@const found = files.match?.line === item ? files.match : null}
							<div
								class="line"
								class:cursor={item === files.cursor.line}
								class:selected={item >= selection[0] && item <= selection[1] && selection[0] !== selection[1]}
								class:lit={commitIndex !== undefined && files.lit === commitIndex}
								class:peeked={files.peek === item}
								data-line={item + 1}
							>
								{#if files.blameOn}
									<button
										class="blame"
										class:agent={!!commit?.agent}
										tabindex="-1"
										disabled={!commit}
										title={commit
											? `${commit.summary} — ${commit.agent ?? commit.authorName}, ${fullDate(commit.time)} — ${commit.short}`
											: undefined}
										onclick={() => files.light(item)}
									>
										<span
											class="bar"
											aria-hidden="true"
											style:opacity={commitIndex === undefined ? 0 : bars[commitIndex]}
										></span>
										{#if commit && first}
											{#if commit.agent}
												<span class="hex"><Icon name="agent" size={15} weight={2.2} /></span>
											{:else}
												<AuthorAvatar email={commit.authorEmail} name={commit.authorName} />
											{/if}
											<span class="who">{commit.agent ?? commit.authorName}</span>
											<span class="why">{commit.summary}</span>
											<span class="age">{shortAge(commit.time)}</span>
										{/if}
									</button>
								{/if}
								<button
									class="number"
									tabindex="-1"
									aria-label="Line {item + 1}"
									onclick={(event) => files.moveTo(item, event.shiftKey)}>{item + 1}</button
								>
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<span class="text" role="presentation" onclick={(event) => pressLine(item, event)}
									>{#each pieces( files.tokens(item), found ? [found.start, found.end] : null ) as piece, i (i)}<span
											class="tok-{piece.type}"
											class:hit={piece.hit}>{piece.text}</span
										>{/each}</span
								>
							</div>
						{/if}
					{/snippet}
				</VirtualRows>
				{#if files.blameOn}
					<!-- On the blame's edge, over the rows: the width is every row's. -->
					<div class="edge">
						<div class="reach"></div>
						<Splitter panel="filesBlame" label="Resize the blame" />
					</div>
				{/if}
			{/if}
		{:else if content.kind === 'image'}
			<div class="picture">
				<ImageDiff
					single
					diff={{
						path: file.path,
						isImage: true,
						mime: content.mime,
						oldSize: null,
						newSize: file.size,
						oldBase64: null,
						newBase64: content.base64
					}}
				/>
			</div>
		{:else if content.kind === 'binary'}
			<p class="said note"><span class="mono">binary</span> · {bytes(file.size)}</p>
		{:else if content.kind === 'lfs'}
			<div class="said note pointer">
				<span>Git LFS pointer · stands for {bytes(content.objectSize)}</span>
				<span class="mono">{content.oid}</span>
			</div>
		{:else if content.kind === 'tooLarge'}
			<p class="said note">{bytes(file.size)} · too large to open here</p>
		{:else}
			<p class="said note">
				<span class="mono">{file.path}</span> is not in {revisionLabel(files.rev)}
			</p>
		{/if}
	</div>

	<footer class="status mono">
		<span class="mode" title="Files reads git's objects and never writes">VIEW</span>
		<span class="cell lifted">{revisionLabel(files.rev)}</span>
		{#if file}<span class="cell">{parts[parts.length - 1] ?? ''}</span>{/if}
		{#if files.searching}
			<label class="search">
				<span aria-hidden="true">{files.backward ? '?' : '/'}</span>
				<input
					bind:this={field}
					type="text"
					aria-label={files.backward ? 'Search backwards' : 'Search'}
					spellcheck="false"
					autocomplete="off"
					value={files.query}
					oninput={(event) => files.setQuery(event.currentTarget.value)}
					onkeydown={searchKey}
					onblur={() => files.endSearch()}
				/>
				<SearchToggles />
			</label>
			{#if files.searchError}<span class="cell error" role="alert">{files.searchError}</span>{/if}
		{:else if files.nowhere}
			<span class="cell muted">not found: {files.query}</span>
		{/if}
		<span class="gap"></span>
		{#if content?.kind === 'text'}
			<span class="cell muted keys">/ search · ]b next blame · b blame</span>
			<span class="cell muted">
				{files.language}{#if files.treeSitter}&nbsp;· tree-sitter{/if}
			</span>
			<span class="cell lifted">{files.cursor.line + 1}:{files.cursor.col + 1}</span>
			<span class="mode">{percent}%</span>
		{/if}
	</footer>
</section>

<style>
	.file {
		flex: 999 1 560px;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: var(--bg);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.tabs {
		flex: none;
		display: flex;
		min-height: 34px;
		overflow-x: auto;
		scrollbar-width: none;
		background: var(--sunken);
		border-bottom: 1px solid var(--soft);
	}

	.tab {
		flex: none;
		display: flex;
		align-items: center;
		border-right: 1px solid var(--soft);
		color: var(--muted);
	}

	.tab.on {
		background: var(--bg);
		color: var(--ink);
	}

	.open {
		display: flex;
		align-items: baseline;
		gap: 8px;
		padding: 8px 4px 8px 14px;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.at {
		font-size: calc(var(--fs-mono) * 0.92);
	}

	.shut {
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		margin-right: 6px;
		border-radius: var(--r-row);
		color: var(--muted);
		opacity: 0;
	}

	.tab:hover .shut,
	.tab.on .shut,
	.shut:focus-visible {
		opacity: 1;
	}

	.shut:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.crumbs {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		padding: 7px 16px;
		border-bottom: 1px solid var(--soft);
		color: var(--muted);
	}

	.crumbs button:hover {
		color: var(--ink);
		text-decoration: underline;
	}

	.leaf {
		color: var(--ink);
	}

	.pane {
		--gutter: 0px;
		--number: calc(var(--digits) * 1ch + 22px);
		position: relative;
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		font-family: var(--font-mono);
		font-size: var(--fs-code);
		line-height: 1.6;
	}

	.pane.blamed {
		--gutter: var(--files-blame-w);
	}

	.edge {
		position: absolute;
		inset-block: 0;
		left: 0;
		display: flex;
		pointer-events: none;
	}

	/* What the splitter measures from: the blame's own width. */
	.reach {
		width: var(--gutter);
	}

	.edge :global(.splitter) {
		pointer-events: auto;
	}

	/* Under the code, not under the gutter: it is about the line. */
	.pane :global(.peek) {
		margin-left: calc(var(--gutter) + var(--number));
	}

	.line {
		display: grid;
		grid-template-columns: var(--gutter) var(--number) minmax(0, 1fr);
		align-items: stretch;
		min-height: 1.6em;
	}

	.line.selected,
	.line.lit {
		background: var(--hover);
	}

	.line.lit {
		background: color-mix(in srgb, var(--accent) 9%, transparent);
	}

	.line.cursor,
	.line.peeked {
		background: var(--selection);
	}

	.blame {
		--avatar-d: 17px;
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		padding: 0 10px 0 0;
		border-right: 1px solid var(--soft);
		font-family: var(--font-ui);
		font-size: var(--fs-mono);
		line-height: 1;
		text-align: left;
		overflow: hidden;
	}

	.blame:disabled {
		cursor: default;
	}

	/* Every row has one: stronger the newer the line is in this file. */
	.bar {
		flex: none;
		align-self: stretch;
		width: 3px;
		background: var(--ink);
	}

	.blame.agent .bar {
		background: var(--agent);
	}

	.hex {
		display: inline-flex;
		color: var(--agent);
	}

	.who {
		flex: none;
		font-weight: 600;
		white-space: nowrap;
	}

	.blame.agent .who {
		color: var(--agent);
	}

	.why {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--muted);
	}

	.age {
		flex: none;
		white-space: nowrap;
		color: var(--muted);
	}

	/*
	 * Placed, not left to flow: with the blame hidden its column is still
	 * there at no width, and a number and text that flowed would take the
	 * first two columns and draw over each other.
	 */
	.number {
		grid-column: 2;
		padding-right: 12px;
		text-align: right;
		color: var(--muted);
		font: inherit;
		font-variant-numeric: tabular-nums;
		user-select: none;
	}

	.number:hover {
		color: var(--ink);
	}

	.line.cursor .number {
		color: var(--ink);
	}

	.text {
		grid-column: 3;
		white-space: pre;
		overflow: hidden;
		cursor: text;
		tab-size: 4;
	}

	.wrap .text {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.hit {
		background: color-mix(in srgb, var(--warn) 45%, transparent);
		border-radius: 2px;
	}

	.said {
		margin: auto;
		padding: 24px;
		text-align: center;
		font-family: var(--font-ui);
	}

	.pointer {
		display: flex;
		flex-direction: column;
		gap: 6px;
		overflow-wrap: anywhere;
	}

	.picture {
		flex: 1;
		min-height: 0;
		overflow: auto;
		font-family: var(--font-ui);
	}

	.error {
		color: var(--danger);
	}

	.status {
		flex: none;
		display: flex;
		align-items: stretch;
		background: var(--sunken);
		border-top: 1px solid var(--soft);
		white-space: nowrap;
		overflow: hidden;
	}

	.status > * {
		padding: 4px 12px;
	}

	/* VIEW is a fact, not a mode there is another of. */
	.mode {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	.lifted {
		background: var(--surface-2);
	}

	.cell {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.gap {
		flex: 1;
		padding: 0;
	}

	.search {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 120px;
		padding-block: 0;
	}

	.search input {
		flex: 1;
		min-width: 0;
		padding: 0;
		border: none;
		border-radius: 0;
		background: none;
		box-shadow: none;
		outline: none;
		font: inherit;
	}

	/* The hints go first when the window is narrow. */
	@media (max-width: 1100px) {
		.keys {
			display: none;
		}
	}
</style>
