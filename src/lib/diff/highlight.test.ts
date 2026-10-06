// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Syntax highlighter tests (FEAT-064).
 */

import { describe, expect, it } from 'vitest';
import {
	detectLanguage,
	escapeHtml,
	highlightLine,
	paint,
	tokenize,
	tokenizeDiff,
	tokenizeLines,
	type Token
} from './highlight';

describe('detectLanguage', () => {
	it('detects common file extensions correctly', () => {
		expect(detectLanguage('src/main.rs')).toBe('rust');
		expect(detectLanguage('src/app.ts')).toBe('typescript');
		expect(detectLanguage('component.svelte')).toBe('svelte');
		expect(detectLanguage('script.py')).toBe('python');
		expect(detectLanguage('server.go')).toBe('go');
		expect(detectLanguage('main.cpp')).toBe('cpp');
		expect(detectLanguage('query.sql')).toBe('sql');
		expect(detectLanguage('config.toml')).toBe('toml');
		expect(detectLanguage('package.json')).toBe('json');
		expect(detectLanguage('deploy.yaml')).toBe('yaml');
		expect(detectLanguage('script.sh')).toBe('shell');
		expect(detectLanguage('README.txt')).toBe('plain');
		expect(detectLanguage(null)).toBe('plain');
	});
});

describe('escapeHtml', () => {
	it('escapes dangerous HTML characters', () => {
		expect(escapeHtml('<script>alert("xss") & \'test\'</script>')).toBe(
			'&lt;script&gt;alert(&quot;xss&quot;) &amp; &#39;test&#39;&lt;/script&gt;'
		);
	});
});

describe('tokenize', () => {
	it('tokenizes Rust keywords, types, and functions', () => {
		const tokens = tokenize('pub fn calculate_total(count: i32) -> Result<Total, Error> {', 'rust');
		expect(tokens).toEqual([
			{ type: 'keyword', text: 'pub' },
			{ type: 'plain', text: ' ' },
			{ type: 'keyword', text: 'fn' },
			{ type: 'plain', text: ' ' },
			{ type: 'fn', text: 'calculate_total' },
			{ type: 'punctuation', text: '(' },
			{ type: 'plain', text: 'count' },
			{ type: 'operator', text: ':' },
			{ type: 'plain', text: ' ' },
			{ type: 'plain', text: 'i32' },
			{ type: 'punctuation', text: ')' },
			{ type: 'plain', text: ' ' },
			{ type: 'operator', text: '->' },
			{ type: 'plain', text: ' ' },
			{ type: 'type', text: 'Result' },
			{ type: 'operator', text: '<' },
			{ type: 'type', text: 'Total' },
			{ type: 'punctuation', text: ',' },
			{ type: 'plain', text: ' ' },
			{ type: 'type', text: 'Error' },
			{ type: 'operator', text: '>' },
			{ type: 'plain', text: ' ' },
			{ type: 'punctuation', text: '{' }
		]);
	});

	it('tokenizes strings and numbers', () => {
		const tokens = tokenize('const msg = "hello world" + 42;', 'typescript');
		expect(tokens).toContainEqual({ type: 'keyword', text: 'const' });
		expect(tokens).toContainEqual({ type: 'string', text: '"hello world"' });
		expect(tokens).toContainEqual({ type: 'number', text: '42' });
	});

	it('tokenizes line comments', () => {
		const tokens = tokenize('let x = 1; // comment here', 'typescript');
		expect(tokens).toContainEqual({ type: 'comment', text: '// comment here' });
	});

	it('tokenizes python hash comments', () => {
		const tokens = tokenize('def run(): # start execution', 'python');
		expect(tokens).toContainEqual({ type: 'keyword', text: 'def' });
		expect(tokens).toContainEqual({ type: 'comment', text: '# start execution' });
	});

	it('gracefully handles empty and plain strings', () => {
		expect(tokenize('', 'rust')).toEqual([{ type: 'plain', text: '' }]);
		expect(tokenize('plain text line', 'plain')).toEqual([{ type: 'plain', text: 'plain text line' }]);
	});
});

describe('highlightLine', () => {
	it('wraps tokens into semantic HTML spans', () => {
		const html = highlightLine('let mut value = 100;', 'rust');
		expect(html).toContain('<span class="tok-keyword">let</span>');
		expect(html).toContain('<span class="tok-keyword">mut</span>');
		expect(html).toContain('<span class="tok-number">100</span>');
	});
});

