// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What kind of file a name is, as a small coloured tag (FEAT-149).
 *
 * A tree of plain names reads as one grey column; the eye finds `Cargo.toml`
 * among sixty files faster when the manifests, the styles and the code each
 * have a colour. The tag is two or three letters on a tint, not a picture:
 * every theme can draw it, it needs no icon font, and it says what it is to
 * somebody who has never seen the logo it would otherwise borrow.
 *
 * Colours are hues only. The lightness and the strength come from the theme,
 * so a tag is as quiet in light as in dark.
 */

export interface FileKind {
	/** Two or three letters, or a symbol. */
	tag: string;
	/** A hue, 0–360, or null for a grey sheet. */
	hue: number | null;
	/** What the tag stands for, for the title. */
	label: string;
}

const kind = (tag: string, hue: number | null, label: string): FileKind => ({ tag, hue, label });

const CODE = {
	ts: kind('TS', 215, 'TypeScript'),
	js: kind('JS', 52, 'JavaScript'),
	rs: kind('RS', 28, 'Rust'),
	py: kind('PY', 205, 'Python'),
	go: kind('GO', 190, 'Go'),
	java: kind('JV', 15, 'Java'),
	kt: kind('KT', 270, 'Kotlin'),
	swift: kind('SW', 20, 'Swift'),
	c: kind('C', 220, 'C'),
	cpp: kind('C++', 225, 'C++'),
	cs: kind('C#', 285, 'C#'),
	rb: kind('RB', 355, 'Ruby'),
	php: kind('PHP', 245, 'PHP'),
	svelte: kind('SV', 12, 'Svelte'),
	vue: kind('VUE', 150, 'Vue'),
	jsx: kind('JSX', 195, 'React'),
	html: kind('<>', 18, 'HTML'),
	css: kind('#', 265, 'Stylesheet'),
	sh: kind('$', 120, 'Shell script'),
	ps: kind('PS', 210, 'PowerShell'),
	sql: kind('SQL', 40, 'SQL'),
	lua: kind('LUA', 235, 'Lua'),
	dart: kind('DT', 195, 'Dart'),
	ex: kind('EX', 280, 'Elixir'),
	hs: kind('HS', 275, 'Haskell'),
	zig: kind('ZIG', 40, 'Zig')
};

const DATA = {
	json: kind('{}', 45, 'JSON'),
	yaml: kind('YML', 330, 'YAML'),
	toml: kind('TML', 30, 'TOML'),
	xml: kind('XML', 25, 'XML'),
	csv: kind('CSV', 140, 'Table'),
	md: kind('MD', 200, 'Markdown'),
	txt: kind('TXT', null, 'Text'),
	lock: kind('LCK', null, 'Lock file'),
	env: kind('ENV', 60, 'Environment'),
	image: kind('IMG', 300, 'Image'),
	font: kind('FNT', null, 'Font'),
	pdf: kind('PDF', 0, 'PDF'),
	archive: kind('ZIP', 35, 'Archive'),
	git: kind('GIT', null, 'Git file'),
	docker: kind('DKR', 205, 'Docker'),
	licence: kind('©', null, 'Licence'),
	make: kind('MK', 100, 'Build file')
};

const BY_EXTENSION: Record<string, FileKind> = {
	ts: CODE.ts, mts: CODE.ts, cts: CODE.ts,
	tsx: CODE.jsx, jsx: CODE.jsx,
	js: CODE.js, mjs: CODE.js, cjs: CODE.js,
	rs: CODE.rs,
	py: CODE.py, pyi: CODE.py,
	go: CODE.go,
	java: CODE.java,
	kt: CODE.kt, kts: CODE.kt,
	swift: CODE.swift,
	c: CODE.c, h: CODE.c,
	cc: CODE.cpp, cpp: CODE.cpp, cxx: CODE.cpp, hpp: CODE.cpp, hh: CODE.cpp,
	cs: CODE.cs,
	rb: CODE.rb,
	php: CODE.php,
	svelte: CODE.svelte,
	vue: CODE.vue,
	html: CODE.html, htm: CODE.html,
	css: CODE.css, scss: CODE.css, sass: CODE.css, less: CODE.css,
	sh: CODE.sh, bash: CODE.sh, zsh: CODE.sh, fish: CODE.sh,
	ps1: CODE.ps, psm1: CODE.ps,
	sql: CODE.sql,
	lua: CODE.lua,
	dart: CODE.dart,
	ex: CODE.ex, exs: CODE.ex,
	hs: CODE.hs,
	zig: CODE.zig,
	json: DATA.json, jsonc: DATA.json, json5: DATA.json,
	yml: DATA.yaml, yaml: DATA.yaml,
	toml: DATA.toml,
	xml: DATA.xml, svg: DATA.image,
	csv: DATA.csv, tsv: DATA.csv,
	md: DATA.md, mdx: DATA.md, markdown: DATA.md,
	txt: DATA.txt, log: DATA.txt,
	lock: DATA.lock,
	env: DATA.env,
	png: DATA.image, jpg: DATA.image, jpeg: DATA.image, gif: DATA.image, webp: DATA.image, ico: DATA.image, bmp: DATA.image, avif: DATA.image,
	ttf: DATA.font, otf: DATA.font, woff: DATA.font, woff2: DATA.font,
	pdf: DATA.pdf,
	zip: DATA.archive, gz: DATA.archive, tgz: DATA.archive, tar: DATA.archive, xz: DATA.archive, '7z': DATA.archive
};

const BY_NAME: Record<string, FileKind> = {
	dockerfile: DATA.docker,
	'docker-compose.yml': DATA.docker,
	'compose.yml': DATA.docker,
	makefile: DATA.make,
	justfile: DATA.make,
	'cmakelists.txt': DATA.make,
	license: DATA.licence,
	licence: DATA.licence,
	copying: DATA.licence,
	notice: DATA.licence,
	'.gitignore': DATA.git,
	'.gitattributes': DATA.git,
	'.gitmodules': DATA.git,
	'.gitlab-ci.yml': DATA.yaml,
	'.env': DATA.env,
	'.editorconfig': DATA.txt,
	'cargo.lock': DATA.lock,
	'bun.lock': DATA.lock,
	'package-lock.json': DATA.lock,
	'yarn.lock': DATA.lock,
	'pnpm-lock.yaml': DATA.lock
};

/** A file nothing here recognises: a plain sheet, no colour. */
export const PLAIN: FileKind = kind('', null, 'File');

/** The kind of the file at `path` (or a bare name). */
export function fileKind(path: string): FileKind {
	const name = path.slice(path.lastIndexOf('/') + 1).toLowerCase();
	const named = BY_NAME[name];
	if (named) return named;
	if (name.startsWith('.env.')) return DATA.env;
	if (name.startsWith('license') || name.startsWith('licence')) return DATA.licence;
	const dot = name.lastIndexOf('.');
	if (dot <= 0) return PLAIN;
	return BY_EXTENSION[name.slice(dot + 1)] ?? PLAIN;
}

