// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The review room's diff as one flat list of rows (FEAT-091).
 *
 * A file arrives whole — every line of the new version with the removed lines
 * in place (FEAT-089) — and is cut here into the changed parts, with the
 * unchanged runs between them folded, or left whole. Each part is a card on
 * the screen, but the list is flat: a header row, its lines, the threads under
 * them, each knowing which card it belongs to and whether it ends it. That is
 * what lets the room draw only the rows in view however long the pull request
 * is — a card that is one element cannot be half drawn.
 *
 * Pure: the same lines, scope and expanded folds give the same rows.
 */

import type { ConflictFix, DiffLine, Hunk } from '../types';
import type { PendingComment } from './record';
import { placeOf, type Thread } from './threads';

export type Scope = 'changes' | 'whole';

/** Unchanged lines kept on each side of a change, as `git diff` keeps them. */
export const CONTEXT = 3;

/**
 * A run of unchanged lines shorter than this is shown, not folded: a fold
 * that hides two lines takes as much room as the two lines would.
 */
export const MIN_FOLD = 4;

export interface Block {
	/** A changed part; unchanged lines shown; unchanged lines folded away. */
	kind: 'hunk' | 'plain' | 'fold';
	/** Indices into the file's lines: `from` inclusive, `to` exclusive. */
	from: number;
	to: number;
}

interface Range {
	from: number;
	to: number;
	/** Where its first change starts and its last one ends. */
	first: number;
	end: number;
	/** Written while fixing a merge conflict (FEAT-092). */
	fixed: boolean;
}

/**
 * The changed parts of `lines`, with a little context, as index ranges.
 *
 * Changes close together are one part. A conflict fix and the author's own
 * change are never one part, however close: the card is what says who wrote
 * it. Two such that would have shared their context split it between them
 * instead, with nothing folded in between.
 */
function changedRanges(lines: DiffLine[], fixed: ReadonlySet<number>): [number, number][] {
	const ranges: Range[] = [];
	let i = 0;
	while (i < lines.length) {
		if (lines[i].origin === 'context') {
			i++;
			continue;
		}
		const first = i;
		let isFix = false;
		while (i < lines.length && lines[i].origin !== 'context') {
			const line = lines[i];
			if (line.origin === 'added' && line.new !== null && fixed.has(line.new)) isFix = true;
			i++;
		}
		const range: Range = {
			from: Math.max(0, first - CONTEXT),
			to: Math.min(lines.length, i + CONTEXT),
			first,
			end: i,
			fixed: isFix
		};
		const last = ranges[ranges.length - 1];
		if (last && range.from - last.to < MIN_FOLD) {
			if (last.fixed === range.fixed) {
				// Close enough that the gap would be too short to fold: one part.
				last.to = range.to;
				last.end = range.end;
				continue;
			}
			const boundary = last.end + Math.ceil((range.first - last.end) / 2);
			last.to = boundary;
			range.from = boundary;
		}
		ranges.push(range);
	}
	if (ranges.length > 0) {
		if (ranges[0].from < MIN_FOLD) ranges[0].from = 0;
		const last = ranges[ranges.length - 1];
		if (lines.length - last.to < MIN_FOLD) last.to = lines.length;
	}
	return ranges.map((range) => [range.from, range.to]);
}

/**
 * Cut a file into blocks.
 *
 * In `changes` the unchanged runs between the parts are folds, each named by
 * where it starts, and stay shown once expanded. In `whole` nothing is folded.
 * A file whose text did not change — a rename — is one fold, or one plain
 * block: its lines are there to read, but nothing in them is news.
 *
 * `fixed` are the new line numbers conflict fixes wrote, which are kept out of
 * the author's parts.
 */
export function blocksOf(
	lines: DiffLine[],
	scope: Scope,
	expanded: ReadonlySet<number> = new Set(),
	fixed: ReadonlySet<number> = new Set()
): Block[] {
	if (lines.length === 0) return [];
	const unchanged = (from: number, to: number): Block =>
		scope === 'whole' || expanded.has(from) ? { kind: 'plain', from, to } : { kind: 'fold', from, to };

	const ranges = changedRanges(lines, fixed);
	if (ranges.length === 0) return [unchanged(0, lines.length)];

	const blocks: Block[] = [];
	let at = 0;
	for (const [from, to] of ranges) {
		if (from > at) blocks.push(unchanged(at, from));
		blocks.push({ kind: 'hunk', from, to });
		at = to;
	}
	if (at < lines.length) blocks.push(unchanged(at, lines.length));
	return blocks;
}

/**
 * A file the host sent as a patch, when the pull request could not be fetched:
 * its hunks laid end to end, each a block. There is nothing between them to
 * fold or expand, because the host never sent it.
 */
