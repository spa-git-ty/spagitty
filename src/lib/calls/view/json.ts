// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A JSON response as lines to draw (FEAT-160, proposal §5.4): formatted,
 * coloured, and with each value carrying the JSONPath that finds it, so a
 * click can keep it (FEAT-161).
 *
 * Read from the text rather than through `JSON.parse`: a parsed object puts
 * keys that look like integers first and loses digits from a large number,
 * and the screen must show what the server sent. Formatting is display only;
 * *Copy* and *Save as…* give the bytes that arrived.
 */

/** A run of a line, with what it is and, for a value, where it is. */
export interface Piece {
	text: string;
	/** A `tok-*` class from `app.css`, without the prefix. */
	cls?: 'string' | 'number' | 'keyword' | 'attr' | 'punctuation' | 'tag' | 'comment';
	/** Set on a value that can be kept: its path. */
	path?: string;
	/** Set with `path`: the value as it would be kept. */
	value?: string;
}

export type Line = Piece[];

type Node =
	| { kind: 'object'; entries: { key: string; raw: string; value: Node }[] }
	| { kind: 'array'; items: Node[] }
	| { kind: 'string'; raw: string; value: string }
	| { kind: 'number' | 'literal'; raw: string };

class Reader {
	at = 0;
	constructor(readonly text: string) {}

	space(): void {
		while (this.at < this.text.length && /\s/.test(this.text[this.at])) this.at++;
	}

	fail(): never {
		throw new Error(`not JSON at ${this.at}`);
	}

	value(depth: number): Node {
		if (depth > 512) this.fail();
		this.space();
		const c = this.text[this.at];
		if (c === '{') return this.object(depth);
		if (c === '[') return this.array(depth);
		if (c === '"') {
			const raw = this.string();
			return { kind: 'string', raw, value: JSON.parse(raw) as string };
		}
		const literal = /^(?:true|false|null)/.exec(this.text.slice(this.at, this.at + 5));
		if (literal) {
			this.at += literal[0].length;
			return { kind: 'literal', raw: literal[0] };
		}
		const number = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(this.text.slice(this.at, this.at + 400));
		if (number) {
			this.at += number[0].length;
			return { kind: 'number', raw: number[0] };
		}
		this.fail();
	}

	string(): string {
		const start = this.at;
		this.at++;
		while (this.at < this.text.length) {
			const c = this.text[this.at];
			if (c === '\\') this.at += 2;
			else if (c === '"') {
				this.at++;
				return this.text.slice(start, this.at);
			} else this.at++;
		}
		this.fail();
	}

	object(depth: number): Node {
		this.at++;
		const entries: { key: string; raw: string; value: Node }[] = [];
		this.space();
		if (this.text[this.at] === '}') {
			this.at++;
			return { kind: 'object', entries };
		}
		for (;;) {
			this.space();
			if (this.text[this.at] !== '"') this.fail();
			const raw = this.string();
			this.space();
			if (this.text[this.at] !== ':') this.fail();
			this.at++;
			entries.push({ key: JSON.parse(raw) as string, raw, value: this.value(depth + 1) });
			this.space();
			const c = this.text[this.at++];
			if (c === '}') return { kind: 'object', entries };
			if (c !== ',') this.fail();
		}
	}

	array(depth: number): Node {
		this.at++;
		const items: Node[] = [];
		this.space();
		if (this.text[this.at] === ']') {
			this.at++;
			return { kind: 'array', items };
		}
		for (;;) {
			items.push(this.value(depth + 1));
			this.space();
			const c = this.text[this.at++];
			if (c === ']') return { kind: 'array', items };
			if (c !== ',') this.fail();
		}
	}
}

/** `$.a`, `$['a-b']`, `$[0]`: the step to `key` from `path`. */
export function step(path: string, key: string | number): string {
	if (typeof key === 'number') return `${path}[${key}]`;
	if (/^[A-Za-z_$][\w$]*$/.test(key)) return `${path}.${key}`;
	return `${path}['${key.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}']`;
}

/** The text a value is kept as: a string without its quotes, anything else as written. */
function kept(node: Node): string {
	if (node.kind === 'string') return node.value;
	if (node.kind === 'number' || node.kind === 'literal') return node.raw;
	return '';
}

/**
 * `text` formatted with two spaces, or `null` when it is not JSON. Objects
 * and arrays are drawn open, each member on its own line; an empty one is
 * `{}` or `[]`.
 */
export function formatJson(text: string): Line[] | null {
	let root: Node;
	try {
		const reader = new Reader(text);
		root = reader.value(0);
		reader.space();
		if (reader.at !== text.length) return null;
	} catch {
		return null;
	}

	const lines: Line[] = [];
	const indent = (depth: number): Piece => ({ text: '  '.repeat(depth) });

	const leaf = (node: Node, path: string): Piece => {
		switch (node.kind) {
			case 'string':
				return { text: node.raw, cls: 'string', path, value: kept(node) };
			case 'number':
				return { text: node.raw, cls: 'number', path, value: kept(node) };
			default:
				return { text: (node as { raw: string }).raw, cls: 'keyword', path, value: kept(node) };
		}
	};

	/** Draw `node` starting on `head`, the pieces already on its first line. */
	const draw = (node: Node, path: string, depth: number, head: Piece[], comma: boolean): void => {
		const tail: Piece[] = comma ? [{ text: ',', cls: 'punctuation' }] : [];
		if (node.kind === 'object' || node.kind === 'array') {
			const [open, close] = node.kind === 'object' ? ['{', '}'] : ['[', ']'];
			const size = node.kind === 'object' ? node.entries.length : node.items.length;
			if (size === 0) {
				lines.push([...head, { text: open + close, cls: 'punctuation' }, ...tail]);
				return;
			}
			lines.push([...head, { text: open, cls: 'punctuation' }]);
			if (node.kind === 'object') {
				node.entries.forEach((entry, index) => {
					draw(
						entry.value,
						step(path, entry.key),
						depth + 1,
						[indent(depth + 1), { text: entry.raw, cls: 'attr' }, { text: ': ', cls: 'punctuation' }],
						index < size - 1
					);
				});
			} else {
				node.items.forEach((item, index) => {
					draw(item, step(path, index), depth + 1, [indent(depth + 1)], index < size - 1);
				});
			}
			lines.push([indent(depth), { text: close, cls: 'punctuation' }, ...tail]);
			return;
		}
		lines.push([...head, leaf(node, path), ...tail]);
	};

	draw(root, '$', 0, [], false);
	return lines.map((line) => line.filter((piece) => piece.text !== ''));
}
