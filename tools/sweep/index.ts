// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * `agile/testing/sweeps.md`: every manual sweep on one page, with how far
 * each has got. Generated from the sweep documents and the agile index; the
 * portal writes it again after every result, and `bun run sweep:index` does
 * it by hand. A test fails when it is out of date.
 *
 * Usage:
 *   bun tools/sweep/index.ts           write it
 *   bun tools/sweep/index.ts --check   exit 1 when it is not what it would be
 */

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parse, tally, type Sweep } from './sweeps';

export const TESTING = 'agile/testing';
export const INDEX = join(TESTING, 'sweeps.md');

export interface Listed {
	title: string;
	status: string;
}

/** The agile index's rows: each item's title and status word. */
export function listed(readme: string): Map<string, Listed> {
	const rows = readme.matchAll(/^\| \[([^\]]+)\]\(items\/[^)]+\) \| ([^|]*)\|[^|]*\| (\w+) \|$/gm);
	return new Map([...rows].map((row) => [row[1], { title: row[2].trim(), status: row[3] }]));
}

/** Every sweep document, read. */
export function readAll(dir = TESTING): Sweep[] {
	return readdirSync(dir)
		.filter((name) => /^(?:FEAT|TASK|BUG)-\d{3}[a-z]?-sweep\.md$/.test(name))
		.sort(byId)
		.map((name) => parse(name, readFileSync(join(dir, name), 'utf8')));
}

/** FEAT, TASK, BUG, each in number order. */
function byId(a: string, b: string): number {
	const kind = (name: string) => ['FEAT', 'TASK', 'BUG'].indexOf(name.slice(0, name.indexOf('-')));
	return kind(a) - kind(b) || a.localeCompare(b, 'en', { numeric: true });
}

/** The page. */
export function render(sweeps: readonly Sweep[], items: ReadonlyMap<string, Listed>): string {
	const all = sweeps.flatMap((sweep) => sweep.tickets);
	const total = tally(all);
	const lines = [
		'<!-- SPDX-License-Identifier: GPL-3.0-or-later -->',
		'',
		'# Manual sweeps',
		'',
		'Every manual sweep, one row each, with how far it has got. **Generated**: the',
		'sweep portal writes it after every result, and `bun run sweep:index` writes it',
		'by hand. Do not edit it; edit the sweep documents.',
		'',
		'`bun run sweep` opens the portal: each ticket with its steps, a result to set',
		'(passed, failed, blocked, partly passed, obsolete), a note, and screenshots or a',
		'screen recording to attach. Results are written into the sweep documents',
		'themselves. Attached files are kept under `agile/testing/evidence/`, which is',
		'not committed.',
		'',
		`**${all.length} tickets in ${sweeps.length} sweeps:** ${total.passed} passed, ${total.failed} failed, ` +
			`${total.blocked} blocked, ${total.partial} partly passed, ${total.obsolete} obsolete, ` +
			`${total['not-run']} not run, ${total.noted} with a note only.`,
		'',
		'| Item | Title | Status | Tickets | Passed | Failed | Blocked | Obsolete | Not run | Sweep |',
		'| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |'
	];
	for (const sweep of sweeps) {
		const count = tally(sweep.tickets);
		const known = items.get(sweep.item);
		const title = (known?.title ?? sweep.heading).replace(/\|/g, '\\|');
		lines.push(
			`| ${sweep.item} | ${title} | ${known?.status ?? '—'} | ${sweep.tickets.length} | ${count.passed} | ` +
				`${count.failed} | ${count.blocked} | ${count.obsolete} | ${count['not-run'] + count.noted + count.partial} | ` +
				`[${sweep.file}](${sweep.file}) |`
		);
	}
	lines.push('');
	return lines.join('\n');
}

/** The page as it should be now. */
export function current(): string {
	return render(readAll(), listed(readFileSync('agile/README.md', 'utf8')));
}

function main(argv: string[]): number {
	const page = current();
	if (argv.includes('--check')) {
		const kept = readFileSync(INDEX, 'utf8').replace(/\r\n/g, '\n');
		if (kept === page) return 0;
		console.error(`${INDEX} is out of date: run \`bun run sweep:index\`.`);
		return 1;
	}
	writeFileSync(INDEX, page);
	console.log(`${INDEX}: ${page.split('\n').length} lines`);
	return 0;
}

if (import.meta.main) {
	process.exit(main(process.argv.slice(2)));
}
