// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Lightweight syntax highlighter and language detector for diff views (FEAT-064).
 *
 * Fast, memory-safe, non-blocking tokenization across common programming languages
 * mapped directly to Spagitty's semantic theme palette tokens.
 *
 * A line at a time, with what runs past a line's end carried to the next
 * (FEAT-094): a block comment, a string that spans lines, a tag whose
 * attributes wrap, the script or style block of a markup file. `tokenize`
 * reads one line as if it were a file's first; `tokenizeLines` reads a file;
 * `tokenizeDiff` reads both sides of a diff at once.
 */

export type TokenType =
	| 'keyword'
	| 'string'
	| 'number'
	| 'comment'
	| 'fn'
	| 'type'
	| 'operator'
	| 'punctuation'
	| 'plain'
	/** A markup tag's name. */
	| 'tag'
	/** A markup attribute, or a key in a configuration file. */
	| 'attr'
	/** An annotation, decorator, attribute or preprocessor line. */
	| 'meta';

export interface Token {
	type: TokenType;
	text: string;
}

/** Something a line opened and did not close. */
interface Open {
	type: 'comment' | 'string' | 'tag';
	close: string;
	/**
	 * The tag whose attributes run on — `script` and `style` start a block —
	 * or, in Markdown, the language of the fenced code.
	 */
	tag?: string;
}

/** Where a file's reading stands at the end of a line. */
export interface LineState {
	open: Open | null;
	/** Inside a markup file's `<script>` or `<style>`: that block's language, and its own state. */
	embed: { language: string; state: LineState } | null;
}

/** A file's first line. */
export const START: LineState = { open: null, embed: null };

function makeMap(words: string[]): Record<string, true> {
	const map: Record<string, true> = {};
	for (const w of words) map[w] = true;
	return map;
}

const CSS_WORDS = makeMap(['important', 'inherit', 'initial', 'unset', 'none', 'auto']);

