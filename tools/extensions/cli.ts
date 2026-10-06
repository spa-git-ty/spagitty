// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * `bun run ext <action>` — the extension developer's tools (FEAT-096).
 *
 *   create <id> [dir]          scaffold an extension with a command, a panel,
 *                              a setting and tests
 *   dev <dir>                  attach a development folder to Spagitty
 *   validate <dir|file>        the host's manifest and package rules
 *   test <dir>                 the extension's own tests, then protocol
 *                              conformance against a fake host
 *   build <dir> [--target t]   compile the worker for one or more targets
 *   pack <dir> [--build] [--target t] [--out file]
 *                              build if asked, then write a validated package
 *   inspect <file>             identity, contents, targets and integrity
 *
 * Every action works on paths given on the command line; none publishes
 * anything anywhere.
 */

import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isExtensionId, parseManifest, TARGETS, validate, type Target } from '../../packages/extension-sdk/src/manifest';
import { inspect, pack } from '../../packages/extension-sdk/src/package';
import { FakeHost } from '../../packages/extension-sdk/src/testing/index';

const here = dirname(fileURLToPath(import.meta.url));
const sdk = resolve(here, '../../packages/extension-sdk');
const template = join(sdk, 'templates', 'basic');

/** Where `bun build --compile` names each target. */
export const BUN_TARGETS: Partial<Record<Target, string>> = {
	'x86_64-pc-windows-msvc': 'bun-windows-x64',
	'x86_64-unknown-linux-gnu': 'bun-linux-x64',
	'aarch64-unknown-linux-gnu': 'bun-linux-arm64',
	'x86_64-apple-darwin': 'bun-darwin-x64',
	'aarch64-apple-darwin': 'bun-darwin-arm64'
};

/** The target this machine is, in the manifest's vocabulary. */
export function hostTarget(): Target {
	const arch = process.arch === 'arm64' ? 'aarch64' : 'x86_64';
	if (process.platform === 'win32') return `${arch}-pc-windows-msvc` as Target;
	if (process.platform === 'darwin') return `${arch}-apple-darwin` as Target;
	return `${arch}-unknown-linux-gnu` as Target;
}

/** Spagitty's data directory, as Tauri computes it for `dev.spagitty.app`. */
export function dataDir(env: NodeJS.ProcessEnv = process.env, platform = process.platform): string {
	if (env.SPAGITTY_DATA_DIR) return env.SPAGITTY_DATA_DIR;
	const id = 'dev.spagitty.app';
	if (platform === 'win32') return join(env.APPDATA ?? join(homedir(), 'AppData', 'Roaming'), id);
	if (platform === 'darwin') return join(homedir(), 'Library', 'Application Support', id);
	return join(env.XDG_DATA_HOME ?? join(homedir(), '.local', 'share'), id);
}

class Failure extends Error {}

function fail(message: string): never {
	throw new Failure(message);
}

function readManifestFrom(dir: string) {
	const path = join(dir, 'extension.json');
	if (!existsSync(path)) fail(`${dir} has no extension.json`);
	return parseManifest(readFileSync(path, 'utf8'));
}

export function create(id: string, target?: string, publisher = 'Unknown'): string {
	if (!isExtensionId(id)) fail(`${id} is not an extension id; use reverse-DNS such as com.example.hello`);
	const dir = resolve(target ?? id.split('.').pop()!);
	if (existsSync(dir) && readdirSync(dir).length) fail(`${dir} is not empty`);
	const program = id.split('.').pop()!;
	const name = program.charAt(0).toUpperCase() + program.slice(1);
	const values: Record<string, string> = {
		id,
		name,
		program,
		publisher,
		license: 'GPL-3.0-or-later',
		sdk: sdk.split('\\').join('/'),
		cli: resolve(here, 'cli.ts').split('\\').join('/')
	};
	cpSync(template, dir, { recursive: true });
	const fill = (path: string) => {
		for (const entry of readdirSync(path)) {
			const full = join(path, entry);
			if (statSync(full).isDirectory()) fill(full);
			else writeFileSync(full, readFileSync(full, 'utf8').replace(/\{\{(\w+)\}\}/g, (_, key: string) => values[key] ?? ''));
		}
	};
	fill(dir);
	return dir;
}

export function attachRequest(dir: string, env: NodeJS.ProcessEnv = process.env): string {
	const manifest = readManifestFrom(dir);
	const requests = join(dataDir(env), 'extensions', 'development-requests');
	mkdirSync(requests, { recursive: true });
	const file = join(requests, `${manifest.id}.json`);
	writeFileSync(file, JSON.stringify({ path: resolve(dir) }));
	return file;
}

export function validatePath(path: string): string[] {
	if (statSync(path).isDirectory()) {
		const text = readFileSync(join(path, 'extension.json'), 'utf8');
		let value: unknown;
		try {
			value = JSON.parse(text);
		} catch (error) {
			return [`extension.json is not JSON: ${(error as Error).message}`];
		}
		const errors = validate(value);
		if (errors.length) return errors;
		const manifest = parseManifest(text);
		const built = Object.values(manifest.runtime.entrypoints).filter((p) => existsSync(join(path, ...p!.split('/'))));
		return built.length ? [] : ['warning: no program is built yet; run `ext build`'];
	}
	return inspect(new Uint8Array(readFileSync(path))).problems;
}

