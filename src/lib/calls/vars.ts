// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * `{{name}}` as the screen draws it (FEAT-160, proposal §5.2): a pill
 * wherever it appears, in `--danger` when nothing defines it, and offered
 * when `{{` is typed.
 *
 * Resolution itself is Rust's (`calls::vars`), at *Send*; this only finds
 * names, so the same rules are kept here in their simplest form: a name is
 * what is between `{{` and `}}`, trimmed, with no brace or line break in it.
 */

/** The read-only built-ins, as `calls::vars::BUILTINS` lists them. */
export const BUILTINS = ['$timestamp', '$isoNow', '$uuid', '$randomInt'] as const;

/** What each built-in gives, for the hover and the suggestions. */
export const BUILTIN_NOTES: Record<(typeof BUILTINS)[number], string> = {
	$timestamp: 'seconds since 1970, when sent',
	$isoNow: 'the time, when sent, as 2026-10-25T14:03:00.000Z',
	$uuid: 'a new random UUID',
	$randomInt: 'a random number from 0 to 999'
};

/** A run of text, or one variable in it. */
export interface Segment {
	text: string;
	/** Set on a variable: the name inside the braces. */
	name?: string;
	start: number;
}

/** `text` cut into plain runs and variables, in order. */
export function segments(text: string): Segment[] {
	const out: Segment[] = [];
	let at = 0;
	let plain = 0;
	while (at < text.length) {
		const open = text.indexOf('{{', at);
		if (open === -1) break;
		const close = text.indexOf('}}', open + 2);
		if (close === -1) break;
		const name = text.slice(open + 2, close).trim();
		if (name === '' || name.includes('{') || name.includes('\n')) {
			at = open + 2;
			continue;
		}
		if (open > plain) out.push({ text: text.slice(plain, open), start: plain });
		out.push({ text: text.slice(open, close + 2), name, start: open });
		at = close + 2;
		plain = at;
	}
	if (plain < text.length) out.push({ text: text.slice(plain), start: plain });
	return out;
}

/** Every name in `text`, each once. */
export function namesIn(text: string): string[] {
	return [...new Set(segments(text).flatMap((segment) => (segment.name ? [segment.name] : [])))];
}

export function isBuiltin(name: string): boolean {
	return (BUILTINS as readonly string[]).includes(name);
}

/** A `{{` being typed before the caret: where it starts and what is typed after it. */
export interface Typing {
	/** The index of the first `{`. */
	from: number;
	query: string;
}

export function typingAt(text: string, caret: number): Typing | null {
	const before = text.slice(0, caret);
	const match = /\{\{\s*([\w$.-]*)$/.exec(before);
	if (!match) return null;
	return { from: match.index, query: match[1] };
}

/** The names to offer for `query`: those that start with it, then those that contain it. */
export function suggest(query: string, names: string[]): string[] {
	const all = [...new Set([...names, ...BUILTINS])];
	const lower = query.toLowerCase();
	const starts = all.filter((name) => name.toLowerCase().startsWith(lower));
	const contains = all.filter((name) => !starts.includes(name) && name.toLowerCase().includes(lower));
	return [...starts.sort(), ...contains.sort()].slice(0, 12);
}

/**
 * `text` with the `{{` being typed at `typing` completed as `{{name}}`, and
 * where the caret goes: after the closing braces. A `}}` already following
 * the caret is not doubled.
 */
export function complete(text: string, caret: number, typing: Typing, name: string): { text: string; caret: number } {
	const after = text.slice(caret);
	const rest = after.replace(/^[\w$.-]*\s*\}\}/, '');
	const inserted = `{{${name}}}`;
	return { text: text.slice(0, typing.from) + inserted + rest, caret: typing.from + inserted.length };
}
