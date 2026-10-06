// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Markdown from an extension, read into a tree of plain values (FEAT-096).
 *
 * Review providers write markdown, and some of what they write is aimed at
 * other programs — HTML comments carrying state, `<details>` blocks, images,
 * instructions for coding agents. None of it is trusted, so none of it is ever
 * handed to the browser as markup: this module returns nodes whose leaves are
 * strings, `Markdown.svelte` renders those strings as text, and nothing in the
 * extension UI hands a string to the browser as markup for a parser bug to reach.
 *
 * What survives: paragraphs, headings, fenced and inline code, lists,
 * quotes, strong and emphasis, and links whose address is `https:`. HTML
 * comments are dropped; HTML tags are removed and their text kept; an image
 * becomes its alt text, so nothing remote is loaded by reading a review.
 */

export type Inline =
	| { t: 'text'; v: string }
	| { t: 'code'; v: string }
	| { t: 'strong'; c: Inline[] }
	| { t: 'em'; c: Inline[] }
	| { t: 'link'; href: string; c: Inline[] };

export type Block =
	| { t: 'p'; c: Inline[] }
	| { t: 'h'; level: 1 | 2 | 3; c: Inline[] }
	| { t: 'pre'; v: string }
	| { t: 'list'; ordered: boolean; items: Inline[][] }
	| { t: 'quote'; c: Block[] };

/** The longest text read. Past this a provider is not writing for a person. */
export const MAX_SOURCE = 100_000;

/** Remove HTML comments and tags, keeping the text between tags. */
export function stripHtml(source: string): string {
	return source
		.replace(/<!--[\s\S]*?(-->|$)/g, '')
		.replace(/<\/?[A-Za-z][^<>]*>/g, '');
}

/** A link address worth keeping: `https:` only, no whitespace. */
export function safeHref(href: string): string | null {
	const trimmed = href.trim();
	return /^https:\/\/[^\s<>"']+$/i.test(trimmed) ? trimmed : null;
}

export function parseInline(text: string): Inline[] {
	const out: Inline[] = [];
	let buffer = '';
	const flush = () => {
		if (buffer) out.push({ t: 'text', v: buffer });
		buffer = '';
	};

	let i = 0;
	while (i < text.length) {
		const rest = text.slice(i);

		// `code`
		if (rest[0] === '`') {
			const end = text.indexOf('`', i + 1);
			if (end > i) {
				flush();
				out.push({ t: 'code', v: text.slice(i + 1, end) });
				i = end + 1;
				continue;
			}
		}

		// ![alt](src) — never loaded; the alt text stands in.
		const image = /^!\[([^\]]*)\]\(([^)\s]*)[^)]*\)/.exec(rest);
		if (image) {
			buffer += image[1];
			i += image[0].length;
			continue;
		}

		// [text](href)
		const link = /^\[([^\]]+)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/.exec(rest);
		if (link) {
			const href = safeHref(link[2]);
			flush();
			if (href) out.push({ t: 'link', href, c: parseInline(link[1]) });
			else out.push(...parseInline(link[1]));
			i += link[0].length;
			continue;
		}

		// **strong** or __strong__
		const strong = /^(\*\*|__)(?=\S)([\s\S]*?\S)\1/.exec(rest);
		if (strong) {
			flush();
			out.push({ t: 'strong', c: parseInline(strong[2]) });
			i += strong[0].length;
			continue;
		}

		// *em* or _em_
		const em = /^(\*|_)(?=\S)([\s\S]*?\S)\1(?![*_\w])/.exec(rest);
		if (em) {
			flush();
			out.push({ t: 'em', c: parseInline(em[2]) });
			i += em[0].length;
			continue;
		}

		// A bare https address.
		const bare = /^https:\/\/[^\s<>"')\]]+/.exec(rest);
		if (bare && (i === 0 || /\s|\(/.test(text[i - 1]))) {
			flush();
			out.push({ t: 'link', href: bare[0], c: [{ t: 'text', v: bare[0] }] });
			i += bare[0].length;
			continue;
		}

		buffer += text[i];
		i += 1;
	}
	flush();
	return out;
}

/** Read `source` into blocks. */
export function parse(source: string): Block[] {
	const text = stripHtml(source.slice(0, MAX_SOURCE)).replace(/\r\n?/g, '\n');
	const lines = text.split('\n');
	const blocks: Block[] = [];
	let paragraph: string[] = [];

	const endParagraph = () => {
		if (paragraph.length) {
			blocks.push({ t: 'p', c: parseInline(paragraph.join(' ').trim()) });
			paragraph = [];
		}
	};

	let i = 0;
	while (i < lines.length) {
		const line = lines[i];

		const fence = /^\s*(```|~~~)/.exec(line);
		if (fence) {
			endParagraph();
			const body: string[] = [];
			i += 1;
			while (i < lines.length && !lines[i].trim().startsWith(fence[1])) {
				body.push(lines[i]);
				i += 1;
			}
			blocks.push({ t: 'pre', v: body.join('\n') });
			i += 1;
			continue;
		}

		if (!line.trim()) {
			endParagraph();
			i += 1;
			continue;
		}

		const heading = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line);
		if (heading) {
			endParagraph();
			const level = Math.min(heading[1].length, 3) as 1 | 2 | 3;
			blocks.push({ t: 'h', level, c: parseInline(heading[2]) });
			i += 1;
			continue;
		}

		const item = /^\s{0,3}([-*+]|\d{1,3}[.)])\s+(.*)$/.exec(line);
		if (item) {
			endParagraph();
			const ordered = /\d/.test(item[1]);
			const items: Inline[][] = [];
			while (i < lines.length) {
				const next = /^\s{0,3}([-*+]|\d{1,3}[.)])\s+(.*)$/.exec(lines[i]);
				if (!next || /\d/.test(next[1]) !== ordered) break;
				items.push(parseInline(next[2]));
				i += 1;
			}
			blocks.push({ t: 'list', ordered, items });
			continue;
		}

		if (/^\s{0,3}>/.test(line)) {
			endParagraph();
			const quoted: string[] = [];
			while (i < lines.length && /^\s{0,3}>/.test(lines[i])) {
				quoted.push(lines[i].replace(/^\s{0,3}>\s?/, ''));
				i += 1;
			}
			blocks.push({ t: 'quote', c: parse(quoted.join('\n')) });
			continue;
		}

		paragraph.push(line.trim());
		i += 1;
	}
	endParagraph();
	return blocks;
}

/** Every link address in `blocks`, for tests and for "copy link". */
export function links(blocks: Block[]): string[] {
	const found: string[] = [];
	const walk = (nodes: Inline[]) => {
		for (const node of nodes) {
			if (node.t === 'link') {
				found.push(node.href);
				walk(node.c);
			} else if (node.t === 'strong' || node.t === 'em') walk(node.c);
		}
	};
	for (const block of blocks) {
		if (block.t === 'p' || block.t === 'h') walk(block.c);
		else if (block.t === 'list') block.items.forEach(walk);
		else if (block.t === 'quote') found.push(...links(block.c));
	}
	return found;
}
