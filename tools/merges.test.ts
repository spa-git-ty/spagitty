// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Every merge is made in Merger (FEAT-117).
 *
 * The calls that write a merge — asking a host to merge a pull request,
 * merging a branch, merging a Farm task — are in Merger's store and nowhere
 * else in the frontend. The other screens open Merger on what they came for.
 * This reads the sources, so a merge made somewhere else is a test failure and
 * not something a reader finds later.
 */

import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const MERGES = [
	/\bmergePullRequest\s*\(/,
	/\bmergerLand\s*\(/,
	/\bmergerRebaseFinish\s*\(/,
	/\bmergeTask\s*\(/,
	/\bintegrate\s*\(/,
	/\.merge\(\s*['"]/ // requests.merge('squash'), the old pull request path
];

/** Merger itself, its screen, and the files that define the calls. */
const MERGER = ['src/lib/merger/', 'src/routes/merge/'];
const DEFINING = new Set(['src/lib/api.ts', 'src/lib/farm/api.ts']);

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

describe('where merges are made', () => {
	it('is Merger, and nowhere else in the frontend', () => {
		const offenders: string[] = [];
		for (const file of walk(join(process.cwd(), 'src'))) {
			const name = relative(process.cwd(), file).split('\\').join('/');
			if (DEFINING.has(name) || MERGER.some((prefix) => name.startsWith(prefix))) continue;
			const text = readFileSync(file, 'utf8')
				.replace(/\/\*[\s\S]*?\*\//g, '')
				.replace(/(^|[^:'"`\\])\/\/[^\n]*/g, '$1');
			for (const pattern of MERGES) {
				if (pattern.test(text)) offenders.push(`${name}: ${pattern}`);
			}
		}
		expect(offenders).toEqual([]);
	});

	it('still finds the merges in Merger, so this check is not looking at nothing', () => {
		const store = readFileSync(join(process.cwd(), 'src/lib/merger/store.svelte.ts'), 'utf8');
		expect(store).toMatch(/api\.mergePullRequest\s*\(/);
		expect(store).toMatch(/api\.mergerLand\s*\(/);
	});
});
