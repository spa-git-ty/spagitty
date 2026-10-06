// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { render } from '../../testing/mount';
import Markdown from './Markdown.svelte';
import { links, parse, parseInline, safeHref, stripHtml } from './markdown';

describe('reading markdown into values', () => {
	it('keeps the structure a review is written in', () => {
		const blocks = parse('# Title\n\nSome **bold** and *em* text.\n\n- one\n- two\n\n1. first\n\n```\ncode()\n```\n\n> quoted');
		expect(blocks.map((b) => b.t)).toEqual(['h', 'p', 'list', 'list', 'pre', 'quote']);
		expect(blocks[4]).toEqual({ t: 'pre', v: 'code()' });
	});

	it('drops comments and tags but keeps their text', () => {
		expect(stripHtml('a<!-- state: {"x":1} -->b')).toBe('ab');
		expect(stripHtml('<details><summary>More</summary>inside</details>')).toBe('Moreinside');
		expect(stripHtml('<!-- never closed')).toBe('');
	});

	it('keeps only https links and never an executable one', () => {
		expect(safeHref('https://example.com/x')).toBe('https://example.com/x');
		for (const href of ['javascript:alert(1)', 'http://example.com', 'data:text/html,x', 'file:///etc/passwd', 'https://a b']) {
			expect(safeHref(href)).toBeNull();
		}
		const inline = parseInline('[safe](https://ok.example) and [bad](javascript:alert(1))');
		expect(links(parse('[safe](https://ok.example) [bad](javascript:alert(1))'))).toEqual(['https://ok.example']);
		expect(inline.some((n) => n.t === 'link' && n.href.startsWith('javascript'))).toBe(false);
	});

	it('turns an image into its alt text so nothing remote is loaded', () => {
		expect(parseInline('![a tracking pixel](https://track.example/p.gif)')).toEqual([
			{ t: 'text', v: 'a tracking pixel' }
		]);
	});

	it('finds bare https addresses but not ones glued to other words', () => {
		expect(links(parse('see https://docs.example/a for more'))).toEqual(['https://docs.example/a']);
		expect(links(parse('xhttps://docs.example/a'))).toEqual([]);
	});

	it('stops reading at the size limit', () => {
		const huge = 'a'.repeat(200_000);
		const [block] = parse(huge);
		expect(block.t === 'p' && block.c[0].t === 'text' && block.c[0].v.length).toBe(100_000);
	});
});

describe('drawing it', () => {
	it('renders hostile markup as text, never as elements', () => {
		const view = render(Markdown, {
			source: '<img src=x onerror="alert(1)"> <script>alert(2)</script> **ok** [x](javascript:alert(3))'
		});
		expect(view.find('img')).toBeNull();
		expect(view.find('script')).toBeNull();
		expect(view.find('a')).toBeNull();
		expect(view.find('strong')?.textContent).toBe('ok');
		expect(view.text()).toContain('alert(2)');
		view.destroy();
	});

	it('draws a link as a button that says where it goes', () => {
		const view = render(Markdown, { source: '[docs](https://docs.example/x)' });
		const button = view.get('button.link');
		expect(button.getAttribute('title')).toBe('Copy https://docs.example/x');
		expect(view.find('a')).toBeNull();
		view.destroy();
	});
});
