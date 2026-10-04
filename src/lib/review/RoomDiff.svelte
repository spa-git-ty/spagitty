<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';
	import { relativeTime } from '$lib/format';
	import { PLAIN, reading } from '$lib/reading.svelte';
	import { scale } from '$lib/scale.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { room } from './room.svelte';
	import type { ConflictFix } from '$lib/types';
	import { lineRows, rowsOf, type Row, type SideName } from './rows';
	import { threadsByPlace } from './threads';

	/**
	 * The review room's diff column (FEAT-091).
	 *
	 * One flat list of rows — a header per file, the cards of each file's
	 * changed parts, the folds between them, the threads under their lines —
	 * drawn only near the viewport, so a pull request of a thousand files
	 * reads as fast as one of three.
	 *
	 * The reading aids live here: the focus ruler under the line being read,
	 * which `j` and `k` move and a line number sets, and the other chunks faded
	 * back when Settings › Reading asks for the whole chunk.
	 */

	/** Room under the last row for the review pill floating over it. */
	const TAIL = 96;

	let list = $state<VirtualRows<Row> | null>(null);

	const rows = $derived(rowsOf(room.laid, threadsByPlace(room.threads), room.sides));
	const target = $derived(room.pr?.targetBranch ?? 'main');
	const lines = $derived(lineRows(rows));
	const focusAt = $derived(room.focus === null ? -1 : rows.findIndex((row) => row.key === room.focus));
	const focusChunk = $derived.by(() => {
		const row = rows[focusAt];
		return row && row.kind === 'line' ? row.chunk : null;
	});

	/** One line's height before it is measured. */
	const linePx = $derived(
		(room.readingSet ? reading.current.size * reading.current.lineHeight : PLAIN.size * PLAIN.lineHeight) *
			scale.zoom *
			scale.text +
			2
	);

	function estimate(row: Row): number {
		switch (row.kind) {
			case 'file':
				return 48;
			case 'note':
				return 44;
			case 'fold':
				return 38;
			case 'head':
				return row.fix ? 92 : 28;
			case 'side':
				return 44 + linePx * Math.max(1, sideLines(row.fix, row.side).length);
			case 'line':
				return linePx + (row.last ? 10 : 0);
			case 'thread':
				return 70 + 64 * row.thread.comments.length + (row.last ? 10 : 0);
		}
	}

	const keyOf = (row: Row) => row.key;

	// A file opened on its own starts at its top.
	$effect(() => {
		void room.selected;
		if (room.layout === 'one') list?.scrollToIndex(0, 'start');
	});

	// The ruler starts on the first change, and moves there when the line it
	// was on is no longer shown.
	$effect(() => {
		if (focusAt >= 0 || lines.length === 0) return;
		const first = lines.find((at) => {
			const row = rows[at];
			return row.kind === 'line' && row.line.origin !== 'context';
		});
		room.setFocus(rows[first ?? lines[0]].key);
	});

	// A jump from the Conversation card, once its row is drawn.
	$effect(() => {
		const target = room.target;
		if (target === null) return;
		const at = rows.findIndex((row) => row.key === target);
		if (at < 0) return;
		list?.scrollToIndex(at, 'center');
		room.reached();
	});

	function move(by: 1 | -1) {
		if (lines.length === 0) return;
		const here = lines.indexOf(focusAt);
		const next = here < 0 ? 0 : Math.min(lines.length - 1, Math.max(0, here + by));
		room.setFocus(rows[lines[next]].key);
		list?.scrollToIndex(lines[next], 'nearest');
	}

	function onkeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement | null;
		if (target && (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))) return;
		if (event.metaKey || event.ctrlKey || event.altKey) return;
		if (event.key === 'j') {
			event.preventDefault();
			move(1);
		} else if (event.key === 'k') {
			event.preventDefault();
			move(-1);
		}
	}

	function split(path: string): [string, string] {
		const cut = path.lastIndexOf('/');
		return [path.slice(0, cut + 1), path.slice(cut + 1)];
	}

	function sideLines(fix: ConflictFix, side: SideName): string[] {
		return side === 'main' ? fix.mainSide : fix.branchSide;
	}

	/** Which merge wrote it, and whether it was a conflict at all. */
	function madeIn(fix: ConflictFix): string {
		const where = `Made in merge ${fix.short}${fix.summary ? ` (${fix.summary})` : ''}`;
		return fix.mainSide.length > 0 || fix.branchSide.length > 0
			? `${where}.`
			: `${where}, away from any conflict.`;
	}

	function unchanged(count: number): string {
		return count === 1 ? '1 unchanged line' : `${count} unchanged lines`;
	}

	function sign(origin: string): string {
		return origin === 'added' ? '+' : origin === 'removed' ? '−' : '';
	}

	function fileOf(path: string) {
		return room.files.find((file) => file.path === path);
	}
</script>

<svelte:window {onkeydown} />

