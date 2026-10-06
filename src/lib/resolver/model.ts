// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The three-column resolver's model (FEAT-102), shared by Merger (1S) and
 * Conflicts (1D).
 *
 * A conflicted file is its merged text with diff3 markers, split into lines,
 * and each marker region is a choice: A, B, both in either order, single
 * lines from each, or text typed by hand. Everything the columns show — the
 * result's lines and where each came from, the line numbers that follow the
 * choices, the context around each region — is a function of the file and the
 * choices made so far, so it is worked out here and tested here.
 *
 * Lines are split on `\n` alone and joined back the same way, so a file whose
 * lines end in `\r\n` keeps them.
 */

import type { ConflictKind } from '../types';

export type SideKey = 'a' | 'b';

/** What was chosen for one region. */
export type Choice =
	| { mode: 'a' }
	| { mode: 'b' }
	/** Both, A's lines first. */
	| { mode: 'ab' }
	/** Both, B's lines first. */
	| { mode: 'ba' }
	/** A's ticked lines, then B's. */
	| { mode: 'pick'; a: boolean[]; b: boolean[] }
	/** Exactly this text. */
	| { mode: 'edit'; text: string };

export type Mode = Choice['mode'];

/** One marker region. */
export interface Region {
	index: number;
	/** The `<<<<<<<` and `>>>>>>>` lines, counted from 0 in the merged text. */
	start: number;
	end: number;
	a: string[];
	/** What the merge base had here; null when the markers do not say. */
	base: string[] | null;
	b: string[];
	/** The first line of A's lines in A's file, from 1, when it is known. */
	aLine: number | null;
	bLine: number | null;
	/** `e41c0b7 · fix(chrome): hidden repositories…`, when it is known. */
	aFrom: string | null;
	bFrom: string | null;
}

/** One conflicted file, as the resolver reads it. */
export interface ResolverFile {
	path: string;
	kind: ConflictKind;
	/** The merged text's lines; empty for a file that conflicts as a whole. */
	merged: string[];
	/** The merged text ended with a newline. */
	eol: boolean;
	/** Each side's whole file, when it has one that is text. */
	aLines: string[] | null;
	bLines: string[] | null;
	/** True when the file has no marker regions: one choice, A or B, whole. */
	whole: boolean;
	/** Each side exists: a side that does not is a deletion. */
	aExists: boolean;
	bExists: boolean;
	/** One side or the other is binary or too large to show. */
	opaque: boolean;
	regions: Region[];
}

/** Where a region came from, as the backend found it. */
export interface RegionSource {
	index: number;
	aLine: number | null;
	bLine: number | null;
	aFrom: string | null;
	bFrom: string | null;
}

const OPEN = '<<<<<<<';
const BASE = '|||||||';
const SPLIT = '=======';
const CLOSE = '>>>>>>>';

/** Text into lines, and whether it ended with a newline. */
export function splitLines(text: string): { lines: string[]; eol: boolean } {
	if (text === '') return { lines: [], eol: false };
	const eol = text.endsWith('\n');
	const lines = (eol ? text.slice(0, -1) : text).split('\n');
	return { lines, eol };
}

/**
 * Every marker region in `lines`, as strictly as the backend reads them
 * (`conflicts::regions`): open, an optional base, split, close, each at the
 * start of its own line. An unterminated region ends the reading.
 */
export function parseRegions(lines: string[]): Omit<Region, 'aLine' | 'bLine' | 'aFrom' | 'bFrom'>[] {
	const found: Omit<Region, 'aLine' | 'bLine' | 'aFrom' | 'bFrom'>[] = [];
	let i = 0;
	while (i < lines.length) {
		if (!lines[i].startsWith(OPEN)) {
			i += 1;
			continue;
		}
		const start = i;
		const a: string[] = [];
		const b: string[] = [];
		let base: string[] | null = null;
		let phase: 'a' | 'base' | 'b' = 'a';
		let end = -1;
		for (i += 1; i < lines.length; i += 1) {
			const line = lines[i];
			if (line.startsWith(BASE) && phase === 'a') {
				phase = 'base';
				base = [];
			} else if (line.startsWith(SPLIT) && phase !== 'b') {
				phase = 'b';
			} else if (line.startsWith(CLOSE)) {
				if (phase === 'b') end = i;
				break;
			} else if (phase === 'a') a.push(line);
			else if (phase === 'base') base!.push(line);
			else b.push(line);
		}
		if (end < 0) break;
		found.push({ index: found.length, start, end, a, base, b });
		i = end + 1;
	}
	return found;
}

/**
 * Where `block` sits in `file`, from `from` on, as a 1-based line: the first
 * match preceded by the context's last line, else the first match. An empty
 * block sits just after the context. The same rule as the backend's.
 */