const KEYWORDS: Record<string, Record<string, true>> = {
	rust: makeMap([
		'as', 'async', 'await', 'break', 'const', 'continue', 'crate', 'dyn', 'else', 'enum',
		'extern', 'false', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop', 'match', 'mod',
		'move', 'mut', 'pub', 'ref', 'return', 'self', 'Self', 'static', 'struct', 'super',
		'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while'
	]),
	typescript: makeMap([
		'abstract', 'any', 'as', 'async', 'await', 'boolean', 'break', 'case', 'catch', 'class',
		'const', 'continue', 'debugger', 'declare', 'default', 'delete', 'do', 'else', 'enum',
		'export', 'extends', 'false', 'finally', 'for', 'from', 'function', 'get', 'if',
		'implements', 'import', 'in', 'infer', 'instanceof', 'interface', 'is', 'keyof',
		'let', 'module', 'namespace', 'never', 'new', 'null', 'number', 'of', 'package',
		'private', 'protected', 'public', 'readonly', 'return', 'set', 'static', 'string',
		'super', 'switch', 'symbol', 'this', 'throw', 'true', 'try', 'type', 'typeof',
		'undefined', 'unknown', 'var', 'void', 'while', 'with', 'yield'
	]),
	javascript: makeMap([
		'async', 'await', 'break', 'case', 'catch', 'class', 'const', 'continue', 'debugger',
		'default', 'delete', 'do', 'else', 'export', 'extends', 'false', 'finally', 'for',
		'from', 'function', 'if', 'import', 'in', 'instanceof', 'let', 'new', 'null', 'of',
		'return', 'static', 'super', 'switch', 'this', 'throw', 'true', 'try', 'typeof',
		'undefined', 'var', 'void', 'while', 'with', 'yield'
	]),
	python: makeMap([
		'and', 'as', 'assert', 'async', 'await', 'break', 'class', 'continue', 'def', 'del',
		'elif', 'else', 'except', 'False', 'finally', 'for', 'from', 'global', 'if', 'import',
		'in', 'is', 'lambda', 'None', 'nonlocal', 'not', 'or', 'pass', 'raise', 'return',
		'True', 'try', 'while', 'with', 'yield'
	]),
	go: makeMap([
		'break', 'case', 'chan', 'const', 'continue', 'default', 'defer', 'else', 'fallthrough',
		'for', 'func', 'go', 'goto', 'if', 'import', 'interface', 'map', 'package', 'range',
		'return', 'select', 'struct', 'switch', 'type', 'var', 'true', 'false', 'nil'
	]),
	cpp: makeMap([
		'auto', 'bool', 'break', 'case', 'catch', 'char', 'class', 'const', 'constexpr',
		'continue', 'default', 'delete', 'do', 'double', 'else', 'enum', 'explicit', 'export',
		'extern', 'false', 'float', 'for', 'friend', 'goto', 'if', 'inline', 'int', 'long',
		'mutable', 'namespace', 'new', 'noexcept', 'nullptr', 'operator', 'private',
		'protected', 'public', 'register', 'reinterpret_cast', 'return', 'short', 'signed',
		'sizeof', 'static', 'static_cast', 'struct', 'switch', 'template', 'this',
		'throw', 'true', 'try', 'typedef', 'typeid', 'typename', 'union', 'unsigned', 'using',
		'virtual', 'void', 'volatile', 'wchar_t', 'while'
	]),
	sql: makeMap([
		'select', 'from', 'where', 'insert', 'into', 'update', 'delete', 'table', 'create',
		'drop', 'alter', 'index', 'view', 'join', 'left', 'right', 'inner', 'outer', 'on',
		'group', 'by', 'order', 'having', 'limit', 'offset', 'as', 'and', 'or', 'not',
		'null', 'is', 'in', 'between', 'like', 'union', 'all', 'values', 'distinct', 'case',
		'when', 'then', 'else', 'end', 'primary', 'key', 'foreign', 'references'
	]),
	kotlin: makeMap([
		'abstract', 'annotation', 'as', 'break', 'by', 'catch', 'class', 'companion', 'const',
		'constructor', 'continue', 'crossinline', 'data', 'do', 'else', 'enum', 'external',
		'false', 'final', 'finally', 'for', 'fun', 'get', 'if', 'import', 'in', 'infix', 'init',
		'inline', 'inner', 'interface', 'internal', 'is', 'lateinit', 'noinline', 'null',
		'object', 'open', 'operator', 'out', 'override', 'package', 'private', 'protected',
		'public', 'reified', 'return', 'sealed', 'set', 'super', 'suspend', 'tailrec', 'this',
		'throw', 'true', 'try', 'typealias', 'val', 'value', 'var', 'vararg', 'when', 'where',
		'while'
	]),
	java: makeMap([
		'abstract', 'assert', 'boolean', 'break', 'byte', 'case', 'catch', 'char', 'class',
		'const', 'continue', 'default', 'do', 'double', 'else', 'enum', 'extends', 'false',
		'final', 'finally', 'float', 'for', 'if', 'implements', 'import', 'instanceof', 'int',
		'interface', 'long', 'native', 'new', 'null', 'package', 'private', 'protected',
		'public', 'record', 'return', 'sealed', 'short', 'static', 'super', 'switch',
		'synchronized', 'this', 'throw', 'throws', 'transient', 'true', 'try', 'var', 'void',
		'volatile', 'while', 'yield'
	]),
	groovy: makeMap([
		'as', 'assert', 'break', 'case', 'catch', 'class', 'def', 'default', 'do', 'else',
		'enum', 'extends', 'false', 'finally', 'for', 'if', 'implements', 'import', 'in',
		'instanceof', 'interface', 'new', 'null', 'package', 'return', 'static', 'super',
		'switch', 'this', 'throw', 'true', 'try', 'var', 'void', 'while'
	]),
	swift: makeMap([
		'as', 'associatedtype', 'async', 'await', 'break', 'case', 'catch', 'class',
		'continue', 'default', 'defer', 'deinit', 'do', 'else', 'enum', 'extension', 'false',
		'fileprivate', 'final', 'for', 'func', 'guard', 'if', 'import', 'in', 'init', 'inout',
		'internal', 'is', 'let', 'nil', 'open', 'operator', 'override', 'private', 'protocol',
		'public', 'repeat', 'rethrows', 'return', 'self', 'Self', 'some', 'static', 'struct',
		'subscript', 'super', 'switch', 'throw', 'throws', 'true', 'try', 'typealias', 'var',
		'where', 'while'
	]),
	csharp: makeMap([
		'abstract', 'as', 'async', 'await', 'base', 'bool', 'break', 'case', 'catch', 'class',
		'const', 'continue', 'decimal', 'default', 'delegate', 'do', 'double', 'else', 'enum',
		'event', 'explicit', 'false', 'finally', 'float', 'for', 'foreach', 'get', 'if',
		'implicit', 'in', 'init', 'int', 'interface', 'internal', 'is', 'lock', 'long',
		'namespace', 'new', 'null', 'object', 'out', 'override', 'params', 'private',
		'protected', 'public', 'readonly', 'record', 'ref', 'return', 'sealed', 'set',
		'static', 'string', 'struct', 'switch', 'this', 'throw', 'true', 'try', 'typeof',
		'using', 'var', 'virtual', 'void', 'while', 'yield'
	]),
	dart: makeMap([
		'abstract', 'as', 'async', 'await', 'break', 'case', 'catch', 'class', 'const',
		'continue', 'default', 'do', 'else', 'enum', 'extends', 'extension', 'factory',
		'false', 'final', 'finally', 'for', 'get', 'if', 'implements', 'import', 'in', 'is',
		'late', 'library', 'mixin', 'new', 'null', 'on', 'part', 'required', 'return', 'set',
		'static', 'super', 'switch', 'this', 'throw', 'true', 'try', 'var', 'void', 'while',
		'with', 'yield'
	]),
	php: makeMap([
		'abstract', 'array', 'as', 'break', 'case', 'catch', 'class', 'const', 'continue',
		'default', 'do', 'echo', 'else', 'elseif', 'enum', 'extends', 'false', 'final',
		'finally', 'fn', 'for', 'foreach', 'function', 'if', 'implements', 'interface',
		'match', 'namespace', 'new', 'null', 'private', 'protected', 'public', 'readonly',
		'return', 'static', 'switch', 'this', 'throw', 'trait', 'true', 'try', 'use', 'while'
	]),
	ruby: makeMap([
		'alias', 'and', 'begin', 'break', 'case', 'class', 'def', 'do', 'else', 'elsif', 'end',
		'ensure', 'false', 'for', 'if', 'in', 'module', 'next', 'nil', 'not', 'or', 'redo',
		'require', 'rescue', 'retry', 'return', 'self', 'super', 'then', 'true', 'undef',
		'unless', 'until', 'when', 'while', 'yield'
	]),
	scala: makeMap([
		'abstract', 'case', 'catch', 'class', 'def', 'do', 'else', 'enum', 'extends', 'false',
		'final', 'finally', 'for', 'given', 'if', 'implicit', 'import', 'lazy', 'match', 'new',
		'null', 'object', 'override', 'package', 'private', 'protected', 'return', 'sealed',
		'super', 'then', 'this', 'throw', 'trait', 'true', 'try', 'type', 'using', 'val', 'var',
		'while', 'with', 'yield'
	]),
	shell: makeMap([
		'case', 'do', 'done', 'elif', 'else', 'esac', 'export', 'fi', 'for', 'function', 'if',
		'in', 'local', 'readonly', 'return', 'select', 'then', 'until', 'while'
	]),
	dockerfile: makeMap([
		'ADD', 'ARG', 'AS', 'CMD', 'COPY', 'ENTRYPOINT', 'ENV', 'EXPOSE', 'FROM', 'HEALTHCHECK',
		'LABEL', 'MAINTAINER', 'ONBUILD', 'RUN', 'SHELL', 'STOPSIGNAL', 'USER', 'VOLUME',
		'WORKDIR'
	]),
	json: makeMap(['true', 'false', 'null']),
	toml: makeMap(['true', 'false']),
	yaml: makeMap(['true', 'false', 'null', 'yes', 'no', 'on', 'off']),
	ini: makeMap(['true', 'false']),
	css: CSS_WORDS,
	scss: CSS_WORDS
};

