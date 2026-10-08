// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * The working record checks itself (TASK-012).
 *
 * `agile/` is the project's working record, and it had drifted from the tree
 * in six separate ways before anyone audited it: an index covering 40% of the
 * items, five statuses that contradicted the code, an item with no documents at
 * all, and four identifiers cited as dependencies that resolved to nothing. None
 * of it was visible, because nothing checked it.
 *
 * These assertions are that check. They read the record as data — the index
 * tables, each item's `**Status:**` line, and every identifier cited anywhere in
 * `agile/` or `docs/` — and fail when the pieces disagree. They deliberately
 * assert nothing about *prose*: a document can say anything it likes, as long as
 * the joins hold and the debts are declared.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const ITEMS = 'agile/items';
const README = 'agile/README.md';

/** `FEAT-036`, `TASK-004`, `BUG-009b` — the trailing letter is BUG-009b's. */
const ID = /\b(?:FEAT|TASK|BUG)-\d{3}[a-z]?\b/g;

/** The vocabulary of `agile/README.md`'s Status section. */
const STATUSES = ['Done', 'Fixed', 'Partial', 'Open', 'Backlog'] as const;
type Status = (typeof STATUSES)[number];

/** Built work: everything that owes four documents. */
const BUILT: Status[] = ['Done', 'Fixed', 'Partial'];

const readme = readFileSync(README, 'utf8');

interface Item {
	id: string;
	file: string;
	status: Status;
	/** `plan`, `automated`, `sweep` — whichever are absent. */
	missing: string[];
}

const items: Item[] = readdirSync(ITEMS)
	.filter((name) => name.endsWith('.md'))
	.map((name) => {
		const id = name.match(/^((?:FEAT|TASK|BUG)-\d{3}[a-z]?)-/)?.[1];
		if (!id) throw new Error(`${name} does not start with an identifier`);

		const status = readFileSync(join(ITEMS, name), 'utf8').match(/^\*\*Status:\*\* (\w+)/m)?.[1];
		if (!status) throw new Error(`${name} has no **Status:** line`);

		const missing = [
			['plan', `agile/plans/${id}-plan.md`],
			['automated', `agile/testing/${id}-automated.md`],
			['sweep', `agile/testing/${id}-sweep.md`]
		]
			.filter(([, path]) => !existsSync(path))
			.map(([kind]) => kind);

		return { id, file: name, status: status as Status, missing };
	})
	.sort((a, b) => a.id.localeCompare(b.id));

/** One row of an index table: `| [ID](items/file.md) | Title | Screen | Status |`. */
const rows = [...readme.matchAll(/^\| \[([^\]]+)\]\(items\/([^)]+)\) \|[^|]*\|[^|]*\| (\w+) \|$/gm)].map(
	(match) => ({ id: match[1], file: match[2], status: match[3] })
);

/** The Skipped identifiers table — identifiers that resolve to "never real". */
const skipped = [...readme.matchAll(/^\| ((?:FEAT|TASK|BUG)-\d{3}[a-z]?) \| [^|]+ \|$/gm)].map((m) => m[1]);

/** The Documents outstanding table: which documents a built item is allowed to lack. */
const outstanding = new Map(
	[...readme.matchAll(/^\| ((?:FEAT|TASK|BUG)-\d{3}[a-z]?) \| ([a-z, ]+) \| [^|]+ \|$/gm)].map((m) => [
		m[1],
		m[2].split(',').map((word) => word.trim())
	])
);

describe('the index covers the tree, and only the tree', () => {
	it('gives every item exactly one row', () => {
		const rowed = rows.map((row) => row.id).sort();
		expect(rowed).toEqual(items.map((item) => item.id));
	});

	it('points every row at the document it names', () => {
		for (const row of rows) {
			expect(existsSync(join(ITEMS, row.file)), `${row.id} → ${row.file}`).toBe(true);
		}
		expect(rows.map((row) => `${row.id} ${row.file}`).sort()).toEqual(
			items.map((item) => `${item.id} ${item.file}`)
		);
	});

	it('lists no identifier twice, in any table', () => {
		const all = [...rows.map((row) => row.id), ...skipped];
		expect(all.length).toBe(new Set(all).size);
	});
});

describe('statuses', () => {
	it('use the vocabulary the README defines', () => {
		for (const item of items) {
			expect(STATUSES, `${item.id}`).toContain(item.status);
		}
	});

	/** The drift that sent people to write code that was already there. */
	it('say the same thing in the item and in the index', () => {
		for (const row of rows) {
			const item = items.find((candidate) => candidate.id === row.id);
			expect(row.status, `${row.id} index vs item`).toBe(item?.status);
		}
	});
});

describe('four documents, or a recorded reason', () => {
	it('gives built work all four, unless the debt is declared', () => {
		for (const item of items.filter((candidate) => BUILT.includes(candidate.status))) {
			if (item.missing.length === 0) continue;
			expect(
				outstanding.get(item.id),
				`${item.id} is ${item.status} and lacks ${item.missing.join(', ')}; declare it under "Documents outstanding"`
			).toEqual(item.missing);
		}
	});

	it('keeps no stale row in the outstanding table', () => {
		for (const [id, declared] of outstanding) {
			const item = items.find((candidate) => candidate.id === id);
			expect(item, `${id} is declared outstanding but has no item document`).toBeDefined();
			expect(declared, `${id}: the outstanding table is out of date`).toEqual(item?.missing);
		}
	});

	/** Backlog items carry an item document only, by design. */
	it('asks nothing of work that has not started', () => {
		for (const item of items.filter((candidate) => candidate.status === 'Backlog')) {
			expect(outstanding.has(item.id), `${item.id} is Backlog; it owes nothing yet`).toBe(false);
		}
	});
});

describe('every identifier cited anywhere resolves', () => {
	const known = new Set([...items.map((item) => item.id), ...skipped]);

	const sources = [
		...readdirSync('agile').flatMap((entry) => {
			const path = join('agile', entry);
			if (entry.endsWith('.md')) return [path];
			return readdirSync(path).map((name) => join(path, name));
		}),
		...readdirSync('docs')
			.filter((name) => name.endsWith('.md'))
			.map((name) => join('docs', name))
	].filter((path) => path.endsWith('.md'));

	it.each(sources)('%s', (path) => {
		const cited = new Set(readFileSync(path, 'utf8').match(ID) ?? []);
		const dangling = [...cited].filter((id) => !known.has(id));
		expect(
			dangling,
			`cited with no item document and no "Skipped identifiers" row: ${dangling.join(', ')}`
		).toEqual([]);
	});
});
