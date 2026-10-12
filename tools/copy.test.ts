// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The words people read must be the words people use (TASK-061).
 *
 * *Land* for *merge* and *gate* for a point where an agent waits to be
 * approved are the vocabulary of the people who built the thing, not of the
 * people who use it. They stay as identifiers in the code (`Gate`, `lands`,
 * `LastAct.land`) and as the names of the CI gates, where nobody reads them in
 * the application. This reads what a person can see: the strings in the
 * frontend and the text in the markup, and fails when either word comes back.
 */

import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const WORD = /\b(lands?|landed|landing|gates?|gated|gating)\b/i;

function walk(directory: string, found: string[] = []): string[] {
	for (const entry of readdirSync(directory, { withFileTypes: true })) {
		const path = join(directory, entry.name);
		if (entry.isDirectory()) {
			if (entry.name === 'node_modules' || entry.name.startsWith('.') || entry.name === 'testing') continue;
			walk(path, found);
		} else if (/\.(ts|svelte)$/.test(entry.name) && !/\.test\.ts$/.test(entry.name)) {
			found.push(path);
		}
	}
	return found;
}

/** Comments are for the people who build it; they keep their vocabulary. */
function withoutComments(text: string): string {
	return text
		.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, ' '))
		.replace(/<!--[\s\S]*?-->/g, (m) => m.replace(/[^\n]/g, ' '))
		.replace(/(^|[^:'"`\\])\/\/[^\n]*/g, (m, before: string) => before + ' '.repeat(m.length - before.length));
}

/**
 * A string that is only an identifier (`'land'`, `'lands'`) is a value the code
 * passes around, not words. Prose has a space in it, and a label is capitalised.
 */
function isCopy(literal: string): boolean {
	const inner = literal.slice(1, -1).replace(/\$\{[^}]*\}|\{[^}]*\}/g, ' ');
	if (!WORD.test(inner)) return false;
	return /\s/.test(inner) || /^[A-Z]/.test(inner);
}

function offences(file: string): string[] {
	const raw = readFileSync(file, 'utf8').replace(/\r\n/g, '\n');
	const lines = withoutComments(raw).split('\n');
	const original = raw.split('\n');
	const svelte = file.endsWith('.svelte');
	const found: string[] = [];
	let inScript = false;
	let inStyle = false;
	lines.forEach((text, index) => {
		// A class name is never read.
		const line = text.replace(/class(?::[\w-]+)?="[^"]*"/g, ' ');
		if (svelte && /<script/.test(line)) inScript = true;
		if (svelte && /<style/.test(line)) inStyle = true;
		if (!inStyle) {
			const literals = line.match(/'(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*"|`(?:[^`\\]|\\.)*`/g) ?? [];
			const prose =
				svelte && !inScript
					? line
							.replace(/\{[^}]*\}/g, ' ')
							.replace(/<[^>]*>/g, ' ')
							.replace(/[\w:.-]+=(?:"[^"]*"|'[^']*')/g, ' ')
							.replace(/[\w:.-]+=/g, ' ')
					: '';
			if (literals.some(isCopy) || (prose && WORD.test(prose))) {
				found.push(`${file}:${index + 1}: ${original[index].trim()}`);
			}
		}
		if (svelte && /<\/script>/.test(line)) inScript = false;
		if (svelte && /<\/style>/.test(line)) inStyle = false;
	});
	return found;
}

describe('the words in the application', () => {
	it('say merge and approval, not land and gate', () => {
		const files = walk(join(process.cwd(), 'src'));
		expect(files.length).toBeGreaterThan(100);
		expect(files.flatMap(offences)).toEqual([]);
	});

	it('are checked against the approved wording too', () => {
		const branding = readFileSync(join(process.cwd(), 'docs', 'branding.md'), 'utf8');
		expect(branding).not.toMatch(/\blands\b/i);
	});
});