<VirtualRows bind:this={list} items={rows} key={keyOf} {estimate} tail={TAIL} label="Diff">
	{#snippet row(row: Row, index: number)}
		{#if row.kind === 'file'}
			{@const file = fileOf(row.path)}
			{@const [dir, name] = split(row.path)}
			<div class="file" class:first={index === 0}>
				<span class="path mono" title={row.path}><span class="dir">{dir}</span>{name}</span>
				{#if file}
					<span class="counts mono"><span class="add">+{file.added}</span> <span class="del">−{file.removed}</span></span>
				{/if}
				<span class="grow"></span>
				<Chip active={room.isViewed(row.path)} onclick={() => room.toggleViewed(row.path)}>
					<Icon name="check" size="0.95em" weight={2.4} />Viewed
				</Chip>
			</div>
		{:else if row.kind === 'note'}
			<div class="file-note note">{row.text}</div>
		{:else if row.kind === 'fold'}
			<div class="fold-slot">
				<button class="fold note" onclick={() => room.expand(row.path, row.id)}>
					<Icon name="unfold" size="1em" weight={2} />{unchanged(row.count)}
				</button>
			</div>
		{:else if row.kind === 'head'}
			<div class="part top {row.tone}">
				{#if row.fix}
					{@const fix = row.fix}
					<div class="fix">
						<span class="fix-icon"><Icon name="conflict" size="1.2em" /></span>
						<span class="fix-text">
							<span class="fix-title">Conflict fix, not in the author's own commits</span>
							<span class="fix-note">{madeIn(fix)}</span>
						</span>
						{#if fix.mainSide.length > 0 || fix.branchSide.length > 0}
							<Chip active={room.sides.get(row.chunk) === 'main'} onclick={() => room.toggleSide(row.chunk, 'main')}
								>{target}'s side</Chip
							>
							<Chip
								active={room.sides.get(row.chunk) === 'branch'}
								onclick={() => room.toggleSide(row.chunk, 'branch')}>branch's side</Chip
							>
						{/if}
					</div>
				{/if}
				<div class="head-text mono">{row.text}</div>
			</div>
		{:else if row.kind === 'side'}
			{@const lines = sideLines(row.fix, row.side)}
			<div class="slot">
				<div class="part {row.tone}">
					<div class="side">
						<span class="note">{row.side === 'main' ? `What ${target} had here` : 'What this branch had here'}</span>
						{#if lines.length > 0}
							<pre class="side-lines">{lines.join('\n')}</pre>
						{:else}
							<span class="note">Nothing: it had removed these lines.</span>
						{/if}
					</div>
				</div>
			</div>
		{:else if row.kind === 'line'}
			{@const words = room.contentOf(row.path)?.words.get(row.index)}
			<div class="slot" class:gap={row.last}>
				<div class="part {row.tone}" class:end={row.last}>
					<div
						class="line {row.line.origin}"
						class:fixed={row.fixed}
						class:focused={room.ruler && focusAt === index}
						class:dim={room.ruler && room.rulerMode === 'chunk' && focusChunk !== null && row.chunk !== focusChunk}
					>
						<span class="marker"></span>
						<button class="num" aria-label="Focus line" onclick={() => room.setFocus(row.key)}
							>{row.line.old ?? ''}</button
						>
						<button class="num" aria-label="Focus line" onclick={() => room.setFocus(row.key)}
							>{row.line.new ?? ''}</button
						>
						<span class="sign">{sign(row.line.origin)}</span>
						<span class="text"
							>{#each words ?? [{ text: row.line.text, changed: false }] as piece, p (p)}<span
									class:word={piece.changed}>{piece.text}</span
								>{/each}{#if row.line.text === ''}&#8203;{/if}</span
						>
					</div>
				</div>
			</div>
		{:else if row.kind === 'thread'}
			<div class="slot" class:gap={row.last}>
				<div class="part {row.tone}" class:end={row.last}>
					<div class="thread">
						<div class="thread-head">
							<Chip active={!row.thread.resolved}>{row.thread.resolved ? 'Resolved' : 'Open'}</Chip>
						</div>
						{#each row.thread.comments as comment (comment.id)}
							<div class="comment">
								<AuthorAvatar email={null} name={comment.author} />
								<div class="said">
									<span class="note"><span class="who">{comment.author}</span> · {relativeTime(comment.createdAt)}</span>
									<p class="body">{comment.body}</p>
								</div>
							</div>
						{/each}
					</div>
				</div>
			</div>
		{/if}
	{/snippet}
</VirtualRows>

<style>
	.file {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 22px 12px 10px;
	}

	.file.first {
		padding-top: 4px;
	}

	.path {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
	}

	.dir {
		color: var(--muted);
	}

	.counts {
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.add {
		color: var(--ok);
	}

	.del {
		color: var(--danger);
	}

	.grow {
		flex: 1;
	}

	.file-note {
		padding: 10px 14px 14px;
	}

	.fold-slot {
		display: flex;
		justify-content: center;
		padding: 0 8px 10px;
	}

	.fold {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
	}

	.fold:hover {
		background: var(--hover);
		color: var(--ink);
	}

	/*
	 * A card drawn a row at a time: every row carries the card's sides, the
	 * header its top, the last row its bottom. A card that is one element
	 * cannot be half drawn, and the column draws only what is in view.
	 */
	.slot {
		padding: 0 8px;
	}

	.slot.gap {
		padding-bottom: 10px;
	}

	.part {
		border-left: 1px solid var(--pane-edge);
		border-right: 1px solid var(--pane-edge);
		background-color: var(--surface);
		overflow: hidden;
	}

	.part.top {
		margin: 0 8px;
		border-top: 1px solid var(--pane-edge);
		border-radius: var(--r-panel) var(--r-panel) 0 0;
	}

	.part.end {
		border-bottom: 1px solid var(--pane-edge);
		border-radius: 0 0 var(--r-panel) var(--r-panel);
		box-shadow: var(--shadow-1);
	}

	/* Unchanged lines shown: there, but not news. */
	/* Written while fixing a merge conflict: framed in sky, never mistaken
	   for the author's own work (FEAT-092). */
	.part.conflict {
		border-color: var(--resolve);
		border-left-width: 1.5px;
		border-right-width: 1.5px;
	}

	.part.top.conflict {
		border-top-width: 1.5px;
	}

	.part.end.conflict {
		border-bottom-width: 1.5px;
	}

	.fix {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
		padding: 10px 14px;
		background: var(--resolve-soft);
	}

	.fix-icon {
		display: inline-flex;
		color: var(--resolve);
	}

	.fix-text {
		flex: 1 1 300px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.fix-title {
		color: var(--resolve);
		font-weight: 600;
		font-size: var(--fs-secondary);
	}

	.fix-note {
		font-family: var(--read-font);
		font-size: var(--fs-secondary);
		line-height: 1.55;
	}

	.side {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px 14px;
		background: var(--sunken);
		border-bottom: 1px solid var(--pane-edge);
	}

	.side-lines {
		margin: 0;
		font-family: var(--code-font);
		font-size: var(--fs-code);
		line-height: var(--code-lh);
		letter-spacing: var(--code-ls);
		white-space: pre-wrap;
		word-break: break-word;
		tab-size: 4;
	}

	.part.plain {
		background-color: transparent;
	}

	.head-text {
		padding: 5px 14px;
		font-size: var(--fs-mono);
		color: var(--muted);
		background: var(--sunken);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.line {
		display: grid;
		grid-template-columns: 3px var(--diff-gutter-w) var(--diff-gutter-w) 20px minmax(0, 1fr);
		font-family: var(--code-font);
		font-size: var(--fs-code);
		line-height: var(--code-lh);
		letter-spacing: var(--code-ls);
	}

	.line.added {
		background: var(--diff-add-bg);
	}

	.line.removed {
		background: var(--diff-del-bg);
	}

	/* The ruler: a warm band under the line being read. */
	.line.focused {
		background: var(--ruler);
	}

	.line.dim {
		opacity: 0.42;
	}

	.marker {
		align-self: stretch;
	}

	.line.added .marker {
		background: var(--ok);
	}

	/* The exact lines the fix wrote, inside a card that may hold more. */
	.line.fixed .marker {
		background: var(--resolve);
	}

	.line.removed .marker {
		background: var(--danger);
	}

	/* Numbers in the interface's mono at its small size, on the code's line
	   pitch, so they sit level with the line they name. */
	.num {
		padding: 1px 8px 0 0;
		text-align: right;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		line-height: calc(var(--fs-code) * var(--code-lh));
		letter-spacing: 0;
		color: var(--muted);
		user-select: none;
	}

	.num:hover {
		color: var(--ink);
	}

	.sign {
		padding-top: 1px;
		text-align: center;
		font-weight: 700;
		user-select: none;
	}

	.line.added .sign {
		color: var(--ok);
	}

	.line.removed .sign {
		color: var(--danger);
	}

	.text {
		padding: 1px 12px 1px 4px;
		white-space: pre-wrap;
		word-break: break-word;
		tab-size: 4;
		color: var(--ink);
	}

	/* Removed lines go quiet rather than struck through. */
	.line.removed .text {
		color: var(--diff-del-ink);
	}

	.line.added .word {
		background: var(--diff-add-hl);
		border-radius: 4px;
	}

	.line.removed .word {
		background: var(--diff-del-hl);
		border-radius: 4px;
	}

	.thread {
		margin: 8px 14px 12px calc(3px + 2 * var(--diff-gutter-w) + 20px);
		padding: 12px 14px;
		border-radius: var(--r-floating);
		background: var(--surface-2);
		border: 1px solid var(--pane-edge);
		box-shadow: var(--shadow-1);
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.thread-head {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.comment {
		display: flex;
		gap: 10px;
	}

	.said {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
	}

	.who {
		color: var(--ink);
		font-weight: 600;
	}

	/* As written: a comment is read as prose, not rendered as a page. */
	.body {
		margin: 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-family: var(--read-font);
		font-size: var(--read-size);
		line-height: 1.65;
		letter-spacing: var(--code-ls);
		max-width: 62ch;
	}
</style>