export function build(dir: string, targets: Target[]): string[] {
	const manifest = readManifestFrom(dir);
	const entry = existsSync(join(dir, 'src', 'main.ts')) ? join(dir, 'src', 'main.ts') : fail('build expects src/main.ts');
	const built: string[] = [];
	for (const target of targets) {
		const out = manifest.runtime.entrypoints[target];
		if (!out) fail(`the manifest lists no program for ${target}`);
		const bunTarget = BUN_TARGETS[target] ?? fail(`bun cannot build for ${target}`);
		const outfile = join(dir, ...out.split('/'));
		mkdirSync(dirname(outfile), { recursive: true });
		const result = spawnSync('bun', ['build', '--compile', `--target=${bunTarget}`, entry, '--outfile', outfile], {
			cwd: dir,
			stdio: 'inherit',
			shell: false
		});
		if (result.status !== 0) fail(`building for ${target} failed`);
		built.push(outfile);
	}
	return built;
}

export async function conformance(dir: string): Promise<string[]> {
	const host = FakeHost.spawn(['bun', 'run', join(dir, 'src', 'main.ts')], { cwd: dir, timeoutMs: 15000 });
	try {
		await host.start();
	} catch (error) {
		return [`the worker did not complete its handshake: ${(error as Error).message}`, host.diagnostics].filter(Boolean);
	}
	const code = await host.stop();
	const problems = [...host.violations];
	if (code !== 0 && code !== null) problems.push(`the worker exited with code ${code} after deactivating`);
	return problems;
}

function targetsFrom(args: string[]): Target[] {
	const chosen = args.flatMap((arg, i) => (arg === '--target' ? [args[i + 1]] : []));
	if (!chosen.length) return [hostTarget()];
	for (const t of chosen) if (!(TARGETS as readonly string[]).includes(t)) fail(`${t} is not a target; use one of ${TARGETS.join(', ')}`);
	return chosen as Target[];
}

export async function main(argv: string[]): Promise<number> {
	const [action, first, ...rest] = argv;
	const say = (line: string) => process.stdout.write(`${line}\n`);
	try {
		switch (action) {
			case 'create': {
				if (!first) fail('usage: ext create <id> [dir]');
				const publisherAt = rest.indexOf('--publisher');
				const dir = create(first, rest[0] && !rest[0].startsWith('--') ? rest[0] : undefined, publisherAt >= 0 ? rest[publisherAt + 1] : undefined);
				say(`Created ${dir}`);
				say('Next: bun install, bun test, then `bun run ext dev .` to try it in Spagitty.');
				return 0;
			}
			case 'dev': {
				const file = attachRequest(first ?? '.');
				say(`Asked Spagitty to attach ${resolve(first ?? '.')}.`);
				say(`It is picked up the next time Settings › Extensions is read (request: ${file}).`);
				return 0;
			}
			case 'validate': {
				const problems = validatePath(first ?? '.');
				for (const problem of problems) say(problem);
				const errors = problems.filter((p) => !p.startsWith('warning:'));
				say(errors.length ? `${errors.length} problem(s).` : 'Valid.');
				return errors.length ? 1 : 0;
			}
			case 'test': {
				const dir = resolve(first ?? '.');
				if (existsSync(join(dir, 'test'))) {
					const tests = spawnSync('bun', ['test'], { cwd: dir, stdio: 'inherit' });
					if (tests.status !== 0) return 1;
				}
				const problems = await conformance(dir);
				for (const problem of problems) say(problem);
				say(problems.length ? 'Not conformant.' : 'Conformant: handshake, activation, deactivation and exit.');
				return problems.length ? 1 : 0;
			}
			case 'build': {
				for (const out of build(resolve(first ?? '.'), targetsFrom(rest))) say(`Built ${out}`);
				return 0;
			}
			case 'pack': {
				const dir = resolve(first ?? '.');
				if (rest.includes('--build')) build(dir, targetsFrom(rest));
				const packed = pack(dir);
				const outAt = rest.indexOf('--out');
				const out = outAt >= 0 ? resolve(rest[outAt + 1]) : join(dir, 'dist', `${packed.manifest.id}-${packed.manifest.version}.spagitty-extension`);
				mkdirSync(dirname(out), { recursive: true });
				writeFileSync(out, packed.bytes);
				say(`Packed ${basename(out)} for ${packed.targets.join(', ')}`);
				if (packed.dropped.length) say(`Not built, so not listed: ${packed.dropped.join(', ')}`);
				return 0;
			}
			case 'inspect': {
				if (!first) fail('usage: ext inspect <file>');
				const report = inspect(new Uint8Array(readFileSync(first)));
				say(`${report.manifest.name} ${report.manifest.version} (${report.manifest.id})`);
				say(`Publisher (as stated, not verified): ${report.manifest.publisher}`);
				say(`Targets: ${Object.keys(report.manifest.runtime.entrypoints).join(', ')}`);
				const caps = report.manifest.capabilities ?? {};
				say(`Asks for: ${[...(caps.required ?? []), ...(caps.optional ?? []).map((c) => `${c} (optional)`)].join(', ') || 'nothing'}`);
				say(`SHA-256: ${report.digest}`);
				for (const file of report.files) say(`  ${file.path}  ${file.size} B${file.program ? '  program' : ''}`);
				for (const problem of report.problems) say(`problem: ${problem}`);
				return report.problems.length ? 1 : 0;
			}
			default:
				say('usage: ext <create|dev|validate|test|build|pack|inspect> …');
				return action ? 1 : 0;
		}
	} catch (error) {
		process.stderr.write(`${(error as Error).message}\n`);
		return 1;
	}
}

if (import.meta.main) process.exit(await main(process.argv.slice(2)));