/** The tokens of one type on a line, as text. */
const of = (tokens: Token[], type: Token['type']) => tokens.filter((t) => t.type === type).map((t) => t.text);

describe('more languages (FEAT-094)', () => {
	it('knows Kotlin, Gradle, markup, styles, Markdown and configuration by name', () => {
		expect(detectLanguage('app/src/main/MainActivity.kt')).toBe('kotlin');
		expect(detectLanguage('app/build.gradle.kts')).toBe('kotlin');
		expect(detectLanguage('build.gradle')).toBe('groovy');
		expect(detectLanguage('Main.java')).toBe('java');
		expect(detectLanguage('res/layout/main.xml')).toBe('xml');
		expect(detectLanguage('index.html')).toBe('html');
		expect(detectLanguage('app.css')).toBe('css');
		expect(detectLanguage('README.md')).toBe('markdown');
		expect(detectLanguage('docker/Dockerfile')).toBe('dockerfile');
		expect(detectLanguage('.env.local')).toBe('ini');
		expect(detectLanguage('gradle.properties')).toBe('ini');
		expect(detectLanguage('Cargo.lock')).toBe('toml');
	});

	it('colours Kotlin', () => {
		const line = tokenize('@Composable fun Greeting(name: String) { val x = 1 }', 'kotlin');
		expect(of(line, 'meta')).toEqual(['@Composable']);
		expect(of(line, 'keyword')).toEqual(['fun', 'val']);
		expect(of(line, 'fn')).toEqual(['Greeting']);
		expect(of(line, 'type')).toEqual(['String']);
	});

	it('names the keys and sections of a TOML file', () => {
		const [section, entry] = tokenizeLines(['[versions]', 'kotlin = "2.4.20"'], 'toml');
		expect(section).toEqual([{ type: 'type', text: '[versions]' }]);
		expect(of(entry, 'attr')).toEqual(['kotlin']);
		expect(of(entry, 'string')).toEqual(['"2.4.20"']);
	});

	it('names JSON keys apart from their values', () => {
		const line = tokenize('  "name": "spagitty",', 'json');
		expect(of(line, 'attr')).toEqual(['"name"']);
		expect(of(line, 'string')).toEqual(['"spagitty"']);
	});

	it('reads Rust lifetimes, characters, macros and attributes', () => {
		const line = tokenize("fn a<'a>(x: &'a str) -> char { println!(\"{}\", 'x') }", 'rust');
		expect(of(line, 'meta')).toEqual(["'a", "'a"]);
		expect(of(line, 'string')).toEqual(['"{}"', "'x'"]);
		expect(of(line, 'fn')).toEqual(['println!']);
		expect(of(tokenize('#[derive(Debug)]', 'rust'), 'meta')).toEqual(['#[derive(Debug)]']);
	});

	it('colours a style sheet', () => {
		const [selector, declaration, rule] = tokenizeLines(
			['.btn:hover {', '\tcolor: #fff;', '@media (width < 600px) {'],
			'css'
		);
		expect(of(selector, 'tag')).toEqual(['.btn:hover ']);
		expect(of(declaration, 'attr')).toEqual(['color']);
		expect(of(declaration, 'number')).toEqual(['#fff']);
		expect(of(rule, 'keyword')).toEqual(['@media']);
	});
});

