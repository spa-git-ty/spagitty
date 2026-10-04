// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import type { DiffLine } from '../types';
import { diffWords, pairWords, tokens } from './words';

const changed = (segments: { text: string; changed: boolean }[]) =>
	segments.filter((segment) => segment.changed).map((segment) => segment.text);

describe('tokens', () => {
	it('splits words, spaces and punctuation, keeping everything', () => {
		const text = '    let key = cache_key(email, size);';
		expect(tokens(text).join('')).toBe(text);
		expect(tokens('a.b(c)')).toEqual(['a', '.', 'b', '(', 'c', ')']);
	});
});

describe('diffWords', () => {
	it('marks only the words that changed', () => {
		const [before, after] = diffWords(
			'    return Ok(hit.clone());',
			'    return Avatar::from_file(&path);'
		)!;
		// One mark on each side, on the call and not on `return`.
		expect(changed(before)).toHaveLength(1);
		expect(changed(before)[0]).toContain('hit.clone');
		expect(changed(after)).toHaveLength(1);
		expect(changed(after)[0]).toContain('Avatar::from_file');
		expect(changed(before).join('') + changed(after).join('')).not.toContain('return');
		expect(before.map((s) => s.text).join('')).toBe('    return Ok(hit.clone());');
	});

	it('joins two changed words across the space between them', () => {
		const [, after] = diffWords('a b c', 'a x y c')!;
		expect(changed(after)).toEqual(['x y']);
	});

	it('marks nothing when the line was rewritten rather than edited', () => {
		expect(diffWords('use std::fs;', 'fn main() {}')).toBeNull();
		expect(diffWords('', 'something')).toBeNull();
	});

	it('leaves a pair too long to compare cheaply unmarked', () => {
		const long = Array.from({ length: 500 }, (_, i) => `w${i}`).join(' ');
		expect(diffWords(long, long + ' more')).toBeNull();
	});
});

describe('pairWords', () => {
	const line = (origin: DiffLine['origin'], text: string): DiffLine => ({
		origin,
		old: null,
		new: null,
		text
	});

	it('pairs each removed line with the added line in the same place', () => {
		const lines = [
			line('context', 'fn x() {'),
			line('removed', '    if let Some(hit) = MEMORY.get(&key) {'),
			line('removed', '        return Ok(hit);'),
			line('added', '    if let Some(hit) = disk.get(&key) {'),
			line('added', '        return Ok(hit);'),
			line('added', '    extra();'),
			line('context', '}')
		];
		const marked = pairWords(lines);
		expect(changed(marked.get(1)!)).toEqual(['MEMORY']);
		expect(changed(marked.get(3)!)).toEqual(['disk']);
		// Identical lines have nothing changed in them.
		expect(changed(marked.get(4)!)).toEqual([]);
		// The leftover added line has no partner.
		expect(marked.has(5)).toBe(false);
		expect(marked.has(0)).toBe(false);
	});
});

describe('pairWords, by likeness', () => {
	const line = (origin: DiffLine['origin'], text: string): DiffLine => ({
		origin,
		old: null,
		new: null,
		text
	});

	it('compares a removed line with the new line it most resembles', () => {
		const lines = [
			line('removed', '    if let Some(hit) = MEMORY.lock().get(&key) {'),
			line('added', '    let path = cache_dir()?.join(format!("{key}.png"));'),
			line('added', '    if let Some(hit) = disk.lock().get(&key) {')
		];
		const marked = pairWords(lines);
		expect(marked.has(1)).toBe(false);
		expect(changed(marked.get(0)!)).toEqual(['MEMORY']);
		expect(changed(marked.get(2)!)).toEqual(['disk']);
	});
});
