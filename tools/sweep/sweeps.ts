// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The manual sweeps, read as data and written back (the sweep portal).
 *
 * `agile/testing/<ID>-sweep.md` holds what a person checks by hand in the
 * running application. The records have three shapes, from three eras:
 *
 * - **Ticket tables**, most of them: one row per ticket, `| Ticket | … |`, with
 *   a Result or Pass/Fail column at the end.
 * - **Sections**, the oldest: `## BUG-006-T1 — Title`, a `| # | Step |
 *   Expected |` table, and a `**Result:**` line the tester fills in.
 * - **Prose**: a list of steps and no place for a result. The whole document
 *   is one ticket, and its result is a `**Result:**` line at the end.
 *
 * Reading keeps where each result lives, so writing changes that one cell or
 * line and nothing else in the file. The markdown stays the record; the
 * portal is a way to fill it in.
 */

export type State = 'not-run' | 'passed' | 'failed' | 'blocked' | 'partial' | 'obsolete' | 'noted';

export const STATES: readonly State[] = ['not-run', 'passed', 'failed', 'blocked', 'partial', 'obsolete', 'noted'];

/** The word a result starts with, for a state the portal sets. */
export const WORD: Record<Exclude<State, 'noted' | 'not-run'>, string> = {
	passed: 'Passed',
	failed: 'Failed',
	blocked: 'Blocked',
	partial: 'Partly passed',
	obsolete: 'Obsolete'
};

export type Shape = 'row' | 'section' | 'document';

export interface Ticket {
	id: string;
	title: string;
	priority: string;
	preconditions: string;
	steps: string;
	expected: string;
	result: string;
	state: State;
	shape: Shape;
	/** The line the result is on: a row, a `**Result:**` line, or -1 for none yet. */
	line: number;
	/** How many lines a section's result runs over. */
	span: number;
	/** For a section with no result line yet: the line one goes after. */
	after: number;
	/** A section whose fields are a bulleted list, `- **Result:**`. */
	bulleted: boolean;
	/** For a row: which cell is the result. */
	column: number;
	/** For a row: how many cells its header has, and how many it has. */
	width?: { header: number; row: number };
}

export interface Sweep {
	/** `BUG-006`, `FEAT-097`. */
	item: string;
	/** The file's name, `BUG-006-sweep.md`. */
	file: string;
	/** The document's own heading. */
	heading: string;
	tickets: Ticket[];
}

/** What a result says, read from its first words. */
export function stateOf(result: string): State {
	const text = result.replace(/\*\*/g, '').trim();
	if (text === '' || text === '—' || text === '-') return 'not-run';
	if (/^obsolete\b/i.test(text)) return 'obsolete';
	if (/^(failed|fail\b|fails\b|❌)/i.test(text)) return 'failed';
	if (/^blocked\b/i.test(text)) return 'blocked';
	if (/^(partly passed|partially|passed in part|in part\b)/i.test(text)) return 'partial';
	if (/^(passed|pass\b|passes\b|ok\b|done\b|✅)/i.test(text)) return 'passed';
	// What only a test, or a headless browser, has seen is still to be swept.
	if (
		/\b(not run|not yet run|owed|unverified|unverifiable|not tried|not looked at|desktop check not run|is checking|covered by|mounted tests?\b|headless)/i.test(
			text
		)
	)
		return 'not-run';
	return 'noted';
}

/** A table row's cells, `\|` kept as a pipe inside its cell. */
export function cells(line: string): string[] {
	const inner = line.trim().replace(/^\|/, '').replace(/\|$/, '');
	const out: string[] = [];
	let cell = '';
	for (let index = 0; index < inner.length; index++) {
		const char = inner[index];
		if (char === '\\' && inner[index + 1] === '|') {
			cell += '\\|';
			index++;
		} else if (char === '|') {
			out.push(cell.trim());
			cell = '';
		} else {
			cell += char;
		}
	}
	out.push(cell.trim());
	return out;
}

const ROW = /^\s*\|/;
const SEPARATOR = /^\s*\|[\s:|-]+\|\s*$/;
const RESULT_LINE = /^(\s*(?:-\s+)?)\*\*Result:\*\*(.*)$/;
/** A bold label and what follows it: `**Steps:** …` or `- **Steps:** …`. */
const FIELD = /^(\s*)(-\s+)?\*\*([A-Za-z][A-Za-z /]*?):\*\*\s*(.*)$/;
const HEADING = /^#{1,4}\s/;