describe('tokenizeLines (FEAT-094)', () => {
	it('carries a block comment from line to line', () => {
		const lines = tokenizeLines(['let a = 1; /* one', 'two', 'three */ const b = 2;'], 'typescript');
		expect(of(lines[0], 'comment')).toEqual(['/* one']);
		expect(lines[1]).toEqual([{ type: 'comment', text: 'two' }]);
		expect(of(lines[2], 'comment')).toEqual(['three */']);
		expect(of(lines[2], 'keyword')).toEqual(['const']);
	});

	it('carries a string that spans lines', () => {
		const lines = tokenizeLines(['val sql = """', '  select * from t', '""".trim()'], 'kotlin');
		expect(lines[1]).toEqual([{ type: 'string', text: '  select * from t' }]);
		expect(of(lines[2], 'string')).toEqual(['"""']);
		expect(of(lines[2], 'fn')).toEqual(['trim']);
		const python = tokenizeLines(['"""Doc', 'more', '"""', 'def f(): pass'], 'python');
		expect(python[1]).toEqual([{ type: 'string', text: 'more' }]);
		expect(of(python[3], 'keyword')).toEqual(['def', 'pass']);
	});

	it('reads a Svelte file as markup, with its script and style in their own languages', () => {
		const lines = tokenizeLines(
			[
				'<script lang="ts">',
				'\tconst a: number = 1;',
				'</script>',
				'<div class="x" onclick={() => go(a)}>{#if a}{a}{/if}</div>',
				'<!-- a',
				'note -->',
				'<style>',
				'\t.x {',
				'\t\tcolor: red;',
				'\t}',
				'</style>'
			],
			'svelte'
		);
		expect(of(lines[0], 'tag')).toEqual(['script']);
		expect(of(lines[0], 'attr')).toEqual(['lang']);
		expect(of(lines[1], 'keyword')).toEqual(['const', 'number']);
		expect(of(lines[2], 'tag')).toEqual(['script']);
		expect(of(lines[3], 'tag')).toEqual(['div', 'div']);
		expect(of(lines[3], 'attr')).toEqual(['class', 'onclick']);
		expect(of(lines[3], 'fn')).toEqual(['go']);
		expect(of(lines[3], 'keyword')).toEqual(['if', 'if']);
		expect(lines[5]).toEqual([{ type: 'comment', text: 'note -->' }]);
		expect(of(lines[7], 'tag')).toEqual(['\t.x ']);
		expect(of(lines[8], 'attr')).toEqual(['color']);
		expect(of(lines[10], 'tag')).toEqual(['style']);
	});

	it('reads a tag whose attributes wrap', () => {
		const lines = tokenizeLines(['<Button', '  android:text="@string/ok"', '/>', 'text'], 'xml');
		expect(of(lines[1], 'attr')).toEqual(['android:text']);
		expect(of(lines[1], 'string')).toEqual(['"@string/ok"']);
		expect(lines[3]).toEqual([{ type: 'plain', text: 'text' }]);
	});

	it('reads Markdown headings, quotes and fenced code in its language', () => {
		const lines = tokenizeLines(
			['# Title', '> quoted', '```ts', 'const a = 1;', '```', '- `code` and [a](b)'],
			'markdown'
		);
		expect(lines[0]).toEqual([{ type: 'keyword', text: '# Title' }]);
		expect(lines[1]).toEqual([{ type: 'comment', text: '> quoted' }]);
		expect(of(lines[3], 'keyword')).toEqual(['const']);
		expect(of(lines[5], 'string')).toEqual(['`code`', '(b)']);
		expect(of(lines[5], 'fn')).toEqual(['[a]']);
	});
});

describe('tokenizeDiff (FEAT-094)', () => {
	it('reads the old and new sides apart', () => {
		const lines = tokenizeDiff(
			[
				{ origin: 'context', text: 'let a = 1;' },
				{ origin: 'removed', text: '/* gone' },
				{ origin: 'added', text: 'let b = 2;' },
				{ origin: 'removed', text: 'still gone */' },
				{ origin: 'context', text: 'let c = 3;' }
			],
			'typescript'
		);
		expect(of(lines[1], 'comment')).toEqual(['/* gone']);
		// The comment the removed line opened is not on the new side.
		expect(of(lines[2], 'keyword')).toEqual(['let']);
		expect(lines[3]).toEqual([{ type: 'comment', text: 'still gone */' }]);
		expect(of(lines[4], 'keyword')).toEqual(['let']);
	});
});

describe('paint (FEAT-094)', () => {
	it('cuts tokens at the changed words, keeping both', () => {
		const tokens: Token[] = [
			{ type: 'keyword', text: 'let' },
			{ type: 'plain', text: ' ' },
			{ type: 'string', text: '"abc"' }
		];
		expect(
			paint(tokens, [
				{ text: 'let "a', changed: false },
				{ text: 'bc"', changed: true }
			])
		).toEqual([
			{ type: 'keyword', text: 'let', changed: false },
			{ type: 'plain', text: ' ', changed: false },
			{ type: 'string', text: '"a', changed: false },
			{ type: 'string', text: 'bc"', changed: true }
		]);
		expect(paint(tokens, undefined).map((p) => p.changed)).toEqual([false, false, false]);
	});
});
