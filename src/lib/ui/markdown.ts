// SPDX-License-Identifier: GPL-3.0-or-later

import { Marked, type Tokens } from 'marked';
import { escapeHtml, fenceLanguage, tokenizeLines } from '$lib/diff/highlight';

/**
 * Markdown as GitHub and GitLab draw it (FEAT-094): tables, task lists, nested
 * lists, line breaks where the writer broke the line, fenced code in colour,
 * and the HTML a description may hold — dependabot's `<details>` above all.
 *
 * What a pull request or a comment says is written by whoever wrote it, and
 * this window can run git. So nothing `marked` makes is trusted: it is parsed
 * inert, in a `<template>`, and a new tree is built from it holding only the
 * elements and attributes listed here. Nothing is removed from the parsed
 * tree; what is not copied is not there, so an `onerror` cannot survive by
 * being one nobody thought to strip. A link is kept only if it parses as
 * http(s) or mail. Images become links to them: the window's content policy
 * loads nothing from the network.
 */

/** Elements copied, each with the attributes it may keep besides `title`. */
const ELEMENTS: Record<string, string[]> = {
	a: ['href'],
	abbr: [],
	b: [],
	blockquote: [],
	br: [],
	code: [],
	dd: [],
	del: [],
	details: ['open'],
	div: ['align'],
	dl: [],
	dt: [],
	em: [],
	h1: ['align'],
	h2: ['align'],
	h3: ['align'],
	h4: ['align'],
	h5: ['align'],
	h6: ['align'],
	hr: [],
	i: [],
	input: ['type', 'checked'],
	ins: [],
	kbd: [],
	li: ['value'],
	mark: [],
	ol: ['start'],
	p: ['align'],
	pre: [],
	q: [],
	s: [],
	samp: [],
	small: [],
	span: [],
	strike: [],
	strong: [],
	sub: [],
	summary: [],
	sup: [],
	table: [],
	tbody: [],
	td: ['align', 'colspan', 'rowspan'],
	tfoot: [],
	th: ['align', 'colspan', 'rowspan'],
	thead: [],
	tr: [],
	u: [],
	ul: [],
	var: []
};

/** Elements dropped with everything inside them. Any other unknown element is unwrapped. */
const DROPPED = new Set([
	'applet', 'audio', 'base', 'button', 'canvas', 'dialog', 'embed', 'form', 'frame', 'frameset',
	'head', 'iframe', 'link', 'math', 'meta', 'noscript', 'object', 'option', 'script', 'select',
	'style', 'svg', 'template', 'textarea', 'title', 'video'
]);

/** Classes the renderer itself writes; any other class is dropped. */
const OWN_CLASS = /^(?:tok-[a-z]+|code-block|code-lang)$/;

/** A link's address, if it is one to follow: http(s) or mail, nothing relative. */
export function safeUrl(value: string): string | null {
	try {
		const url = new URL(value.trim());
		return /^(?:https?|mailto):$/.test(url.protocol) ? url.href : null;
	} catch {
		return null;
	}
}

const marked = new Marked({
	gfm: true,
	// A line break in a description or a comment is one on both hosts.
	breaks: true,
	renderer: {
		code({ text, lang }: Tokens.Code): string {
			const name = (lang ?? '').trim().split(/\s+/)[0] ?? '';
			const body = tokenizeLines(text.split('\n'), fenceLanguage(name))
				.map((tokens) =>
					tokens
						.map((token) =>
							token.type === 'plain'
								? escapeHtml(token.text)
								: `<span class="tok-${token.type}">${escapeHtml(token.text)}</span>`
						)
						.join('')
				)
				.join('\n');
			const label = name ? `<span class="code-lang">${escapeHtml(name)}</span>` : '';
			return `<div class="code-block">${label}<pre><code>${body}</code></pre></div>\n`;
		}
	}
});

/** A link opening in a window of its own, or the words alone when there is nowhere to go. */
function link(href: string | null, words: string): HTMLElement {
	if (!href) {
		const span = document.createElement('span');
		span.textContent = words;
		return span;
	}
	const a = document.createElement('a');
	a.href = href;
	a.target = '_blank';
	a.rel = 'noopener noreferrer';
	a.textContent = words;
	return a;
}

/** A copy of `from`'s children under `into`, keeping only what is listed. */
function rebuild(from: Node, into: Node): void {
	for (const node of [...from.childNodes]) {
		if (node.nodeType === Node.TEXT_NODE) {
			into.appendChild(document.createTextNode(node.textContent ?? ''));
			continue;
		}
		if (node.nodeType !== Node.ELEMENT_NODE) continue;
		const element = node as Element;
		const tag = element.tagName.toLowerCase();
		if (DROPPED.has(tag)) continue;

		if (tag === 'img') {
			const words = element.getAttribute('alt')?.trim() || 'image';
			into.appendChild(link(safeUrl(element.getAttribute('src') ?? ''), words));
			continue;
		}
		const allowed = ELEMENTS[tag];
		if (!allowed) {
			// Unknown, not dangerous: its contents without it.
			rebuild(element, into);
			continue;
		}
		if (tag === 'input' && element.getAttribute('type') !== 'checkbox') continue;

		const copy = document.createElement(tag);
		for (const name of ['title', ...allowed]) {
			const value = element.getAttribute(name);
			if (value === null) continue;
			if (name === 'href') {
				const href = safeUrl(value);
				if (!href) continue;
				copy.setAttribute('href', href);
				copy.setAttribute('target', '_blank');
				copy.setAttribute('rel', 'noopener noreferrer');
				continue;
			}
			copy.setAttribute(name, value);
		}
		const classes = (element.getAttribute('class') ?? '').split(/\s+/).filter((name) => OWN_CLASS.test(name));
		if (classes.length > 0) copy.setAttribute('class', classes.join(' '));
		// A task list's box shows its state; it is not a control.
		if (tag === 'input') copy.setAttribute('disabled', '');
		rebuild(element, copy);
		into.appendChild(copy);
	}
}

/** `source` as safe HTML. */
export function renderMarkdown(source: string): string {
	if (!source.trim()) return '';
	const parsed = document.createElement('template');
	parsed.innerHTML = marked.parse(source, { async: false }) as string;
	const out = document.createElement('div');
	rebuild(parsed.content, out);
	return out.innerHTML.trim();
}

/** What `source` says, as plain words: for a preview of a comment, clipped to a few lines. */
export function markdownText(source: string): string {
	const holder = document.createElement('div');
	holder.innerHTML = renderMarkdown(source);
	for (const label of holder.querySelectorAll('.code-lang')) label.remove();
	return (holder.textContent ?? '').replace(/\s+/g, ' ').trim();
}