function columnOf(header: string[], ...names: RegExp[]): number {
	return header.findIndex((name) => names.some((pattern) => pattern.test(name)));
}

interface Field {
	text: string;
	line: number;
	span: number;
	bulleted: boolean;
}

/** The labelled fields between `from` and `to`, each with the lines it runs over. */
function fieldsOf(lines: string[], from: number, to: number): Map<string, Field> {
	const fields = new Map<string, Field>();
	for (let at = from; at < to; at++) {
		const found = FIELD.exec(lines[at]);
		if (!found) continue;
		const label = found[3].toLowerCase();
		const name = label.startsWith('precondition')
			? 'preconditions'
			: label.startsWith('step')
				? 'steps'
				: label.startsWith('expected')
					? 'expected'
					: label;
		let next = at + 1;
		while (next < to) {
			const line = lines[next];
			if (FIELD.test(line) || HEADING.test(line) || line.trim() === '---' || ROW.test(line)) break;
			// A blank line ends it, unless an indented line carries it on.
			if (line.trim() === '' && !/^\s+\S/.test(lines[next + 1] ?? '')) break;
			next++;
		}
		const text = [found[4], ...lines.slice(at + 1, next).map((line) => line.trim())]
			.join('\n')
			.trim();
		if (!fields.has(name)) fields.set(name, { text, line: at, span: next - at, bulleted: !!found[2] });
		at = next - 1;
	}
	return fields;
}