export function locate(
	file: string[],
	before: string[],
	block: string[],
	from: number
): { line: number; next: number } | null {
	const same = (at: number, lines: string[]) =>
		at + lines.length <= file.length && lines.every((line, k) => file[at + k] === line);
	let found: number | null = null;
	if (block.length === 0) {
		if (before.length === 0) found = from;
		else {
			for (let at = from; at + before.length <= file.length; at += 1) {
				if (same(at, before)) {
					found = at + before.length;
					break;
				}
			}
		}
	} else {
		let first: number | null = null;
		for (let at = from; at < file.length; at += 1) {
			if (!same(at, block)) continue;
			if (first === null) first = at;
			const last = before[before.length - 1];
			if (last === undefined || (at > 0 && file[at - 1] === last)) {
				found = at;
				break;
			}
		}
		found ??= first;
	}
	if (found === null) return null;
	return { line: found + 1, next: found + block.length };
}

/** Up to `count` merged lines before region `index`, stopping at the one before. */
function contextBefore(lines: string[], regions: { start: number; end: number }[], index: number, count = 3): string[] {
	const floor = index === 0 ? 0 : regions[index - 1].end + 1;
	return lines.slice(Math.max(floor, regions[index].start - count), regions[index].start);
}

export interface FileInput {
	path: string;
	kind: ConflictKind;
	/** The merged text with markers; null when the file conflicts as a whole. */
	merged: string | null;
	a: string | null;
	b: string | null;
	aExists: boolean;
	bExists: boolean;
	opaque?: boolean;
	/** What the backend found per region; located here when absent. */
	sources?: RegionSource[];
}

/** A conflicted file, read for the resolver. */
export function readFile(input: FileInput): ResolverFile {
	const merged = input.merged === null ? { lines: [], eol: false } : splitLines(input.merged);
	const aLines = input.a === null ? null : splitLines(input.a).lines;
	const bLines = input.b === null ? null : splitLines(input.b).lines;
	const parsed = parseRegions(merged.lines);
	let aFrom = 0;
	let bFrom = 0;
	const regions: Region[] = parsed.map((region) => {
		const source = input.sources?.find((s) => s.index === region.index);
		const before = contextBefore(merged.lines, parsed, region.index);
		let aLine = source?.aLine ?? null;
		let bLine = source?.bLine ?? null;
		if (!source && aLines) {
			const at = locate(aLines, before, region.a, aFrom);
			if (at) {
				aFrom = at.next;
				if (region.a.length > 0) aLine = at.line;
			}
		}
		if (!source && bLines) {
			const at = locate(bLines, before, region.b, bFrom);
			if (at) {
				bFrom = at.next;
				if (region.b.length > 0) bLine = at.line;
			}
		}
		return { ...region, aLine, bLine, aFrom: source?.aFrom ?? null, bFrom: source?.bFrom ?? null };
	});
	const whole = regions.length === 0;
	return {
		path: input.path,
		kind: input.kind,
		merged: merged.lines,
		eol: merged.eol,
		aLines,
		bLines,
		whole,
		aExists: input.aExists,
		bExists: input.bExists,
		opaque: input.opaque ?? false,
		regions: whole ? [wholeRegion(aLines, bLines)] : regions
	};
}

/** A file that conflicts as a whole is one region: every line of each side. */
function wholeRegion(a: string[] | null, b: string[] | null): Region {
	return { index: 0, start: 0, end: 0, a: a ?? [], base: null, b: b ?? [], aLine: a ? 1 : null, bLine: b ? 1 : null, aFrom: null, bFrom: null };
}

export interface ResultLine {
	text: string;
	from: SideKey | 'mine';
}

/** The lines a choice puts in the result, or null for no choice yet. */
export function chosenLines(region: Region, choice: Choice | null): ResultLine[] | null {
	if (!choice) return null;
	const take = (lines: string[], from: SideKey) => lines.map((text) => ({ text, from }));
	switch (choice.mode) {
		case 'a':
			return take(region.a, 'a');
		case 'b':
			return take(region.b, 'b');
		case 'ab':
			return [...take(region.a, 'a'), ...take(region.b, 'b')];
		case 'ba':
			return [...take(region.b, 'b'), ...take(region.a, 'a')];
		case 'pick':
			return [
				...take(region.a, 'a').filter((_, i) => choice.a[i]),
				...take(region.b, 'b').filter((_, i) => choice.b[i])
			];
		case 'edit':
			return editedLines(choice.text).map((text) => ({ text, from: 'mine' as const }));
	}
}

/** What a hand edit puts in: its lines, none for an empty box. */
export function editedLines(text: string): string[] {
	return splitLines(text).lines;
}

