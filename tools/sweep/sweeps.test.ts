// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The sweeps read as data and written back (the sweep portal). Each of the
 * four shapes the records come in is read, a result written into each lands
 * in its one place, and every real sweep document reads cleanly.
 */

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { INDEX, current } from './index';
import { cells, parse, resultText, stateOf, write } from './sweeps';

const ROWS = `<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-082 — Sweep

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG082-01 | Settings › You | Type a \\| b; save | Both stay | Mounted test; desktop check not run |
| SWEEP-BUG082-02 | Same | Save | Kept | Passed, 2026-10-12 |
`;

const SECTIONS = `# BUG-006 — Manual sweep

## BUG-006-T1 — The reported defect is gone

**Priority:** high
**Preconditions:** a repository whose branch name is long
enough to overflow.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open All repositories | The card shows. |
| 2 | Read the branch | It ends in an ellipsis. |

**Result:**

---

## BUG-006-T2 — No result line yet

| # | Step | Expected |
| --- | --- | --- |
| 1 | Look | Fine. |
`;

const BULLETS = `# FEAT-004 — Manual sweep

### SWEEP-1F-01 — The list matches git

- **Priority:** P1
- **Steps:** Compare the table against \`git branch -a\`.
- **Expected:** Every branch appears,
  once.
- **Result:** pass, but slow
  on a large repository

### SWEEP-1F-02 — Ahead and behind

- **Priority:** P1
- **Steps:** Compare the drift.
- **Expected:** They agree.
`;

const CHECKS = `# TASK-032 — Manual sweep

## BASE-01 — Release baseline

**Priority:** High. **Preconditions:** Published 0.5.0.
1. Compare the versions.
2. Confirm the changelog.
**Expected:** Matching versions.
**Pass/fail:** ______
`;

const PROSE = `<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-059 — Manual Sweep

1. Open Requests.
2. Click a row.
`;

describe('stateOf', () => {
	it('reads what a result says from its first words', () => {
		expect(stateOf('')).toBe('not-run');
		expect(stateOf('Passed, 2026-10-12, on #63')).toBe('passed');
		expect(stateOf('pass')).toBe('passed');
		expect(stateOf('Failed — the menu is under the dialog')).toBe('failed');
		expect(stateOf('Obsolete — Pull requests no longer reviews')).toBe('obsolete');
		expect(stateOf('Blocked, 2026-10-19 — needs a macOS machine')).toBe('blocked');
		expect(stateOf('Partly passed, 2026-10-19')).toBe('partial');
	});

	it('takes what only a test or a headless browser saw as still to be swept', () => {
		expect(stateOf('Unit tests; desktop check not run')).toBe('not-run');
		expect(stateOf('Mounted test; the author is checking the desktop app')).toBe('not-run');
		expect(stateOf('Covered by mounted tests.')).toBe('not-run');
		expect(stateOf('Seen in the default dark theme only.')).toBe('noted');
	});
});

describe('cells', () => {
	it('keeps an escaped pipe inside its cell', () => {
		expect(cells('| a | b \\| c | d |')).toEqual(['a', 'b \\| c', 'd']);
	});
});

describe('parse', () => {
	it('reads a ticket table, one ticket a row', () => {
		const sweep = parse('BUG-082-sweep.md', ROWS);
		expect(sweep.item).toBe('BUG-082');
		expect(sweep.tickets.map((ticket) => [ticket.id, ticket.state])).toEqual([
			['SWEEP-BUG082-01', 'not-run'],
			['SWEEP-BUG082-02', 'passed']
		]);
		expect(sweep.tickets[0].steps).toBe('Type a \\| b; save');
	});

	it('reads sections with a step table, their result line or the place for one', () => {
		const [first, second] = parse('BUG-006-sweep.md', SECTIONS).tickets;
		expect(first.id).toBe('BUG-006-T1');
		expect(first.title).toBe('The reported defect is gone');
		expect(first.priority).toBe('high');
		expect(first.preconditions).toBe('a repository whose branch name is long\nenough to overflow.');
		expect(first.steps).toBe('1. Open All repositories\n2. Read the branch');
		expect(first.state).toBe('not-run');
		expect(second.line).toBe(-1);
		expect(second.after).toBeGreaterThan(first.line);
	});

	it('reads bulleted sections, a result over two lines included', () => {
		const [first, second] = parse('FEAT-004-sweep.md', BULLETS).tickets;
		expect(first.id).toBe('SWEEP-1F-01');
		expect(first.expected).toBe('Every branch appears,\nonce.');
		expect(first.result).toBe('pass, but slow\non a large repository');
		expect(first.state).toBe('passed');
		expect(first.span).toBe(2);
		expect(second.line).toBe(-1);
		expect(second.bulleted).toBe(true);
	});

	it('reads a check of its own name, with a numbered list and a Pass/fail line', () => {
		const [only] = parse('TASK-032-sweep.md', CHECKS).tickets;
		expect(only.id).toBe('BASE-01');
		expect(only.steps).toBe('1. Compare the versions.\n2. Confirm the changelog.');
		expect(only.expected).toBe('Matching versions.');
		expect(only.state).toBe('not-run');
		const text = write(CHECKS, only, 'Passed, 2026-10-19.');
		expect(text).toContain('**Pass/fail:** Passed, 2026-10-19.');
		expect(parse('TASK-032-sweep.md', text).tickets[0].state).toBe('passed');
	});

	it('reads prose as one ticket', () => {
		const [only] = parse('FEAT-059-sweep.md', PROSE).tickets;
		expect(only.id).toBe('FEAT-059-SWEEP');
		expect(only.shape).toBe('document');
		expect(only.steps).toContain('2. Click a row.');
	});
});

