// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * An XML or SOAP response as lines to draw (FEAT-160, proposal §5.4 and
 * §7.2): formatted, coloured, and with each element's text and each
 * attribute carrying the XPath that finds it, so a click can keep it.
 *
 * The path uses the response's own prefixes. Inside a SOAP body it starts
 * at the body's first element with `//`
 * (`//inv:GetInvoiceResponse/inv:invoice/inv:total`), which is what a
 * person would write; elsewhere it is absolute. Where siblings share a name,
 * a position says which (`[2]`). The prefixes the document declares are
 * returned with the lines, to be stored on a keep rule.
 */

import type { Line, Piece } from './json';

type Node =
	| { kind: 'element'; name: string; attrs: Attr[]; children: Node[]; empty: boolean }
	| { kind: 'text'; raw: string }
	| { kind: 'cdata'; raw: string }
	| { kind: 'other'; raw: string };

interface Attr {
	name: string;
	raw: string;
	value: string;
}

export interface FormattedXml {
	lines: Line[];
	/** Every `xmlns:prefix` the document declares, prefix to namespace. */
	ns: Record<string, string>;
}

const ENTITIES: Record<string, string> = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'" };

/** `&amp;`, `&#38;` and `&#x26;` read as `&`. */
export function unescapeXml(text: string): string {
	return text.replace(/&(#x[0-9a-f]+|#\d+|\w+);/gi, (whole, entity: string) => {
		if (entity[0] === '#') {
			const code = entity[1].toLowerCase() === 'x' ? parseInt(entity.slice(2), 16) : parseInt(entity.slice(1), 10);
			return Number.isFinite(code) ? String.fromCodePoint(code) : whole;
		}
		return ENTITIES[entity] ?? whole;
	});
}