/** Whether a choice keeps line `i` of `side`; null before anything is chosen. */
export function kept(choice: Choice | null, side: SideKey, i: number): boolean | null {
	if (!choice || choice.mode === 'edit') return null;
	switch (choice.mode) {
		case 'ab':
		case 'ba':
			return true;
		case 'a':
			return side === 'a';
		case 'b':
			return side === 'b';
		case 'pick':
			return Boolean(side === 'a' ? choice.a[i] : choice.b[i]);
	}
}

/** Every line of both sides ticked: where Pick lines starts. */
export function pickAll(region: Region): Choice {
	return { mode: 'pick', a: region.a.map(() => true), b: region.b.map(() => true) };
}

/** Pick lines with one line's tick turned over. */
export function toggled(choice: Choice & { mode: 'pick' }, side: SideKey, i: number): Choice {
	const next = { mode: 'pick' as const, a: [...choice.a], b: [...choice.b] };
	next[side][i] = !next[side][i];
	return next;
}

/** What Edit by hand starts from: the result so far, or both sides. */
export function editSeed(region: Region, choice: Choice | null): Choice {
	const lines = chosenLines(region, choice) ?? [...region.a, ...region.b].map((text) => ({ text }));
	return { mode: 'edit', text: lines.map((line) => line.text).join('\n') };
}

/** The file as it will land, or null while a region is unresolved. */
export function resultText(file: ResolverFile, choices: (Choice | null)[]): string | null {
	if (file.whole) return null;
	const out: string[] = [];
	let at = 0;
	for (const region of file.regions) {
		out.push(...file.merged.slice(at, region.start));
		const lines = chosenLines(region, choices[region.index] ?? null);
		if (!lines) return null;
		out.push(...lines.map((line) => line.text));
		at = region.end + 1;
	}
	out.push(...file.merged.slice(at));
	if (out.length === 0) return '';
	return out.join('\n') + (file.eol ? '\n' : '');
}

export interface Numbered {
	n: number | null;
	text: string;
}

/** Where one region is drawn: its context in each column and its first result line. */
export interface RegionLayout {
	/** The result's line number for the region's first line. */
	resultStart: number;
	before: Numbered[];
	after: Numbered[];
	aBefore: Numbered[];
	aAfter: Numbered[];
	bBefore: Numbered[];
	bAfter: Numbered[];
}

/**
 * Every region's context and numbering, for the choices made so far. The
 * result's numbers follow the choices: a region resolved to three lines pushes
 * every later line of the file down by however many it added.
 */
export function layout(file: ResolverFile, choices: (Choice | null)[], context = 3): RegionLayout[] {
	if (file.whole) {
		return [{ resultStart: 1, before: [], after: [], aBefore: [], aAfter: [], bBefore: [], bAfter: [] }];
	}
	const numbers: (number | null)[] = new Array(file.merged.length).fill(null);
	const starts: number[] = [];
	let n = 1;
	let at = 0;
	for (const region of file.regions) {
		for (; at < region.start; at += 1) numbers[at] = n++;
		starts.push(n);
		n += chosenLines(region, choices[region.index] ?? null)?.length ?? 0;
		at = region.end + 1;
	}
	for (; at < file.merged.length; at += 1) numbers[at] = n++;

	return file.regions.map((region, k) => {
		const floor = k === 0 ? 0 : file.regions[k - 1].end + 1;
		const ceiling = k === file.regions.length - 1 ? file.merged.length : file.regions[k + 1].start;
		const beforeIdx = range(Math.max(floor, region.start - context), region.start);
		const afterIdx = range(region.end + 1, Math.min(ceiling, region.end + 1 + context));
		const merged = (idx: number[]) => idx.map((i) => ({ n: numbers[i], text: file.merged[i] }));
		const plain = (idx: number[]) => idx.map((i) => ({ n: null, text: file.merged[i] }));
		const side = (lines: string[] | null, line: number | null, length: number, before: boolean): Numbered[] => {
			if (!lines || line === null) return plain(before ? beforeIdx : afterIdx);
			const from = before ? Math.max(0, line - 1 - context) : line - 1 + length;
			const to = before ? line - 1 : Math.min(lines.length, line - 1 + length + context);
			return range(from, to).map((i) => ({ n: i + 1, text: lines[i] }));
		};
		return {
			resultStart: starts[k],
			before: merged(beforeIdx),
			after: merged(afterIdx),
			aBefore: side(file.aLines, region.aLine, region.a.length, true),
			aAfter: side(file.aLines, region.aLine, region.a.length, false),
			bBefore: side(file.bLines, region.bLine, region.b.length, true),
			bAfter: side(file.bLines, region.bLine, region.b.length, false)
		};
	});
}

function range(from: number, to: number): number[] {
	const out: number[] = [];
	for (let i = from; i < to; i += 1) out.push(i);
	return out;
}