describe('write', () => {
	const day = '2026-10-19';

	it('changes the result cell of a row and nothing else', () => {
		const ticket = parse('BUG-082-sweep.md', ROWS).tickets[0];
		const out = write(ROWS, ticket, resultText('failed', day, 'the a | b field | lost its text'));
		const again = parse('BUG-082-sweep.md', out).tickets[0];
		expect(again.state).toBe('failed');
		expect(again.result).toBe('Failed, 2026-10-19 — the a \\| b field \\| lost its text.');
		expect(again.steps).toBe(ticket.steps);
		expect(out.split('\n').length).toBe(ROWS.split('\n').length);
	});

	it('fills the result line of a section, and adds one where there is none', () => {
		let text = SECTIONS;
		let [first] = parse('BUG-006-sweep.md', text).tickets;
		text = write(text, first, resultText('passed', day, 'the ellipsis shows'));
		const second = parse('BUG-006-sweep.md', text).tickets[1];
		text = write(text, second, resultText('obsolete', day, 'the card was redesigned in FEAT-085'));
		[first] = parse('BUG-006-sweep.md', text).tickets;
		const again = parse('BUG-006-sweep.md', text).tickets[1];
		expect(first.result).toBe('Passed, 2026-10-19 — the ellipsis shows.');
		expect(again.state).toBe('obsolete');
		expect(text).toContain('| 1 | Look | Fine. |\n\n**Result:** Obsolete');
	});

	it('replaces a result over two lines with one, and adds a bullet where there is none', () => {
		let text = BULLETS;
		const [first] = parse('FEAT-004-sweep.md', text).tickets;
		text = write(text, first, resultText('passed', day, ''));
		const second = parse('FEAT-004-sweep.md', text).tickets[1];
		text = write(text, second, resultText('blocked', day, 'no remote here'));
		expect(text).toContain('- **Result:** Passed, 2026-10-19.\n\n### SWEEP-1F-02');
		expect(text).toContain('- **Expected:** They agree.\n- **Result:** Blocked, 2026-10-19 — no remote here.');
	});

	it('gives prose a result line at the end, then fills it', () => {
		let text = write(PROSE, parse('FEAT-059-sweep.md', PROSE).tickets[0], 'Passed, 2026-10-19.');
		expect(text.trimEnd().endsWith('**Result:** Passed, 2026-10-19.')).toBe(true);
		text = write(text, parse('FEAT-059-sweep.md', text).tickets[0], 'Failed, 2026-10-20.');
		expect(text.match(/\*\*Result:\*\*/g)).toHaveLength(1);
	});

	it('keeps Windows line endings', () => {
		const crlf = ROWS.replace(/\n/g, '\r\n');
		const out = write(crlf, parse('BUG-082-sweep.md', crlf).tickets[1], 'Failed, 2026-10-19.');
		expect(out.includes('\r\n')).toBe(true);
		expect(out.replace(/\r\n/g, '').includes('\n')).toBe(false);
	});

	it('refuses a row whose cells do not match its header', () => {
		const broken = ROWS.replace('| SWEEP-BUG082-02 | Same |', '| SWEEP-BUG082-02 | Same | extra |');
		const ticket = parse('BUG-082-sweep.md', broken).tickets[1];
		expect(() => write(broken, ticket, 'Passed.')).toThrow(/cells/);
	});
});

describe('the sweeps in agile/testing', () => {
	const dir = 'agile/testing';
	const sweeps = readdirSync(dir)
		.filter((name) => name.endsWith('-sweep.md'))
		.map((name) => parse(name, readFileSync(join(dir, name), 'utf8')));

	it('all read, each with at least one ticket', () => {
		expect(sweeps.length).toBeGreaterThan(250);
		for (const sweep of sweeps) expect(sweep.tickets.length, sweep.file).toBeGreaterThan(0);
	});

	it('name each ticket once within a document', () => {
		for (const sweep of sweeps) {
			const ids = sweep.tickets.map((ticket) => ticket.id);
			expect(ids.length, sweep.file).toBe(new Set(ids).size);
		}
	});

	it('are listed in agile/testing/sweeps.md as they are now (`bun run sweep:index` writes it)', () => {
		expect(readFileSync(INDEX, 'utf8').replace(/\r\n/g, '\n')).toBe(current());
	});

	it('give every row as many cells as its header, so its result can be written', () => {
		for (const sweep of sweeps) {
			for (const ticket of sweep.tickets.filter((one) => one.width)) {
				expect(ticket.width!.row, `${sweep.file} ${ticket.id}`).toBe(ticket.width!.header);
			}
		}
	});
});
