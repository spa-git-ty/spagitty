// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import { markdownText, renderMarkdown, safeUrl } from './markdown';

/** The rendered HTML, parsed back, to ask about. */
function dom(source: string): HTMLElement {
	const holder = document.createElement('div');
	holder.innerHTML = renderMarkdown(source);
	return holder;
}

describe('renderMarkdown (FEAT-094)', () => {
	it('draws a table, as dependabot writes one', () => {
		const html = dom(
			['| Package | From | To |', '| --- | --- | --- |', '| [io.ktor:ktor-client-core](https://github.com/ktorio/ktor) | `3.5.2` | `3.6.0` |'].join('\n')
		);
		expect([...html.querySelectorAll('th')].map((th) => th.textContent)).toEqual(['Package', 'From', 'To']);
		const cells = [...html.querySelectorAll('td')];
		expect(cells.map((td) => td.textContent)).toEqual(['io.ktor:ktor-client-core', '3.5.2', '3.6.0']);
		expect(cells[1].querySelector('code')).not.toBeNull();
	});

	it('keeps the HTML a description holds: details, summary, blockquote', () => {
		const html = dom('<details>\n<summary>Release notes</summary>\n<blockquote>\n<p>Fixed a leak</p>\n</blockquote>\n</details>');
		expect(html.querySelector('details summary')?.textContent).toBe('Release notes');
		expect(html.querySelector('details blockquote')?.textContent?.trim()).toBe('Fixed a leak');
	});

	it('nests lists, ticks task lists and breaks lines where the writer did', () => {
		const html = dom('- one\n  - inner\n- [x] done\n\nfirst\nsecond');
		expect(html.querySelector('ul ul li')?.textContent).toBe('inner');
		const box = html.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
		expect(box.checked).toBe(true);
		expect(box.disabled).toBe(true);
		expect(html.querySelector('p br')).not.toBeNull();
	});

	it('colours fenced code by its language', () => {
		const html = dom('```kotlin\nval x = "a"\n```');
		expect(html.querySelector('.code-lang')?.textContent).toBe('kotlin');
		expect(html.querySelector('.code-block code .tok-keyword')?.textContent).toBe('val');
		expect(html.querySelector('.code-block code .tok-string')?.textContent).toBe('"a"');
	});

	it('opens links in a window of their own', () => {
		const link = dom('[site](https://example.com)').querySelector('a')!;
		expect(link.getAttribute('href')).toBe('https://example.com/');
		expect(link.getAttribute('target')).toBe('_blank');
		expect(link.getAttribute('rel')).toBe('noopener noreferrer');
	});

	it('runs nothing a stranger wrote', () => {
		const html = dom(
			[
				'[click](javascript:alert(1))',
				'<a href="javascript:alert(2)">two</a>',
				'<script>alert(3)</script>',
				'<img src="x" onerror="alert(4)">',
				'<div onclick="alert(5)" style="position:fixed" class="btn primary" id="app">over</div>',
				'<iframe src="https://example.com"></iframe>',
				'<form action="https://example.com"><button>go</button></form>',
				'[relative](docs/readme.md)'
			].join('\n\n')
		);
		const text = html.innerHTML;
		expect(text).not.toContain('javascript:');
		expect(text).not.toContain('<script');
		expect(text).not.toContain('onerror');
		expect(text).not.toContain('onclick');
		expect(text).not.toContain('style=');
		expect(text).not.toContain('class="btn');
		expect(text).not.toContain('id="app"');
		expect(html.querySelector('iframe, form, button')).toBeNull();
		// Links that cannot be followed keep their words and lose their target.
		expect([...html.querySelectorAll('a')].every((a) => /^https?:|^mailto:/.test(a.getAttribute('href') ?? 'https:'))).toBe(true);
		expect(html.textContent).toContain('relative');
	});

	it('turns an image into a link to it, as nothing is loaded from the network', () => {
		const html = dom('![a screenshot](https://example.com/shot.png)');
		expect(html.querySelector('img')).toBeNull();
		const link = html.querySelector('a')!;
		expect(link.textContent).toBe('a screenshot');
		expect(link.getAttribute('href')).toBe('https://example.com/shot.png');
	});

	it('follows only http, https and mail, however the scheme is dressed', () => {
		expect(safeUrl('https://example.com/a')).toBe('https://example.com/a');
		expect(safeUrl('mailto:a@b.c')).toBe('mailto:a@b.c');
		for (const bad of ['javascript:alert(1)', ' JAVASCRIPT:alert(1)', 'java	script:alert(1)', 'data:text/html,<b>x</b>', 'vbscript:x', 'file:///etc/passwd', 'docs/a.md', '/a', '#top']) {
			expect(safeUrl(bad)).toBeNull();
		}
	});

	it('says nothing for nothing', () => {
		expect(renderMarkdown('  \n')).toBe('');
	});
});

describe('markdownText (FEAT-094)', () => {
	it('gives the words without the marks, for a preview', () => {
		expect(markdownText('Use `tempfile`:\n\n```rust\nlet f = tempfile()?;\n```\n\n**Soon.**')).toBe(
			'Use tempfile: let f = tempfile()?; Soon.'
		);
	});
});