export function blocksOfHunks(hunks: Hunk[]): { lines: DiffLine[]; blocks: Block[] } {
	const lines: DiffLine[] = [];
	const blocks: Block[] = [];
	for (const hunk of hunks) {
		const from = lines.length;
		lines.push(...hunk.lines);
		blocks.push({ kind: 'hunk', from, to: lines.length });
	}
	return { lines, blocks };
}

/**
 * The nearest line above `from` that starts at the margin with a name — what
 * `git diff` puts after a hunk's numbers to say where it is.
 */
function enclosing(lines: DiffLine[], from: number): string {
	for (let i = from - 1; i >= 0; i--) {
		const text = lines[i].text;
		if (/^[A-Za-z_$]/.test(text)) return text.length > 60 ? `${text.slice(0, 59)}…` : text;
	}
	return '';
}

/** `@@ -13,8 +13,10 @@  pub fn avatar` for the lines of one block. */
export function headerOf(lines: DiffLine[], block: Block): string {
	const count = (side: 'old' | 'new') => {
		let first: number | null = null;
		let n = 0;
		for (let i = block.from; i < block.to; i++) {
			const value = lines[i][side];
			if (value === null) continue;
			first ??= value;
			n++;
		}
		if (first === null) {
			// None on this side: git names the line before, or 0.
			for (let i = block.from - 1; i >= 0 && first === null; i--) first = lines[i][side];
			first ??= 0;
		}
		return `${first},${n}`;
	};
	const name = enclosing(lines, block.from);
	return `@@ -${count('old')} +${count('new')} @@${name ? `  ${name}` : ''}`;
}

/** `unchanged · 21–26`, by the new version's numbers. */
export function plainHeaderOf(lines: DiffLine[], block: Block): string {
	const first = lines[block.from].new ?? lines[block.from].old;
	const last = lines[block.to - 1].new ?? lines[block.to - 1].old;
	return first === last ? `unchanged · ${first}` : `unchanged · ${first}–${last}`;
}

/**
 * The look of a card: a changed part, a changed part written while fixing a
 * merge conflict (FEAT-092), or unchanged lines shown.
 */
export type Tone = 'hunk' | 'conflict' | 'plain';

/** One side of a conflict, shown under its card's header. */
export type SideName = 'main' | 'branch';

export type Row =
	| { kind: 'file'; key: string; path: string }
	| { kind: 'note'; key: string; path: string; text: string }
	| { kind: 'fold'; key: string; path: string; id: number; count: number }
	| {
			kind: 'head';
			key: string;
			path: string;
			chunk: string;
			tone: Tone;
			text: string;
			/** The conflict fix a changed part holds. */
			fix: ConflictFix | null;
	  }
	| { kind: 'side'; key: string; path: string; chunk: string; tone: Tone; fix: ConflictFix; side: SideName }
	| {
			kind: 'line';
			key: string;
			path: string;
			chunk: string;
			tone: Tone;
			/** The line's index in its file's lines. */
			index: number;
			line: DiffLine;
			/** Written while fixing a merge conflict. */
			fixed: boolean;
			/** The last row of its card. */
			last: boolean;
	  }
	| {
			kind: 'thread';
			key: string;
			path: string;
			chunk: string;
			tone: Tone;
			thread: Thread;
			last: boolean;
	  }
	| {
			/** A comment written here and not sent yet (FEAT-093). */
			kind: 'draft';
			key: string;
			path: string;
			chunk: string;
			tone: Tone;
			draft: PendingComment;
			last: boolean;
	  }
	| {
			/** The box a comment is being written in, under the last line it covers. */
			kind: 'composer';
			key: string;
			path: string;
			chunk: string;
			tone: Tone;
			from: number;
			to: number;
			last: boolean;
	  };

export type LineRow = Extract<Row, { kind: 'line' }>;

/** Where a comment is being written: a file and the lines it covers, by index. */
export interface Composing {
	path: string;
	from: number;
	to: number;
}

/** What is written on lines besides the host's threads (FEAT-093). */
export interface Notes {
	/** Pending comments by where they sit (`draftsByPlace`). */
	drafts?: Map<string, PendingComment[]>;
	composer?: Composing | null;
}

/** One file, ready to be laid out. */
export interface Laid {
	path: string;
	/** Null while the file is being read. */
	lines: DiffLine[] | null;
	blocks: Block[];
	/** What to say instead of lines: binary, too large, could not be read. */
	note?: string;
	/** What merges wrote in it while fixing conflicts (FEAT-092). */
	fixes?: ConflictFix[];
}

/** The fix a changed part holds: the first that wrote one of its added lines. */
function fixOf(lines: DiffLine[], block: Block, fixes: ConflictFix[]): ConflictFix | null {
	if (block.kind !== 'hunk' || fixes.length === 0) return null;
	const added = new Set<number>();
	for (let i = block.from; i < block.to; i++) {
		const line = lines[i];
		if (line.origin === 'added' && line.new !== null) added.add(line.new);
	}
	return fixes.find((fix) => fix.lines.some((n) => added.has(n))) ?? null;
}