function parse(text: string): Node[] | null {
	const root: Node[] = [];
	const stack: { kind: 'element'; name: string; attrs: Attr[]; children: Node[]; empty: boolean }[] = [];
	const add = (node: Node) => (stack.length ? stack[stack.length - 1].children : root).push(node);
	let at = 0;
	while (at < text.length) {
		const open = text.indexOf('<', at);
		if (open === -1) {
			add({ kind: 'text', raw: text.slice(at) });
			break;
		}
		if (open > at) add({ kind: 'text', raw: text.slice(at, open) });
		const rest = text.slice(open, open + 9);
		const closeOf = (marker: string, skip: number) => {
			const end = text.indexOf(marker, open + skip);
			return end === -1 ? -1 : end + marker.length;
		};
		if (rest.startsWith('<!--')) {
			const end = closeOf('-->', 4);
			if (end === -1) return null;
			add({ kind: 'other', raw: text.slice(open, end) });
			at = end;
		} else if (rest.startsWith('<![CDATA[')) {
			const end = closeOf(']]>', 9);
			if (end === -1) return null;
			add({ kind: 'cdata', raw: text.slice(open, end) });
			at = end;
		} else if (rest.startsWith('<?') || rest.startsWith('<!')) {
			const end = closeOf('>', 2);
			if (end === -1) return null;
			add({ kind: 'other', raw: text.slice(open, end) });
			at = end;
		} else if (rest.startsWith('</')) {
			const end = text.indexOf('>', open);
			if (end === -1) return null;
			const name = text.slice(open + 2, end).trim();
			const top = stack.pop();
			if (!top || top.name !== name) return null;
			at = end + 1;
		} else {
			const tag = /^<([^\s/>]+)((?:\s+[^\s=/>]+\s*=\s*(?:"[^"]*"|'[^']*'))*)\s*(\/?)>/.exec(text.slice(open));
			if (!tag) return null;
			const attrs: Attr[] = [];
			for (const match of tag[2].matchAll(/([^\s=/>]+)\s*=\s*("[^"]*"|'[^']*')/g)) {
				attrs.push({ name: match[1], raw: match[2], value: unescapeXml(match[2].slice(1, -1)) });
			}
			const element = { kind: 'element' as const, name: tag[1], attrs, children: [] as Node[], empty: tag[3] === '/' };
			add(element);
			if (!element.empty) stack.push(element);
			at = open + tag[0].length;
		}
	}
	if (stack.length > 0) return null;
	return root;
}

const local = (name: string) => name.slice(name.indexOf(':') + 1);

/** `text` formatted with two spaces, or `null` when it is not well-formed. */
export function formatXml(text: string): FormattedXml | null {
	const nodes = parse(text);
	if (!nodes || !nodes.some((node) => node.kind === 'element')) return null;

	const ns: Record<string, string> = {};
	const lines: Line[] = [];
	const pad = (depth: number): Piece => ({ text: '  '.repeat(depth) });

	const openTag = (element: Extract<Node, { kind: 'element' }>, path: string, end: string): Piece[] => {
		const pieces: Piece[] = [{ text: `<${element.name}`, cls: 'tag' }];
		for (const attr of element.attrs) {
			if (attr.name.startsWith('xmlns:')) ns[attr.name.slice(6)] = attr.value;
			pieces.push({ text: ` ${attr.name}`, cls: 'attr' }, { text: '=', cls: 'punctuation' });
			const isNs = attr.name === 'xmlns' || attr.name.startsWith('xmlns:');
			pieces.push(
				isNs
					? { text: attr.raw, cls: 'string' }
					: { text: attr.raw, cls: 'string', path: `${path}/@${attr.name}`, value: attr.value }
			);
		}
		pieces.push({ text: end, cls: 'tag' });
		return pieces;
	};

	const draw = (node: Node, path: string, depth: number, soapBody: boolean): void => {
		if (node.kind === 'text') {
			const trimmed = node.raw.trim();
			if (trimmed) lines.push([pad(depth), { text: trimmed }]);
			return;
		}
		if (node.kind !== 'element') {
			lines.push([pad(depth), { text: node.raw.trim(), cls: node.kind === 'cdata' ? 'string' : 'comment' }]);
			return;
		}
		const kids = node.children.filter((child) => !(child.kind === 'text' && child.raw.trim() === ''));
		if (node.empty || kids.length === 0) {
			lines.push([pad(depth), ...openTag(node, path, node.empty ? '/>' : `></${node.name}>`)]);
			return;
		}
		const only = kids.length === 1 && (kids[0].kind === 'text' || kids[0].kind === 'cdata') ? kids[0] : null;
		if (only) {
			const raw = only.kind === 'text' ? only.raw.trim() : only.raw;
			const value = only.kind === 'text' ? unescapeXml(raw) : raw.slice(9, -3);
			lines.push([
				pad(depth),
				...openTag(node, path, '>'),
				{ text: raw, cls: only.kind === 'cdata' ? 'string' : undefined, path, value },
				{ text: `</${node.name}>`, cls: 'tag' }
			]);
			return;
		}
		lines.push([pad(depth), ...openTag(node, path, '>')]);
		const elements = kids.filter((kid) => kid.kind === 'element') as Extract<Node, { kind: 'element' }>[];
		const steps = path.split('/');
		const isBody = local(node.name) === 'Body' && /Envelope$/.test(steps[steps.length - 2] ?? '');
		for (const kid of kids) {
			if (kid.kind !== 'element') {
				draw(kid, path, depth + 1, soapBody);
				continue;
			}
			const same = elements.filter((other) => other.name === kid.name);
			const position = same.length > 1 ? `[${same.indexOf(kid) + 1}]` : '';
			const next = isBody ? `//${kid.name}${position}` : `${path}/${kid.name}${position}`;
			draw(kid, next, depth + 1, soapBody || isBody);
		}
		lines.push([pad(depth), { text: `</${node.name}>`, cls: 'tag' }]);
	};

	for (const node of nodes) {
		if (node.kind === 'element') draw(node, `/${node.name}`, 0, false);
		else draw(node, '', 0, false);
	}
	return { lines: lines.map((line) => line.filter((piece) => piece.text !== '')), ns };
}

/** The prefixes `path` uses, with their namespaces from `ns`. */
export function prefixesOf(path: string, ns: Record<string, string>): Record<string, string> {
	const used: Record<string, string> = {};
	for (const match of path.matchAll(/([A-Za-z_][\w.-]*):[A-Za-z_]/g)) {
		if (ns[match[1]] !== undefined) used[match[1]] = ns[match[1]];
	}
	return used;
}