/** How a language writes comments and strings, beyond its words. */
interface Syntax {
	/** What starts a comment that runs to the end of the line. */
	line: string[];
	/** Comments that run until their close, across lines. */
	block: [string, string][];
	/** Strings that may run across lines, longest opener first. */
	multi: [string, string][];
	/** Quotes of strings that end with their line. */
	quotes: string;
	/** `@Name` annotations and decorators. */
	at?: boolean;
	/** Keywords match whatever their case. */
	caseless?: boolean;
	/** A key at the start of a line: `key =`, `key:`. */
	key?: RegExp;
	/** `[section]` lines. */
	sections?: boolean;
	/** `#include`-style lines. */
	directives?: boolean;
}

const C: Syntax = { line: ['//'], block: [['/*', '*/']], multi: [], quotes: '"\'' };
const TRIPLE: [string, string][] = [['"""', '"""']];

const SYNTAX: Record<string, Syntax> = {
	// Rust's strings may run across lines; `'` is a character or a lifetime.
	rust: { ...C, multi: [['r#"', '"#'], ['r"', '"'], ['"', '"']], quotes: '' },
	typescript: { ...C, multi: [['`', '`']], at: true },
	javascript: { ...C, multi: [['`', '`']], at: true },
	go: { ...C, multi: [['`', '`']] },
	cpp: { ...C, directives: true },
	java: { ...C, multi: TRIPLE, at: true },
	kotlin: { ...C, multi: TRIPLE, at: true },
	groovy: { ...C, multi: [["'''", "'''"], ...TRIPLE], at: true },
	swift: { ...C, multi: TRIPLE, at: true },
	csharp: { ...C, multi: TRIPLE },
	dart: { ...C, multi: [["'''", "'''"], ...TRIPLE], at: true },
	scala: { ...C, multi: TRIPLE, at: true },
	php: { ...C, line: ['//', '#'] },
	python: { line: ['#'], block: [], multi: [...TRIPLE, ["'''", "'''"]], quotes: '"\'', at: true },
	ruby: { line: ['#'], block: [], multi: [], quotes: '"\'' },
	shell: { line: ['#'], block: [], multi: [], quotes: '"\'' },
	dockerfile: { line: ['#'], block: [], multi: [], quotes: '"\'' },
	sql: { line: ['--'], block: [['/*', '*/']], multi: [], quotes: '"\'', caseless: true },
	json: { line: ['//'], block: [['/*', '*/']], multi: [], quotes: '"' },
	toml: {
		line: ['#'],
		block: [],
		multi: [...TRIPLE, ["'''", "'''"]],
		quotes: '"\'',
		key: /^\s*[A-Za-z0-9_.\-"']+(?=\s*=)/,
		sections: true
	},
	yaml: {
		line: ['#'],
		block: [],
		multi: [],
		quotes: '"\'',
		key: /^\s*(?:- )?[A-Za-z0-9_.\-/"']+(?=\s*:(?:\s|$))/
	},
	ini: {
		line: ['#', ';'],
		block: [],
		multi: [],
		quotes: '"\'',
		key: /^\s*[A-Za-z0-9_.-]+(?=\s*[=:])/,
		sections: true
	},
	css: { line: [], block: [['/*', '*/']], multi: [], quotes: '"\'' },
	scss: { line: ['//'], block: [['/*', '*/']], multi: [], quotes: '"\'' }
};

/** What Markdown fences name, as file languages. */
const FENCES: Record<string, string> = {
	ts: 'typescript', tsx: 'typescript', typescript: 'typescript', js: 'javascript',
	jsx: 'javascript', javascript: 'javascript', rs: 'rust', rust: 'rust', py: 'python',
	python: 'python', go: 'go', kt: 'kotlin', kts: 'kotlin', kotlin: 'kotlin', java: 'java',
	sh: 'shell', bash: 'shell', shell: 'shell', zsh: 'shell', console: 'shell', json: 'json',
	jsonc: 'json', yaml: 'yaml', yml: 'yaml', toml: 'toml', sql: 'sql', css: 'css',
	scss: 'scss', html: 'html', xml: 'xml', svelte: 'svelte', vue: 'vue', swift: 'swift',
	cs: 'csharp', csharp: 'csharp', c: 'cpp', cpp: 'cpp', groovy: 'groovy', gradle: 'groovy',
	dart: 'dart', php: 'php', rb: 'ruby', ruby: 'ruby', scala: 'scala', ini: 'ini',
	dockerfile: 'dockerfile', md: 'markdown', markdown: 'markdown'
};

/** The language a fence's info string names, or plain. */
export function fenceLanguage(info: string): string {
	return FENCES[info.trim().split(/\s+/)[0]?.toLowerCase() ?? ''] ?? 'plain';
}

export function detectLanguage(path: string | null | undefined): string {
	if (!path) return 'plain';
	const name = path.slice(path.lastIndexOf('/') + 1);
	if (/^(Dockerfile|Containerfile)(\..*)?$/.test(name) || name.endsWith('.dockerfile')) return 'dockerfile';
	if (name === '.env' || name.startsWith('.env.')) return 'ini';
	if (name === 'Cargo.lock') return 'toml';
	const ext = name.includes('.') ? (name.split('.').pop()?.toLowerCase() ?? '') : '';
	switch (ext) {
		case 'rs':
			return 'rust';
		case 'ts':
		case 'mts':
		case 'cts':
		case 'tsx':
			return 'typescript';
		case 'js':
		case 'mjs':
		case 'cjs':
		case 'jsx':
			return 'javascript';
		case 'svelte':
			return 'svelte';
		case 'vue':
			return 'vue';
		case 'html':
		case 'htm':
			return 'html';
		case 'xml':
		case 'xsd':
		case 'xsl':
		case 'svg':
		case 'plist':
		case 'csproj':
			return 'xml';
		case 'py':
		case 'pyi':
			return 'python';
		case 'go':
			return 'go';
		case 'c':
		case 'h':
		case 'cpp':
		case 'hpp':
		case 'cc':
		case 'cxx':
			return 'cpp';
		case 'kt':
		case 'kts':
			return 'kotlin';
		case 'java':
			return 'java';
		case 'gradle':
		case 'groovy':
			return 'groovy';
		case 'swift':
			return 'swift';
		case 'cs':
			return 'csharp';
		case 'dart':
			return 'dart';
		case 'php':
			return 'php';
		case 'rb':
			return 'ruby';
		case 'scala':
			return 'scala';
		case 'json':
		case 'jsonc':
			return 'json';
		case 'toml':
			return 'toml';
		case 'yaml':
		case 'yml':
			return 'yaml';
		case 'ini':
		case 'cfg':
		case 'conf':
		case 'properties':
		case 'env':
			return 'ini';
		case 'css':
			return 'css';
		case 'scss':
		case 'less':
			return 'scss';
		case 'md':
		case 'markdown':
		case 'mdx':
			return 'markdown';
		case 'sh':
		case 'bash':
		case 'zsh':
			return 'shell';
		case 'sql':
			return 'sql';
		default:
			return 'plain';
	}
}

export function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#39;');
}

/** One line read, and where that leaves the file. */
interface Read {
	tokens: Token[];
	state: LineState;
}

/** Past this length a line is not read: a minified bundle, not something to colour. */
const LONGEST = 2000;

const MARKUP = new Set(['svelte', 'vue', 'html', 'xml']);

/**
 * One line as if it were a file's first.
 *
 * A Svelte or Vue line read on its own, with nothing before it, is read as
 * script: that is where the Diff screen's hunks mostly fall.
 */
export function tokenize(line: string, language: string): Token[] {
	const lang = language === 'svelte' || language === 'vue' ? 'typescript' : language;
	return readLine(line, lang, START).tokens;
}

/** A file's lines, each read with what the lines before it left open. */
export function tokenizeLines(lines: string[], language: string): Token[][] {
	let state = START;
	return lines.map((line) => {
		const read = readLine(line, language, state);
		state = read.state;
		return read.tokens;
	});
}

/**
 * Both sides of a diff: the old file is its unchanged and removed lines, the
 * new one its unchanged and added lines, and each is read on its own, so a
 * comment opened on one side does not run into the other.
 */
export function tokenizeDiff(
	lines: { origin: 'context' | 'added' | 'removed'; text: string }[],
	language: string
): Token[][] {
	let old = START;
	let now = START;
	return lines.map((line) => {
		if (line.origin === 'removed') {
			const read = readLine(line.text, language, old);
			old = read.state;
			return read.tokens;
		}
		const read = readLine(line.text, language, now);
		now = read.state;
		if (line.origin === 'context') old = readLine(line.text, language, old).state;
		return read.tokens;
	});
}

function readLine(line: string, language: string, state: LineState): Read {
	if (language === 'plain' || !line || line.length > LONGEST) {
		return { tokens: [{ type: state.open?.type === 'comment' ? 'comment' : 'plain', text: line }], state };
	}
	if (MARKUP.has(language)) return markup(line, language, state);
	if (language === 'markdown') return markdown(line, state);
	return code(line, language, state, 0);
}

/** The index just past `close`, from `from`, skipping what a backslash escapes. */
function closing(line: string, from: number, close: string, escapes: boolean): number {
	let i = from;
	while (i < line.length) {
		if (escapes && line[i] === '\\') {
			i += 2;
			continue;
		}
		if (line.startsWith(close, i)) return i + close.length;
		i++;
	}
	return -1;
}

/** A backslash escapes inside a string closed by one quote; a triple quote is raw. */
function escapesIn(open: Open): boolean {
	return open.type === 'string' && open.close.length === 1;
}

function push(tokens: Token[], type: TokenType, text: string) {
	if (text) tokens.push({ type, text });
}

const OPERATOR = /[=+\-*/%&|^!~<>?:.]/;
const PUNCTUATION = /[{}()[\];,]/;
const NONE: Record<string, true> = {};

/** A programming or configuration language's line, from `start`. */
function code(line: string, language: string, state: LineState, start: number): Read {
	const syntax = SYNTAX[language] ?? C;
	const kwMap = KEYWORDS[language] ?? NONE;
	const tokens: Token[] = [];
	const len = line.length;
	let i = start;

	// What the last line left open, first.
	if (state.open) {
		const type = state.open.type === 'comment' ? 'comment' : 'string';
		const end = closing(line, i, state.open.close, escapesIn(state.open));
		if (end < 0) return { tokens: [{ type, text: line.slice(i) }], state };
		push(tokens, type, line.slice(i, end));
		i = end;
	}

	if (i === 0 && syntax.sections) {
		const section = /^\s*\[\[?[^\]]*\]\]?/.exec(line);
		if (section) {
			push(tokens, 'type', section[0]);
			i = section[0].length;
		}
	}
	if (i === 0 && syntax.key) {
		const key = syntax.key.exec(line);
		if (key) {
			const lead = key[0].length - key[0].trimStart().length;
			push(tokens, 'plain', key[0].slice(0, lead));
			push(tokens, 'attr', key[0].slice(lead));
			i = key[0].length;
		}
	}
	if (i === 0 && syntax.directives && /^\s*#/.test(line)) {
		const cut = line.indexOf('//');
		push(tokens, 'meta', line.slice(0, cut < 0 ? len : cut));
		if (cut >= 0) push(tokens, 'comment', line.slice(cut));
		return { tokens, state: START };
	}
	if (i === 0 && (language === 'css' || language === 'scss')) {
		const read = css(line, language);
		if (read) return read;
	}

	while (i < len) {
		const char = line[i];
		const next = i + 1 < len ? line[i + 1] : '';

		// Block comments, which may run on.
		const block = syntax.block.find(([open]) => line.startsWith(open, i));
		if (block) {
			const end = closing(line, i + block[0].length, block[1], false);
			if (end < 0) {
				push(tokens, 'comment', line.slice(i));
				return { tokens, state: { open: { type: 'comment', close: block[1] }, embed: null } };
			}
			push(tokens, 'comment', line.slice(i, end));
			i = end;
			continue;
		}

		// Line comments. A `#` is one only where a word could start.
		const lineComment = syntax.line.find(
			(open) => line.startsWith(open, i) && (open !== '#' || i === 0 || /\s/.test(line[i - 1]))
		);
		if (lineComment) {
			push(tokens, 'comment', line.slice(i));
			break;
		}

		// Rust: a character, or a lifetime.
		if (language === 'rust' && char === "'") {
			const literal = /^'(?:\\[^']*|[^\\'])'/.exec(line.slice(i));
			const lifetime = literal ? null : /^'[A-Za-z_]\w*/.exec(line.slice(i));
			const found = literal ?? lifetime;
			if (found) {
				push(tokens, literal ? 'string' : 'meta', found[0]);
				i += found[0].length;
				continue;
			}
		}

		// Rust attributes: `#[derive(Debug)]`, `#![allow(dead_code)]`.
		if (language === 'rust' && char === '#' && (next === '[' || (next === '!' && line[i + 2] === '['))) {
			const end = line.indexOf(']', i);
			const stop = end < 0 ? len : end + 1;
			push(tokens, 'meta', line.slice(i, stop));
			i = stop;
			continue;
		}

		// Strings that may run on.
		const multi = syntax.multi.find(([open]) => line.startsWith(open, i));
		if (multi) {
			const open: Open = { type: 'string', close: multi[1] };
			const end = closing(line, i + multi[0].length, multi[1], escapesIn(open));
			if (end < 0) {
				push(tokens, 'string', line.slice(i));
				return { tokens, state: { open, embed: null } };
			}
			push(tokens, 'string', line.slice(i, end));
			i = end;
			continue;
		}

		// Strings that end with the line.
		if (syntax.quotes.includes(char)) {
			const found = closing(line, i + 1, char, true);
			const end = found < 0 ? len : found;
			// A JSON key is a string followed by a colon.
			const isKey = language === 'json' && /^\s*:/.test(line.slice(end));
			push(tokens, isKey ? 'attr' : 'string', line.slice(i, end));
			i = end;
			continue;
		}

		// Annotations and decorators.
		if (syntax.at && char === '@' && /[A-Za-z_]/.test(next)) {
			const name = /^@[A-Za-z_][\w.]*/.exec(line.slice(i))![0];
			push(tokens, 'meta', name);
			i += name.length;
			continue;
		}

		// Numbers.
		if (/\d/.test(char) && (i === 0 || /[\s,([{:;=+\-*/%<>&|!]/.test(line[i - 1]))) {
			let end = i + 1;
			while (end < len && /[0-9a-fA-FxX._]/.test(line[end])) end++;
			push(tokens, 'number', line.slice(i, end));
			i = end;
			continue;
		}

		// Identifiers, keywords, types, functions.
		if (/[a-zA-Z_$]/.test(char)) {
			let end = i + 1;
			while (end < len && /[a-zA-Z0-9_$]/.test(line[end])) end++;
			const word = line.slice(i, end);
			// A Rust macro: `println!(`.
			const bang = language === 'rust' && line[end] === '!' && /[([{]/.test(line[end + 1] ?? '');

			let peek = bang ? end + 1 : end;
			while (peek < len && line[peek] === ' ') peek++;
			const isFn = bang || (peek < len && line[peek] === '(');

			if (kwMap[word] || (syntax.caseless && kwMap[word.toLowerCase()])) {
				push(tokens, 'keyword', word);
			} else if (isFn) {
				push(tokens, 'fn', bang ? `${word}!` : word);
				if (bang) end++;
			} else if (/^[A-Z][a-zA-Z0-9_]*$/.test(word)) {
				push(tokens, 'type', word);
			} else {
				push(tokens, 'plain', word);
			}
			i = end;
			continue;
		}

		// Operators.
		if (OPERATOR.test(char)) {
			let end = i + 1;
			while (end < len && OPERATOR.test(line[end])) end++;
			push(tokens, 'operator', line.slice(i, end));
			i = end;
			continue;
		}

		// Punctuation.
		if (PUNCTUATION.test(char)) {
			push(tokens, 'punctuation', char);
			i++;
			continue;
		}

		// Whitespace and anything else.
		let end = i + 1;
		while (end < len && /\s/.test(line[end])) end++;
		push(tokens, 'plain', line.slice(i, end));
		i = end;
	}

	return { tokens, state: START };
}

/**
 * A style sheet's line, when it is an at-rule, a selector or a declaration;
 * null for anything else, which reads as code.
 */
function css(line: string, language: string): Read | null {
	if (line.includes('/*') || (language === 'scss' && line.includes('//'))) return null;
	const tokens: Token[] = [];
	const trimmed = line.trim();
	const rule = /^(\s*)(@[\w-]+)/.exec(line);
	if (rule) {
		push(tokens, 'plain', rule[1]);
		push(tokens, 'keyword', rule[2]);
		tokens.push(...code(line, language, START, rule[0].length).tokens);
		return { tokens, state: START };
	}
	// A selector: what opens a block, or one of a list of them.
	if (/[{,]$/.test(trimmed) && !/:\s*[^\s{]+;/.test(trimmed)) {
		const brace = line.lastIndexOf('{');
		push(tokens, 'tag', brace < 0 ? line : line.slice(0, brace));
		if (brace >= 0) push(tokens, 'punctuation', line.slice(brace));
		return { tokens, state: START };
	}
	// A declaration: `property: value;`.
	const declaration = /^(\s*)(--[\w-]+|-?[a-z][\w-]*)(\s*:)(.*)$/.exec(line);
	if (declaration) {
		push(tokens, 'plain', declaration[1]);
		push(tokens, 'attr', declaration[2]);
		push(tokens, 'operator', declaration[3]);
		tokens.push(...cssValue(declaration[4]));
		return { tokens, state: START };
	}
	return null;
}

/** A declaration's value: colours and sizes, strings, functions, words. */
function cssValue(value: string): Token[] {
	const tokens: Token[] = [];
	const pattern = /#[0-9a-fA-F]{3,8}\b|-?\d*\.?\d+[a-z%]*|"[^"]*"?|'[^']*'?|!important|[a-zA-Z-]+|\s+|./g;
	for (const match of value.matchAll(pattern)) {
		const text = match[0];
		const after = value[(match.index ?? 0) + text.length];
		if (/^#|^-?\.?\d/.test(text)) push(tokens, 'number', text);
		else if (/^["']/.test(text)) push(tokens, 'string', text);
		else if (text === '!important') push(tokens, 'keyword', text);
		else if (/^[a-zA-Z-]+$/.test(text)) push(tokens, after === '(' ? 'fn' : CSS_WORDS[text] ? 'keyword' : 'plain', text);
		else if (PUNCTUATION.test(text)) push(tokens, 'punctuation', text);
		else push(tokens, 'plain', text);
	}
	return tokens;
}

/** The language a `<script>` or `<style>` tag's body is in, or null. */
function embedded(tag: string, attributes: string): string | null {
	if (tag === 'style') return /lang=["']?(scss|less)/.test(attributes) ? 'scss' : 'css';
	if (tag === 'script') return /lang=["']?(ts|typescript)/.test(attributes) ? 'typescript' : 'javascript';
	return null;
}

/** The tag that ends an embedded block. */
function ender(language: string): string {
	return language === 'css' || language === 'scss' ? '</style' : '</script';
}

/** A markup file's line: tags, attributes, comments, and the script and style blocks inside. */
function markup(line: string, language: string, state: LineState): Read {
	const tokens: Token[] = [];
	const len = line.length;
	const expressions = language === 'svelte' || language === 'vue';
	let i = 0;
	let open = state.open;
	let attributes = '';

	// An embedded block, up to its closing tag, in its own language.
	const inside = (from: number, inner: string, innerState: LineState): Read | null => {
		const rest = line.slice(from);
		const at = rest.toLowerCase().indexOf(ender(inner));
		const read = code(at < 0 ? rest : rest.slice(0, at), inner, innerState, 0);
		tokens.push(...read.tokens);
		if (at < 0) return { tokens, state: { open: null, embed: { language: inner, state: read.state } } };
		i = from + at;
		return null;
	};

	if (state.embed) {
		const carried = inside(0, state.embed.language, state.embed.state);
		if (carried) return carried;
	}

	while (i < len) {
		// A comment, or one carried over.
		if (open?.type === 'comment' || line.startsWith('<!--', i)) {
			const end = closing(line, open?.type === 'comment' ? i : i + 4, '-->', false);
			if (end < 0) {
				push(tokens, 'comment', line.slice(i));
				return { tokens, state: { open: { type: 'comment', close: '-->' }, embed: null } };
			}
			push(tokens, 'comment', line.slice(i, end));
			open = null;
			i = end;
			continue;
		}

		// Inside a tag: attributes until `>`.
		if (open?.type === 'tag') {
			const char = line[i];
			if (char === '>' || (char === '/' && line[i + 1] === '>')) {
				const end = char === '>' ? i + 1 : i + 2;
				push(tokens, 'punctuation', line.slice(i, end));
				i = end;
				const inner = char === '>' && open.tag ? embedded(open.tag, attributes) : null;
				open = null;
				attributes = '';
				if (inner) {
					const carried = inside(i, inner, START);
					if (carried) return carried;
				}
				continue;
			}
			if (char === '"' || char === "'") {
				const found = closing(line, i + 1, char, false);
				const end = found < 0 ? len : found;
				push(tokens, 'string', line.slice(i, end));
				attributes += line.slice(i, end);
				i = end;
				continue;
			}
			if (char === '{' && expressions) {
				i = expression(line, i, tokens);
				continue;
			}
			if (char === '=') {
				push(tokens, 'operator', char);
				attributes += char;
				i++;
				continue;
			}
			const name = /^[^\s=>"'{/]+/.exec(line.slice(i));
			if (name) {
				push(tokens, 'attr', name[0]);
				attributes += ` ${name[0]}`;
				i += name[0].length;
				continue;
			}
			let end = i + 1;
			while (end < len && /\s/.test(line[end])) end++;
			push(tokens, 'plain', line.slice(i, end));
			i = end;
			continue;
		}

		// A tag opens.
		const tag = /^<(\/?)([A-Za-z][\w.:-]*)/.exec(line.slice(i));
		if (tag) {
			push(tokens, 'punctuation', tag[1] ? '</' : '<');
			push(tokens, 'tag', tag[2]);
			i += tag[0].length;
			open = { type: 'tag', close: '>', tag: tag[1] ? undefined : tag[2].toLowerCase() };
			continue;
		}

		// A Svelte or Vue expression in text.
		if (line[i] === '{' && expressions) {
			i = expression(line, i, tokens);
			continue;
		}

		// Text, up to the next tag or expression.
		let end = i + 1;
		while (end < len && line[end] !== '<' && !(line[end] === '{' && expressions)) end++;
		push(tokens, 'plain', line.slice(i, end));
		i = end;
	}

	return { tokens, state: { open, embed: null } };
}

/** `{expression}`, `{#if x}`, `{{ value }}`: braces as punctuation, a block's word as a keyword, the rest as script. */
function expression(line: string, from: number, tokens: Token[]): number {
	let depth = 0;
	let end = from;
	while (end < line.length) {
		if (line[end] === '{') depth++;
		else if (line[end] === '}' && --depth === 0) {
			end++;
			break;
		}
		end++;
	}
	const text = line.slice(from, end);
	const opener = /^\{\{?\s*(?:([#:/@])([a-z]+))?/.exec(text)!;
	const close = /\}?\}$/.exec(text)?.[0] ?? '';
	if (opener[2]) {
		push(tokens, 'punctuation', opener[0].slice(0, opener[0].length - opener[2].length));
		push(tokens, 'keyword', opener[2]);
	} else {
		push(tokens, 'punctuation', opener[0]);
	}
	const body = text.slice(opener[0].length, Math.max(opener[0].length, text.length - close.length));
	if (body) tokens.push(...code(body, 'typescript', START, 0).tokens);
	push(tokens, 'punctuation', close);
	return end;
}

/** A Markdown line: headings, quotes, list marks, code, links. */
function markdown(line: string, state: LineState): Read {
	const tokens: Token[] = [];
	const fence = /^\s*(```|~~~)(.*)$/.exec(line);
	if (state.open) {
		if (fence && fence[1] === state.open.close) return { tokens: [{ type: 'meta', text: line }], state: START };
		const language = state.open.tag ?? 'plain';
		return { tokens: language === 'plain' ? [{ type: 'string', text: line }] : tokenize(line, language), state };
	}
	if (fence) {
		return {
			tokens: [{ type: 'meta', text: line }],
			state: { open: { type: 'string', close: fence[1], tag: fenceLanguage(fence[2]) }, embed: null }
		};
	}
	if (/^\s{0,3}#{1,6}(\s|$)/.test(line)) return { tokens: [{ type: 'keyword', text: line }], state: START };
	if (/^\s*>/.test(line)) return { tokens: [{ type: 'comment', text: line }], state: START };

	let i = 0;
	const mark = /^\s*(?:[-*+]|\d+[.)])\s/.exec(line);
	if (mark) {
		push(tokens, 'operator', mark[0]);
		i = mark[0].length;
	}
	let at = i;
	for (const match of line.slice(i).matchAll(/`[^`]*`|\[[^\]]*\]\([^)]*\)|\*\*[^*]+\*\*|__[^_]+__/g)) {
		const from = i + (match.index ?? 0);
		const text = match[0];
		push(tokens, 'plain', line.slice(at, from));
		if (text.startsWith('`')) push(tokens, 'string', text);
		else if (text.startsWith('[')) {
			const cut = text.indexOf('](');
			push(tokens, 'fn', text.slice(0, cut + 1));
			push(tokens, 'string', text.slice(cut + 1));
		} else push(tokens, 'keyword', text);
		at = from + text.length;
	}
	push(tokens, 'plain', line.slice(at));
	return { tokens, state: START };
}

export function highlightLine(line: string, language: string): string {
	const tokens = tokenize(line, language);
	return tokens
		.map((t) => {
			const safe = escapeHtml(t.text);
			if (t.type === 'plain') return safe;
			return `<span class="tok-${t.type}">${safe}</span>`;
		})
		.join('');
}

/** A run of a line's text with its colour, and whether the diff changed it. */
export interface Painted {
	text: string;
	type: TokenType;
	changed: boolean;
}

/**
 * A line's tokens cut at its changed words (FEAT-090), so each run carries
 * both its colour and its change. The two cut the line in different places;
 * every boundary of either is a boundary here.
 */
export function paint(tokens: Token[], pieces: { text: string; changed: boolean }[] | undefined): Painted[] {
	if (!pieces || pieces.length === 0) return tokens.map((token) => ({ ...token, changed: false }));
	const out: Painted[] = [];
	let p = 0;
	let left = pieces[0].text.length;
	for (const token of tokens) {
		let text = token.text;
		while (text.length > 0) {
			while (left === 0 && p < pieces.length - 1) {
				p++;
				left = pieces[p].text.length;
			}
			const take = left > 0 ? Math.min(left, text.length) : text.length;
			out.push({ text: text.slice(0, take), type: token.type, changed: pieces[p].changed });
			text = text.slice(take);
			left = Math.max(0, left - take);
		}
	}
	return out;
}