/** Read one sweep document. `file` is its name, which says whose it is. */
export function parse(file: string, text: string): Sweep {
	const item = /^((?:FEAT|TASK|BUG)-\d{3}[a-z]?)-sweep\.md$/.exec(file)?.[1] ?? file;
	const lines = text.split(/\r?\n/);
	const heading = lines.find((line) => line.startsWith('# '))?.slice(2).trim() ?? item;
	const tickets: Ticket[] = [];

	// Ticket tables.
	for (let index = 0; index < lines.length; index++) {
		const header = ROW.test(lines[index]) ? cells(lines[index]) : null;
		if (!header || !/^ticket$/i.test(header[0] ?? '') || !SEPARATOR.test(lines[index + 1] ?? '')) continue;
		const at = {
			preconditions: columnOf(header, /^preconditions?$/i),
			steps: columnOf(header, /^steps?$/i),
			expected: columnOf(header, /^expected/i),
			priority: columnOf(header, /^priority$/i),
			result: columnOf(header, /^result$/i, /^pass\s*\/\s*fail$/i)
		};
		let row = index + 2;
		for (; row < lines.length && ROW.test(lines[row]); row++) {
			const values = cells(lines[row]);
			const cell = (column: number) => (column >= 0 ? (values[column] ?? '') : '');
			const result = cell(at.result);
			tickets.push({
				id: values[0],
				title: '',
				priority: cell(at.priority),
				preconditions: cell(at.preconditions),
				steps: cell(at.steps),
				expected: cell(at.expected),
				result,
				state: stateOf(result),
				shape: 'row',
				line: row,
				span: 1,
				after: -1,
				bulleted: false,
				column: at.result,
				width: { header: header.length, row: values.length }
			});
		}
		index = row - 1;
	}

	// Sections: `## BUG-006-T1 — Title` with a step table and **Result:**
	// lines, or `### SWEEP-1F-01 — Title` with bulleted fields.
	const SECTION = /^#{2,4}\s+((?:FEAT|TASK|BUG)-\d{3}[a-z]?-T\d+|SWEEP-[A-Za-z0-9-]+)\s*[—–-]\s*(.*)$/;
	for (let index = 0; index < lines.length; index++) {
		const found = SECTION.exec(lines[index]);
		if (!found) continue;
		let end = index + 1;
		while (end < lines.length && !/^#{1,4}\s/.test(lines[end])) end++;
		const fields = fieldsOf(lines, index + 1, end);
		const steps: string[] = [];
		const expected: string[] = [];
		for (const line of lines.slice(index + 1, end)) {
			if (!ROW.test(line) || SEPARATOR.test(line)) continue;
			const values = cells(line);
			if (!/^\d+$/.test(values[0] ?? '')) continue;
			steps.push(`${values[0]}. ${values[1] ?? ''}`);
			expected.push(`${values[0]}. ${values[2] ?? ''}`);
		}
		const result = fields.get('result');
		const said = result?.text ?? '';
		// With no result line, one goes after the section's last line.
		let last = end - 1;
		while (last > index && (lines[last].trim() === '' || lines[last].trim() === '---')) last--;
		tickets.push({
			id: found[1],
			title: found[2].trim(),
			priority: fields.get('priority')?.text ?? '',
			preconditions: fields.get('preconditions')?.text ?? '',
			steps: steps.length > 0 ? steps.join('\n') : (fields.get('steps')?.text ?? ''),
			expected: expected.length > 0 ? expected.join('\n') : (fields.get('expected')?.text ?? ''),
			result: said,
			state: stateOf(said),
			shape: 'section',
			line: result?.line ?? -1,
			span: result?.span ?? 0,
			after: last,
			bulleted: result ? result.bulleted : /^\s*-\s+\*\*/.test(lines[last] ?? ''),
			column: -1
		});
	}

	// Prose: the document is its own ticket.
	if (tickets.length === 0) {
		let resultAt = -1;
		lines.forEach((line, index) => {
			if (RESULT_LINE.test(line)) resultAt = index;
		});
		const result = resultAt >= 0 ? (RESULT_LINE.exec(lines[resultAt])?.[2] ?? '').trim() : '';
		const body = lines
			.filter((line, index) => index !== resultAt && !line.startsWith('<!--') && !line.startsWith('# '))
			.join('\n')
			.trim();
		tickets.push({
			id: `${item}-SWEEP`,
			title: heading,
			priority: '',
			preconditions: '',
			steps: body,
			expected: '',
			result,
			state: stateOf(result),
			shape: 'document',
			line: resultAt,
			span: resultAt >= 0 ? 1 : 0,
			after: -1,
			bulleted: false,
			column: -1
		});
	}

	return { item, file, heading, tickets };
}

/** A result as it is written: its word, the day, and what was seen. */
export function resultText(state: State, day: string, note: string, evidence: string[] = []): string {
	const said = note.replace(/\s+/g, ' ').trim();
	const shown = evidence.length > 0 ? ` Evidence: ${evidence.join(', ')}.` : '';
	if (state === 'noted') return `${said}${shown}`.trim();
	if (state === 'not-run' && said === '' && shown === '') return '';
	const word = state === 'not-run' ? 'Not run' : WORD[state];
	return `${word}, ${day}${said ? ` — ${said.replace(/\.$/, '')}` : ''}.${shown}`;
}

/** Make text safe for one table cell. */
function forCell(text: string): string {
	return text.replace(/\r?\n/g, ' ').replace(/(?<!\\)\|/g, '\\|');
}

/**
 * `text` with `ticket`'s result replaced by `result`. Everything else in the
 * file is left byte for byte, its line endings included.
 */
export function write(text: string, ticket: Ticket, result: string): string {
	const eol = text.includes('\r\n') ? '\r\n' : '\n';
	const lines = text.split(/\r?\n/);
	if (ticket.shape === 'row') {
		const values = cells(lines[ticket.line]);
		if (values[0] !== ticket.id) throw new Error(`${ticket.id} is no longer on line ${ticket.line + 1}`);
		if (ticket.width && ticket.width.row !== ticket.width.header)
			throw new Error(`${ticket.id}'s row has ${ticket.width.row} cells and its header ${ticket.width.header}`);
		const column = ticket.column >= 0 ? ticket.column : values.length - 1;
		while (values.length <= column) values.push('');
		values[column] = forCell(result);
		lines[ticket.line] = `| ${values.join(' | ')} |`;
	} else if (ticket.line >= 0) {
		const found = RESULT_LINE.exec(lines[ticket.line]);
		if (!found) throw new Error(`${ticket.id}'s result line has moved`);
		// One line now, however many the result ran over before.
		lines.splice(ticket.line, Math.max(1, ticket.span), `${found[1]}**Result:**${result ? ` ${result}` : ''}`);
	} else if (ticket.shape === 'document') {
		while (lines.length > 0 && lines[lines.length - 1].trim() === '') lines.pop();
		lines.push('', `**Result:**${result ? ` ${result}` : ''}`, '');
	} else if (ticket.after >= 0) {
		const added = `**Result:**${result ? ` ${result}` : ''}`;
		lines.splice(ticket.after + 1, 0, ...(ticket.bulleted ? [`- ${added}`] : ['', added]));
	} else {
		throw new Error(`${ticket.id} has no result line to write`);
	}
	return lines.join(eol);
}

/** How many tickets are in each state. */
export function tally(tickets: readonly Ticket[]): Record<State, number> {
	const out = Object.fromEntries(STATES.map((state) => [state, 0])) as Record<State, number>;
	for (const ticket of tickets) out[ticket.state] += 1;
	return out;
}
