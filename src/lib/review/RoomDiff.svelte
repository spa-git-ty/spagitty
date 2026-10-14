<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';
	import { relativeTime } from '$lib/format';
	import { PLAIN, reading } from '$lib/reading.svelte';
	import { scale } from '$lib/scale.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Markdown from '$lib/ui/Markdown.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { pullFileAttrs } from '$lib/files/open';
	import { room } from './room.svelte';
	import type { ConflictFix } from '$lib/types';
	import { holds, lineOfRow, lineRows, rowsOf, sideBySide, type LineRow, type Row, type SideName } from './rows';
	import { draftsByPlace } from './drafts';
	import { paint } from '$lib/diff/highlight';
	import { threadsByPlace, type Thread } from './threads';
	import * as agentWork from './agent.svelte';
	import { agents } from '$lib/agents/store.svelte';
	import type { PendingComment } from './record';

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
	 *
	 * And the writing (FEAT-093): a `+` on every line opens a box under it,
	 * shift-clicking a line number stretches it over a range, and what is
	 * written waits, tinted, under its lines until Finish review. Threads
	 * from the host take a reply and are resolved or reopened in place.
	 *
	 * Before and after (FEAT-129) is the same list with each line of a file
	 * set beside its counterpart: the file as it was on the left, as it is on
	 * the right. One column, so both sides move as one and stay level.
	 */

	/** Room under the last row for the review pill floating over it. */
	const TAIL = 132;

	let list = $state<VirtualRows<Row> | null>(null);

	const laidOut = $derived(
		rowsOf(room.laid, threadsByPlace(room.threads), room.sides, {
			drafts: draftsByPlace(room.currentDrafts),
			composer: room.composer
		})
	);
	const paired = $derived(room.scope === 'sides');
	const rows = $derived(paired ? sideBySide(laidOut) : laidOut);
	/** The lines the comment being written covers, by index, in its file. */
	const covered = $derived.by(() => {
		const writing = room.composer;
		if (!writing) return null;
		return { path: writing.path, from: Math.min(writing.from, writing.to), to: Math.max(writing.from, writing.to) };
	});
	/** The agent's finding being edited, and its words as they are typed (2.0). */
	let rewriting = $state<{ id: string; text: string } | null>(null);

	function levelOf(draft: PendingComment) {
		return agents.assignments.find((a) => a.id === draft.agent?.assignment)?.level ?? null;
	}

	async function keepRewrite(draft: PendingComment) {
		if (!rewriting) return;
		await agentWork.edit(draft, rewriting.text);
		rewriting = null;
	}

	/** What is typed into the composer, and into each thread's reply box. */
	let draftText = $state('');
	let replies = $state<Record<number, string>>({});
	let replying = $state<number | null>(null);
	const target = $derived(room.pr?.targetBranch ?? 'main');
	const lines = $derived(lineRows(rows));
	const focusAt = $derived.by(() => {
		const key = room.focus;
		return key === null ? -1 : rows.findIndex((row) => holds(row, key));
	});
	const focusChunk = $derived.by(() => {
		const row = rows[focusAt];
		return (row && lineOfRow(row)?.chunk) ?? null;
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
			case 'pair':
				return linePx + (row.last ? 10 : 0);
			case 'thread':
				return 120 + 64 * row.thread.comments.length + (row.last ? 10 : 0);
			case 'draft':
				return 96 + (row.last ? 10 : 0);
			case 'composer':
				return 180 + (row.last ? 10 : 0);
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
		const first = lines.find((at) => lineOfRow(rows[at])?.line.origin !== 'context');
		const line = lineOfRow(rows[first ?? lines[0]]);
		if (line) room.setFocus(line.key);
	});

	// A jump from the Conversation card, once its row is drawn.
	$effect(() => {
		const target = room.target;
		if (target === null) return;
		const at = rows.findIndex((row) => holds(row, target));
		if (at < 0) return;
		list?.scrollToIndex(at, 'center');
		room.reached();
	});

	function move(by: 1 | -1) {
		if (lines.length === 0) return;
		const here = lines.indexOf(focusAt);
		const next = here < 0 ? 0 : Math.min(lines.length - 1, Math.max(0, here + by));
		const line = lineOfRow(rows[lines[next]]);
		if (line) room.setFocus(line.key);
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

	/** A line number pressed: the ruler there, or with shift a range to comment on. */
	function pressNumber(event: MouseEvent, row: LineRow) {
		if (event.shiftKey && room.head) {
			room.extend(row.path, row.index);
			return;
		}
		room.setFocus(row.key);
	}

	function compose(row: LineRow) {
		room.setFocus(row.key);
		room.compose(row.path, row.index);
		draftText = '';
	}

	let keeping = false;

	async function keep() {
		if (keeping) return;
		keeping = true;
		if (await room.addDraft(draftText)) draftText = '';
		keeping = false;
	}

	/** `Line 17`, or `Lines 12–17`: what the box covers, by the new numbers where there are any. */
	function coverage(row: Extract<Row, { kind: 'composer' }>): string {
		const content = room.contentOf(row.path);
		if (!content) return '';
		const number = (index: number) => {
			const line = content.lines[index];
			return line.new ?? line.old ?? 0;
		};
		const [from, to] = [Math.min(row.from, row.to), Math.max(row.from, row.to)];
		return from === to ? `Line ${number(to)}` : `Lines ${number(from)}–${number(to)}`;
	}

	async function answer(thread: Thread) {
		const text = replies[thread.id] ?? '';
		if (text.trim() === '' || replying !== null) return;
		replying = thread.id;
		const sent = await room.reply(thread, text);
		replying = null;
		if (sent) replies = { ...replies, [thread.id]: '' };
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

<!--
	One line of a file. `both` is the line in the one column, numbered by the
	new file where it is in it. `before` and `after` are the two halves of a
	pair: the same line twice when it did not change, each by its own file's
	number. A comment is started from the side the line is read on: the new
	file's, or the old one's for a line that was removed.
-->
{#snippet cells(row: LineRow, side: 'both' | 'before' | 'after')}
	{@const content = room.contentOf(row.path)}
	{@const runs = paint(
		content?.syntax[row.index] ?? [{ type: 'plain', text: row.line.text }],
		content?.words.get(row.index)
	)}
	{@const focused = room.focus === row.key}
	{@const number = side === 'before' ? row.line.old : side === 'after' ? row.line.new : (row.line.new ?? row.line.old)}
	<div
		class="line {row.line.origin}"
		class:fixed={row.fixed}
		class:covered={covered !== null && covered.path === row.path && row.index >= covered.from && row.index <= covered.to}
		class:focused
		class:ruled={room.ruler && focused}
		class:dim={room.ruler && room.rulerMode === 'chunk' && focusChunk !== null && row.chunk !== focusChunk}
	>
		<span class="marker"></span>
		<button
			class="num"
			class:old={side === 'both' && row.line.new === null}
			aria-label="Focus line"
			onclick={(event) => pressNumber(event, row)}>{number ?? ''}</button
		>
		<span class="sign">{sign(row.line.origin)}</span>
		<span class="plus-cell">
			{#if room.head && (side !== 'before' || row.line.origin === 'removed')}
				<button
					class="plus"
					class:shown={focused || (covered !== null && covered.path === row.path && covered.to === row.index)}
					aria-label="Comment on this line"
					onclick={() => compose(row)}><Icon name="plus" size="0.75em" weight={2.6} /></button
				>
			{/if}
		</span>
		<span class="text"
			>{#each runs as run, p (p)}<span class="tok-{run.type}" class:word={run.changed}>{run.text}</span
				>{/each}{#if row.line.text === ''}&#8203;{/if}</span
		>
	</div>
{/snippet}

<VirtualRows bind:this={list} items={rows} key={keyOf} {estimate} tail={TAIL} label="Diff">
	{#snippet row(row: Row, index: number)}
		{#if row.kind === 'file'}
			{@const file = fileOf(row.path)}
			{@const [dir, name] = split(row.path)}
			<div
				class="file"
				class:first={index === 0}
				{...pullFileAttrs(row.path, file?.status ?? 'modified', room.head)}
			>
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
				{#if paired}
					<div class="head-text sides"><span>Before</span><span>After</span></div>
				{:else}
					<div class="head-text mono">{row.text}</div>
				{/if}
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
			<div class="slot" class:gap={row.last}>
				<div class="part {row.tone}" class:end={row.last}>
					{@render cells(row, 'both')}
				</div>
			</div>
		{:else if row.kind === 'pair'}
			<div class="slot" class:gap={row.last}>
				<div class="part pair {row.tone}" class:end={row.last}>
					{#if row.before}{@render cells(row.before, 'before')}{:else}<div class="line none"></div>{/if}
					{#if row.after}{@render cells(row.after, 'after')}{:else}<div class="line none"></div>{/if}
				</div>
			</div>
		{:else if row.kind === 'thread'}
			<div class="slot" class:gap={row.last}>
				<div class="part {row.tone}" class:end={row.last}>
					<div class="thread">
						<div class="thread-head">
							<Chip active={!row.thread.resolved}>{row.thread.resolved ? 'Resolved' : 'Open'}</Chip>
							<span class="grow"></span>
							{#if row.thread.threadId}
								<Chip onclick={() => room.setResolved(row.thread, !row.thread.resolved)}
									>{row.thread.resolved ? 'Reopen' : 'Resolve'}</Chip
								>
							{/if}
						</div>
						{#each row.thread.comments as comment (comment.id)}
							<div class="comment">
								<AuthorAvatar email={null} name={comment.author} />
								<div class="said">
									<span class="note"><span class="who">{comment.author}</span> · {relativeTime(comment.createdAt)}</span>
									<div class="body"><Markdown source={comment.body} compact /></div>
								</div>
							</div>
						{/each}
						<div class="reply">
							<input
								type="text"
								aria-label="Reply"
								placeholder="Reply…"
								value={replies[row.thread.id] ?? ''}
								oninput={(event) => (replies = { ...replies, [row.thread.id]: event.currentTarget.value })}
								onkeydown={(event) => {
									if (event.key === 'Enter') void answer(row.thread);
								}}
							/>
							<Btn onclick={() => answer(row.thread)} disabled={replying === row.thread.id || !(replies[row.thread.id] ?? '').trim()}
								>Reply</Btn
							>
						</div>
					</div>
				</div>
			</div>
		{:else if row.kind === 'draft' && row.draft.agent?.state === 'proposed'}
			{@const note = row.draft.agent}
			{@const why = agentWork.whyOf(row.draft)}
			<div class="slot" class:gap={row.last}>
				<div class="part {row.tone}" class:end={row.last}>
					<div class="proposal" aria-label="{note.name} proposes">
						<div class="pending-head">
							<span class="by"><Icon name="agent" size="1em" weight={2} />{note.name}</span>
							<span class="note">proposed</span>
							<Chip><span class="severity {note.severity}">{note.severity}</span></Chip>
							{#if !note.sure}<Chip><span class="unsure">unsure</span></Chip>{/if}
						</div>
						{#if rewriting?.id === row.draft.id}
							<textarea rows="3" aria-label="Your words for this comment" bind:value={rewriting.text}></textarea>
							<div class="composer-actions">
								<Btn onclick={() => (rewriting = null)}>Cancel</Btn>
								<Btn primary quiet disabled={!rewriting.text.trim()} onclick={() => keepRewrite(row.draft)}>Keep as mine</Btn>
							</div>
						{:else}
							<div class="body"><Markdown source={row.draft.body} compact /></div>
							{#if why}<p class="why note">{why}</p>{/if}
							<div class="proposal-actions">
								<Btn primary quiet onclick={() => agentWork.accept(row.draft)}>Accept</Btn>
								<Btn onclick={() => (rewriting = { id: row.draft.id, text: row.draft.body })}>Edit</Btn>
								<Btn onclick={() => agentWork.dismiss(row.draft)}>Dismiss</Btn>
								{#if levelOf(row.draft) !== 'unattended' && !why}
									<Btn title="The agent says what it read and why" onclick={() => agentWork.askWhy(row.draft)}>Why?</Btn>
								{/if}
							</div>
						{/if}
					</div>
				</div>
			</div>
		{:else if row.kind === 'draft'}
			<div class="slot" class:gap={row.last}>
				<div class="part {row.tone}" class:end={row.last}>
					<div class="pending">
						<div class="pending-head">
							<span class="pending-title">Pending · goes out with Finish review</span>
							{#if row.draft.agent}<span class="note">drafted with {row.draft.agent.name}</span>{/if}
							<span class="grow"></span>
							<button class="delete note" onclick={() => room.removeDraft(row.draft.id)}>Delete</button>
						</div>
						<div class="body"><Markdown source={row.draft.body} compact /></div>
					</div>
				</div>
			</div>
		{:else if row.kind === 'composer'}
			<div class="slot" class:gap={row.last}>
				<div class="part {row.tone}" class:end={row.last}>
					<div class="composer">
						<label for="composer" class="note">{coverage(row)} · shift-click a line number to cover a range</label>
						<!-- svelte-ignore a11y_autofocus -->
						<textarea
							id="composer"
							rows="3"
							autofocus
							placeholder="What should change here, and why?"
							bind:value={draftText}
							onkeydown={(event) => {
								if (event.key === 'Escape') room.closeComposer();
							}}
						></textarea>
						<div class="composer-actions">
							<Btn onclick={() => room.closeComposer()}>Cancel</Btn>
							<Btn primary quiet onclick={keep} disabled={!draftText.trim()}>Add to review</Btn>
						</div>
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

	/* Before and after (FEAT-129): the row is two lines, the file as it was
	   and the file as it is, with a rule between them. */
	.part.pair {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
	}

	.part.pair > .line + .line {
		border-left: 1px solid var(--pane-edge);
	}

	/* Nothing on this side: the line is only in the other file. */
	.line.none {
		background-color: color-mix(in srgb, var(--sunken) 70%, transparent);
	}

	.head-text.sides {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		font-size: var(--fs-mono);
		font-weight: 600;
	}

	.line {
		display: grid;
		/* One number per line (TASK-053): the new one, or the old one on a
		   removed line. Two side by side read as each line numbered twice. */
		grid-template-columns: 3px var(--diff-gutter-w) 20px 28px minmax(0, 1fr);
		font-family: var(--code-font);
		font-size: var(--fs-code);
		line-height: var(--code-lh);
		letter-spacing: var(--code-ls);
	}

	/* An addition is green and a removal red, always (BUG-069). The ruler and
	   the comment range are layers over that colour, never a replacement for
	   it, which is why they are an image and not a background colour. */
	.line.added {
		background-color: var(--diff-add-bg);
	}

	.line.removed {
		background-color: var(--diff-del-bg);
	}

	/* The ruler: a warm band under the line being read. */
	.line.ruled {
		background-image: linear-gradient(var(--ruler), var(--ruler));
	}

	/* The lines the comment being written covers. */
	.line.covered {
		background-image: linear-gradient(var(--covered), var(--covered));
	}

	/* Both at once, which is where a comment usually starts: one over the other. */
	.line.ruled.covered {
		background-image:
			linear-gradient(var(--covered), var(--covered)), linear-gradient(var(--ruler), var(--ruler));
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

	/* A removed line's number is the old file's: there, but quieter. */
	.num.old {
		opacity: 0.55;
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

	.plus-cell {
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding-top: max(0px, calc((var(--fs-code) * var(--code-lh) - 22px) / 2));
	}

	/* Faint until the line is the one being read: a column of strong buttons
	   down every line would be the loudest thing in the diff. */
	.plus {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		/* Neutral: an accent-coloured button on a green line reads red. */
		background: color-mix(in srgb, var(--ink) 10%, transparent);
		color: var(--ink);
		opacity: 0.3;
		transition: opacity var(--t-fast) var(--ease);
	}

	.plus.shown,
	.plus:hover,
	.plus:focus-visible {
		opacity: 1;
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
		margin: 8px 14px 12px calc(3px + var(--diff-gutter-w) + 48px);
		padding: 12px 14px;
		border-radius: var(--r-floating);
		background: var(--surface-2);
		border: 1px solid var(--pane-edge);
		box-shadow: var(--shadow-1);
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.reply {
		display: flex;
		gap: 8px;
	}

	.reply input {
		flex: 1;
		min-width: 0;
		border: 1px solid var(--pane-edge);
		background: var(--sunken);
		border-radius: var(--r-pill);
		padding: 7px 14px;
		font-size: var(--fs-secondary);
	}

	/* Written here and not sent: warm, so it is never taken for what the
	   host already has. */
	.pending {
		margin: 8px 14px 12px calc(3px + var(--diff-gutter-w) + 48px);
		padding: 12px 14px;
		border-radius: var(--r-floating);
		background: var(--warn-soft);
		border: 1px solid color-mix(in srgb, var(--warn) 40%, transparent);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	/* An agent's finding, waiting for the person (2.0): in the agent's colour,
	   so it is never taken for the person's own words or the host's. */
	.proposal {
		margin: 8px 14px 12px calc(3px + var(--diff-gutter-w) + 48px);
		padding: 12px 14px;
		border-radius: var(--r-floating);
		background: var(--agent-soft);
		border: 1px solid var(--agent-edge);
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.proposal textarea {
		border: 1px solid var(--pane-edge);
		background: var(--sunken);
		border-radius: var(--r-field);
		padding: 9px 12px;
		font-family: var(--read-font);
		font-size: var(--read-size);
		line-height: 1.6;
		resize: vertical;
	}

	.by {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--agent);
		font-weight: 600;
	}

	.severity.high {
		color: var(--danger);
	}

	.severity.medium {
		color: var(--warn);
	}

	.unsure {
		color: var(--warn);
	}

	.why {
		margin: 0;
		padding-left: 10px;
		border-left: 2px solid var(--agent-edge);
	}

	.proposal-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.pending-head {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.pending-title {
		color: var(--warn);
		font-weight: 600;
		font-size: var(--fs-secondary);
	}

	.delete {
		text-decoration: underline;
	}

	.composer {
		margin: 8px 14px 12px calc(3px + var(--diff-gutter-w) + 48px);
		padding: 12px 14px;
		border-radius: var(--r-floating);
		background: var(--surface-2);
		border: 1px solid color-mix(in srgb, var(--accent) 55%, transparent);
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.composer textarea {
		border: 1px solid var(--pane-edge);
		background: var(--sunken);
		border-radius: var(--r-field);
		padding: 9px 12px;
		font-family: var(--read-font);
		font-size: var(--read-size);
		line-height: 1.6;
		resize: vertical;
	}

	.composer-actions {
		display: flex;
		gap: 8px;
		justify-content: flex-end;
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

	/* Drawn as the host draws it (FEAT-094), at a reading measure. */
	.body {
		letter-spacing: var(--code-ls);
		max-width: 72ch;
	}
</style>