/** The key a line's row has, which focus and jumps hold on to. */
export function lineKey(path: string, index: number): string {
	return `${path}\n${index}`;
}

/** Where a line can carry a thread: its new number, or its old one. */
function placesOf(path: string, line: DiffLine): string[] {
	const places: string[] = [];
	if (line.new !== null) places.push(placeOf(path, 'RIGHT', line.new));
	if (line.old !== null && line.origin !== 'added') places.push(placeOf(path, 'LEFT', line.old));
	return places;
}

/**
 * Rows for files, in order: a header per file, then its blocks.
 *
 * `threads` are by where they sit (`threadsByPlace`); each is drawn under the
 * line it is on, inside that line's card. A thread on a folded line is in the
 * Conversation card until the fold is opened.
 *
 * A changed part that holds a conflict fix is a card of its own tone, and
 * `sides` says, by card, which side of the conflict is open under its header.
 */
export function rowsOf(
	files: Laid[],
	threads: Map<string, Thread[]> = new Map(),
	sides: ReadonlyMap<string, SideName> = new Map(),
	notes: Notes = {}
): Row[] {
	const drafts = notes.drafts ?? new Map<string, PendingComment[]>();
	const composer = notes.composer ?? null;
	const rows: Row[] = [];
	for (const file of files) {
		const { path } = file;
		rows.push({ kind: 'file', key: `${path}\nfile`, path });
		if (file.note) {
			rows.push({ kind: 'note', key: `${path}\nnote`, path, text: file.note });
			continue;
		}
		if (file.lines === null) {
			rows.push({ kind: 'note', key: `${path}\nreading`, path, text: 'Reading…' });
			continue;
		}
		const lines = file.lines;
		const fixes = file.fixes ?? [];
		const fixed = new Set(fixes.flatMap((fix) => fix.lines));
		if (file.blocks.length === 0) {
			rows.push({ kind: 'note', key: `${path}\nempty`, path, text: 'The file is empty.' });
			continue;
		}
		for (const block of file.blocks) {
			if (block.kind === 'fold') {
				const count = block.to - block.from;
				rows.push({ kind: 'fold', key: `${path}\nfold\n${block.from}`, path, id: block.from, count });
				continue;
			}
			const chunk = `${path}\n${block.from}`;
			const fix = fixOf(lines, block, fixes);
			const tone: Tone = fix ? 'conflict' : block.kind;
			const text = block.kind === 'hunk' ? headerOf(lines, block) : plainHeaderOf(lines, block);
			rows.push({ kind: 'head', key: `${chunk}\nhead`, path, chunk, tone, text, fix });
			const side = sides.get(chunk);
			if (fix && side) rows.push({ kind: 'side', key: `${chunk}\nside`, path, chunk, tone, fix, side });
			for (let index = block.from; index < block.to; index++) {
				const line = lines[index];
				const isFix = line.origin === 'added' && line.new !== null && fixed.has(line.new);
				rows.push({
					kind: 'line',
					key: lineKey(path, index),
					path,
					chunk,
					tone,
					index,
					line,
					fixed: isFix,
					last: false
				});
				for (const place of placesOf(path, line)) {
					for (const thread of threads.get(place) ?? []) {
						rows.push({
							kind: 'thread',
							key: `${path}\nthread\n${thread.id}`,
							path,
							chunk,
							tone,
							thread,
							last: false
						});
					}
				}
				for (const place of placesOf(path, line)) {
					for (const draft of drafts.get(place) ?? []) {
						rows.push({ kind: 'draft', key: `${path}\ndraft\n${draft.id}`, path, chunk, tone, draft, last: false });
					}
				}
				if (composer && composer.path === path && index === Math.max(composer.from, composer.to)) {
					const { from, to } = composer;
					rows.push({ kind: 'composer', key: `${path}\ncomposer`, path, chunk, tone, from, to, last: false });
				}
			}
			const end = rows[rows.length - 1];
			if (end.kind === 'line' || end.kind === 'thread' || end.kind === 'draft' || end.kind === 'composer') {
				end.last = true;
			}
		}
	}
	return rows;
}

/** The row a thread is drawn under, or -1 when it is not drawn. */
export function rowOfThread(rows: Row[], thread: Thread): number {
	return rows.findIndex((row) => row.kind === 'thread' && row.thread.id === thread.id);
}

/** The line rows' indices, in order: what `j` and `k` step through. */
export function lineRows(rows: Row[]): number[] {
	const out: number[] = [];
	rows.forEach((row, index) => {
		if (row.kind === 'line') out.push(index);
	});
	return out;
}