export interface Names {
	a: string;
	b: string;
}

/** The status chip: what was chosen, in words. */
export function statusLabel(choice: Choice | null, names: Names): string {
	if (!choice) return 'Unresolved';
	switch (choice.mode) {
		case 'a':
			return `Took ${names.a}`;
		case 'b':
			return `Took ${names.b}`;
		case 'ab':
			return `Both, ${names.a} first`;
		case 'ba':
			return `Both, ${names.b} first`;
		case 'pick':
			return 'Picked lines';
		case 'edit':
			return 'Edited by hand';
	}
}

/** The side badges a choice shows, in order. */
export function badgesOf(choice: Choice | null): SideKey[] {
	switch (choice?.mode) {
		case 'a':
			return ['a'];
		case 'b':
			return ['b'];
		case 'ab':
			return ['a', 'b'];
		case 'ba':
			return ['b', 'a'];
		default:
			return [];
	}
}

/** One sentence on why this region conflicts. */
export function why(region: Region, file: ResolverFile, names: Names): string {
	if (file.whole) {
		if (!file.aExists) return `${names.a} deleted this file; ${names.b} changed it.`;
		if (!file.bExists) return `${names.b} deleted this file; ${names.a} changed it.`;
		if (file.opaque) return 'Both changed this file, and it cannot be merged line by line.';
		return 'Both changed this file in ways git cannot put together.';
	}
	if (region.base !== null && region.base.length === 0) return 'Both added lines at the same spot.';
	if (region.a.length === 0) return `${names.a} removed these lines; ${names.b} changed them.`;
	if (region.b.length === 0) return `${names.b} removed these lines; ${names.a} changed them.`;
	if (region.a.length === 1 && region.b.length === 1) return 'Both rewrote the same line.';
	return 'Both changed these lines since they split.';
}

/** `2 lines from main, 3 lines from feat/tab-drag. Pick below, or type your own.` */
export function placeholder(region: Region, names: Names): string {
	const lines = (count: number) => `${count} ${count === 1 ? 'line' : 'lines'}`;
	return `${lines(region.a.length)} from ${names.a}, ${lines(region.b.length)} from ${names.b}. Pick below, or type your own.`;
}

/** The file's line to name a region by: A's, else where it lands. */
export function lineOf(region: Region, place: RegionLayout): number {
	return region.aLine ?? place.resultStart;
}

/** How many regions, and how many have a choice. */
export function counts(files: ResolverFile[], choices: Record<string, (Choice | null)[]>): { total: number; resolved: number } {
	let total = 0;
	let resolved = 0;
	for (const file of files) {
		for (const region of file.regions) {
			total += 1;
			if (choices[file.path]?.[region.index]) resolved += 1;
		}
	}
	return { total, resolved };
}

/**
 * What lands at a file, once every region of it is chosen: its text, or for a
 * file that conflicts as a whole, one side whole — which, for a side that
 * deleted it, deletes it.
 */
export function landing(
	file: ResolverFile,
	choices: (Choice | null)[]
): { path: string; text: string } | { path: string; take: 'ours' | 'theirs' } | null {
	if (file.whole) {
		const choice = choices[0];
		if (!choice) return null;
		if (choice.mode === 'a') return { path: file.path, take: 'ours' };
		if (choice.mode === 'b') return { path: file.path, take: 'theirs' };
		if (choice.mode === 'edit') return { path: file.path, text: choice.text === '' ? '' : choice.text.replace(/\n?$/, '\n') };
		return null;
	}
	const text = resultText(file, choices);
	return text === null ? null : { path: file.path, text };
}

/**
 * A short fingerprint of a region's sides, so a choice kept from an earlier
 * visit is only applied to the same conflict, not to one that moved there.
 */
export function fingerprint(region: Region): string {
	return hash([region.a.join('\n'), region.base?.join('\n') ?? '', region.b.join('\n')].join('\u0000'));
}

/** FNV-1a, 32 bits, as eight hex digits. */
export function hash(text: string): string {
	let h = 0x811c9dc5;
	for (let i = 0; i < text.length; i += 1) {
		h ^= text.charCodeAt(i);
		h = Math.imul(h, 0x01000193) >>> 0;
	}
	return h.toString(16).padStart(8, '0');
}

/** The choices that make every region of a file one side. */
export function allFrom(file: ResolverFile, side: SideKey): Choice[] {
	return file.regions.map(() => ({ mode: side }));
}

/** Which choices a region offers: a file that conflicts as a whole is one side or the other. */
export function offered(file: ResolverFile): Mode[] {
	if (!file.whole) return ['a', 'b', 'ab', 'ba', 'pick', 'edit'];
	if (file.opaque || !file.aExists || !file.bExists) return ['a', 'b'];
	return ['a', 'b', 'edit'];
}
