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

import type { DiffLine, Hunk } from '../types';
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

/** The changed parts of `lines`, with a little context, as index ranges. */
function changedRanges(lines: DiffLine[]): [number, number][] {
	const ranges: [number, number][] = [];
	for (let i = 0; i < lines.length; i++) {
		if (lines[i].origin === 'context') continue;
		const from = Math.max(0, i - CONTEXT);
		const to = Math.min(lines.length, i + CONTEXT + 1);
		const last = ranges[ranges.length - 1];
		// Close enough that the gap would be too short to fold: one part.
		if (last && from - last[1] < MIN_FOLD) last[1] = to;
		else ranges.push([from, to]);
	}
	if (ranges.length > 0) {
		if (ranges[0][0] < MIN_FOLD) ranges[0][0] = 0;
		const last = ranges[ranges.length - 1];
		if (lines.length - last[1] < MIN_FOLD) last[1] = lines.length;
	}
	return ranges;
}

/**
 * Cut a file into blocks.
 *
 * In `changes` the unchanged runs between the parts are folds, each named by
 * where it starts, and stay shown once expanded. In `whole` nothing is folded.
 * A file whose text did not change — a rename — is one fold, or one plain
 * block: its lines are there to read, but nothing in them is news.
 */
export function blocksOf(lines: DiffLine[], scope: Scope, expanded: ReadonlySet<number> = new Set()): Block[] {
	if (lines.length === 0) return [];
	const unchanged = (from: number, to: number): Block =>
		scope === 'whole' || expanded.has(from) ? { kind: 'plain', from, to } : { kind: 'fold', from, to };

	const ranges = changedRanges(lines);
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

/** The look of a card: a changed part, or unchanged lines shown. */
export type Tone = 'hunk' | 'plain';

export type Row =
	| { kind: 'file'; key: string; path: string }
	| { kind: 'note'; key: string; path: string; text: string }
	| { kind: 'fold'; key: string; path: string; id: number; count: number }
	| { kind: 'head'; key: string; path: string; chunk: string; tone: Tone; text: string }
	| {
			kind: 'line';
			key: string;
			path: string;
			chunk: string;
			tone: Tone;
			/** The line's index in its file's lines. */
			index: number;
			line: DiffLine;
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
	  };

export type LineRow = Extract<Row, { kind: 'line' }>;

/** One file, ready to be laid out. */
export interface Laid {
	path: string;
	/** Null while the file is being read. */
	lines: DiffLine[] | null;
	blocks: Block[];
	/** What to say instead of lines: binary, too large, could not be read. */
	note?: string;
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
 */
export function rowsOf(files: Laid[], threads: Map<string, Thread[]> = new Map()): Row[] {
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
			const tone: Tone = block.kind;
			const text = block.kind === 'hunk' ? headerOf(lines, block) : plainHeaderOf(lines, block);
			rows.push({ kind: 'head', key: `${chunk}\nhead`, path, chunk, tone, text });
			for (let index = block.from; index < block.to; index++) {
				const line = lines[index];
				rows.push({ kind: 'line', key: lineKey(path, index), path, chunk, tone, index, line, last: false });
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
			}
			const end = rows[rows.length - 1];
			if (end.kind === 'line' || end.kind === 'thread') end.last = true;
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
