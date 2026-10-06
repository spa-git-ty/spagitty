// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Packing and inspecting `.spagitty-extension` files.
 *
 * A package is a ZIP with `extension.json`, `integrity.json` (the SHA-256 of
 * every other file), the programs, and whatever else the extension ships.
 * **The checksums prove the files are the ones that were packed; they say
 * nothing about who packed them.**
 *
 * Packing only writes entrypoints for targets whose program actually exists:
 * a listed target is a claim that the package runs there, so one with no
 * program is dropped from the packaged manifest rather than shipped as a
 * promise. Spagitty runs the same checks again on import and is the authority.
 */

import { createHash } from 'node:crypto';
import { existsSync, lstatSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { parseManifest, type Manifest, type Target } from './manifest';
import { readZip, writeZip, type ZipEntry } from './zip';

export const MANIFEST = 'extension.json';
export const INTEGRITY = 'integrity.json';
const EXECUTABLE = /\.(exe|dll|so|dylib|bat|cmd|ps1|sh|com|msi|scr|node)$/i;

export function sha256(data: Uint8Array): string {
	return createHash('sha256').update(data).digest('hex');
}

export function looksExecutable(data: Uint8Array): boolean {
	const b = data;
	return (
		(b[0] === 0x4d && b[1] === 0x5a) ||
		(b[0] === 0x7f && b[1] === 0x45 && b[2] === 0x4c && b[3] === 0x46) ||
		(b[0] === 0x23 && b[1] === 0x21) ||
		[
			[0xfe, 0xed, 0xfa, 0xce],
			[0xfe, 0xed, 0xfa, 0xcf],
			[0xce, 0xfa, 0xed, 0xfe],
			[0xcf, 0xfa, 0xed, 0xfe],
			[0xca, 0xfe, 0xba, 0xbe]
		].some((magic) => magic.every((byte, i) => b[i] === byte))
	);
}

/** Files to leave out of a package: sources, tests and tooling, not runtime. */
const SKIP = [/^node_modules(\/|$)/, /^\.git(\/|$)/, /^src(\/|$)/, /^test(s)?(\/|$)/, /^dist\/.*\.spagitty-extension$/, /\.spagitty-extension$/, /^(bun\.lockb?|package\.json|tsconfig\.json|bunfig\.toml)$/, /^\.DS_Store$/];

function walk(root: string, dir: string, out: string[]): void {
	for (const name of readdirSync(dir)) {
		const path = join(dir, name);
		const rel = relative(root, path).split(sep).join('/');
		const stat = lstatSync(path);
		if (stat.isSymbolicLink()) throw new Error(`${rel} is a link; packages cannot contain links`);
		if (SKIP.some((pattern) => pattern.test(rel))) continue;
		if (stat.isDirectory()) walk(root, path, out);
		else out.push(rel);
	}
}

export interface Packed {
	bytes: Uint8Array;
	manifest: Manifest;
	targets: Target[];
	dropped: Target[];
	files: { path: string; size: number; sha256: string }[];
}

/** Build a package from `dir`. */
export function pack(dir: string): Packed {
	const manifest = parseManifest(readFileSync(join(dir, MANIFEST), 'utf8'));
	const entrypoints = manifest.runtime.entrypoints;
	const targets: Target[] = [];
	const dropped: Target[] = [];
	for (const [target, path] of Object.entries(entrypoints) as [Target, string][]) {
		if (existsSync(join(dir, ...path.split('/')))) targets.push(target);
		else dropped.push(target);
	}
	if (!targets.length) {
		throw new Error(`no program is built for any target it lists (${dropped.join(', ')}); build one first`);
	}
	const packaged: Manifest = {
		...manifest,
		runtime: { ...manifest.runtime, entrypoints: Object.fromEntries(targets.map((t) => [t, entrypoints[t]!])) }
	};
	const programs = new Set(Object.values(packaged.runtime.entrypoints));

	const names: string[] = [];
	walk(dir, dir, names);
	const entries: ZipEntry[] = [];
	const files: Packed['files'] = [];
	const index: Record<string, string> = {};
	for (const name of names.sort()) {
		if (name === INTEGRITY) continue;
		const data =
			name === MANIFEST ? new TextEncoder().encode(`${JSON.stringify(packaged, null, 2)}\n`) : new Uint8Array(readFileSync(join(dir, ...name.split('/'))));
		const program = programs.has(name);
		const looksLikeOne = EXECUTABLE.test(name) || looksExecutable(data);
		if (looksLikeOne && !program) {
			// A program for a target that was not built is left out with it.
			const forDroppedTarget = dropped.some((t) => entrypoints[t] === name);
			if (forDroppedTarget) continue;
			throw new Error(`${name} is a program the manifest does not declare as an entrypoint`);
		}
		const digest = sha256(data);
		index[name] = digest;
		files.push({ path: name, size: data.length, sha256: digest });
		entries.push({ name, data, mode: program ? 0o100755 : 0o100644 });
	}
	const integrity = new TextEncoder().encode(`${JSON.stringify({ version: 1, algorithm: 'sha256', files: index }, null, 2)}\n`);
	entries.push({ name: INTEGRITY, data: integrity, mode: 0o100644 });
	return { bytes: writeZip(entries), manifest: packaged, targets, dropped, files };
}

export interface Inspection {
	manifest: Manifest;
	digest: string;
	files: { path: string; size: number; sha256: string; program: boolean }[];
	problems: string[];
}

/** Describe a package and every way it would be refused. */
export function inspect(bytes: Uint8Array): Inspection {
	const entries = readZip(bytes);
	const problems: string[] = [];
	const manifestEntry = entries.find((e) => e.name === MANIFEST);
	if (!manifestEntry) throw new Error('it has no extension.json');
	const manifest = parseManifest(new TextDecoder().decode(manifestEntry.data));
	const indexEntry = entries.find((e) => e.name === INTEGRITY);
	const index: Record<string, string> = indexEntry ? JSON.parse(new TextDecoder().decode(indexEntry.data)).files ?? {} : {};
	if (!indexEntry) problems.push('it has no integrity.json');
	const programs = new Set(Object.values(manifest.runtime.entrypoints));
	const seen = new Set<string>();
	const files = entries
		.filter((e) => e.name !== INTEGRITY)
		.map((e) => {
			const digest = sha256(e.data);
			if (seen.has(e.name.toLowerCase())) problems.push(`${e.name} appears twice`);
			seen.add(e.name.toLowerCase());
			if (index[e.name] !== digest) problems.push(`${e.name} does not match integrity.json`);
			const program = programs.has(e.name);
			if (!program && (EXECUTABLE.test(e.name) || looksExecutable(e.data) || (e.mode & 0o111) !== 0)) {
				problems.push(`${e.name} is an undeclared program`);
			}
			return { path: e.name, size: e.data.length, sha256: digest, program };
		});
	for (const name of Object.keys(index)) if (!entries.some((e) => e.name === name)) problems.push(`integrity.json lists ${name}, which is missing`);
	for (const [target, path] of Object.entries(manifest.runtime.entrypoints)) {
		if (!files.some((f) => f.path === path)) problems.push(`the program for ${target} (${path}) is missing`);
	}
	return { manifest, digest: sha256(bytes), files, problems };
}
