// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A file's colour, as it arrives and as it is drawn (FEAT-131).
 *
 * Tree-sitter runs in Rust and sends, for each line, pairs of *token class,
 * length* — no text, because the screen already has the file. This turns a
 * line's text and its pairs into the same runs Diff draws, so one set of
 * `tok-*` classes colours both screens.
 *
 * A language with no grammar compiled in is read by Diff's own highlighter
 * instead, which gives the same runs by another road.
 */

import type { Token, TokenType } from '$lib/diff/highlight';

/**
 * The token classes, in the order their numbers cross the wire.
 *
 * `crates/spagitty-core/src/highlight.rs` carries the same list as `CLASSES`,
 * and `colour.test.ts` reads that file to keep the two in step.
 */
export const CLASSES: readonly TokenType[] = [
	'plain',
	'keyword',
	'string',
	'number',
	'comment',
	'fn',
	'type',
	'operator',
	'punctuation',
	'tag',
	'attr',
	'meta'
];

/**
 * A line's tokens from its text and its `[class, length, …]` pairs.
 *
 * The pairs are trusted to cover the line, and not relied on to: whatever
 * they leave is drawn plain, and whatever they claim past the end is ignored.
 * A line and its colour can be a frame apart, and a run cut wrong is better
 * than a missing word.
 */
export function tokensOf(text: string, runs: readonly number[] | undefined): Token[] {
	if (!runs || runs.length === 0) return text === '' ? [] : [{ type: 'plain', text }];

	const out: Token[] = [];
	let at = 0;
	for (let i = 0; i + 1 < runs.length && at < text.length; i += 2) {
		const length = runs[i + 1];
		if (length <= 0) continue;
		const piece = text.slice(at, at + length);
		at += length;
		if (piece !== '') out.push({ type: CLASSES[runs[i]] ?? 'plain', text: piece });
	}
	if (at < text.length) out.push({ type: 'plain', text: text.slice(at) });
	return out;
}

/** A run of a line as it is drawn: its colour, and whether a search found it. */
export interface Piece {
	type: TokenType;
	text: string;
	hit: boolean;
}

/**
 * A line's tokens cut at a search match, `[start, end)` in UTF-16 units.
 *
 * The match and the tokens cut the line in different places, and every
 * boundary of either is a boundary here — the arrangement `paint` makes for a
 * diff's changed words.
 */
export function pieces(tokens: readonly Token[], hit: readonly [number, number] | null): Piece[] {
	if (!hit || hit[1] <= hit[0]) return tokens.map((token) => ({ ...token, hit: false }));

	const [start, end] = hit;
	const out: Piece[] = [];
	let at = 0;
	for (const token of tokens) {
		const from = at;
		const to = at + token.text.length;
		at = to;
		// The three parts of a token: before the match, in it, after it.
		const cuts = [from, Math.min(Math.max(start, from), to), Math.min(Math.max(end, from), to), to];
		for (let i = 0; i < 3; i++) {
			const text = token.text.slice(cuts[i] - from, cuts[i + 1] - from);
			if (text !== '') out.push({ type: token.type, text, hit: i === 1 });
		}
	}
	return out;
}

/**
 * A file's lines, counted the way a file viewer counts them: a trailing
 * newline ends the last line rather than starting an empty one, and a `\r`
 * before a `\n` is part of the terminator. The same rule the highlighter and
 * `git blame` keep, so line 14 is line 14 to all three.
 */
export function linesOf(text: string): string[] {
	if (text === '') return [];
	const lines = text.split('\n');
	if (lines[lines.length - 1] === '') lines.pop();
	for (let i = 0; i < lines.length; i++) {
		if (lines[i].endsWith('\r')) lines[i] = lines[i].slice(0, -1);
	}
	return lines;
}
