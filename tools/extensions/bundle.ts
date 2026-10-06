// SPDX-License-Identifier: GPL-3.0-or-later
/** Build only the current target's official worker before Tauri reads resources. */
import {
	chmodSync, copyFileSync, existsSync, lstatSync, mkdirSync,
	readFileSync, rmSync, writeFileSync
} from 'node:fs';
import { isAbsolute, join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const TARGETS = [
	'x86_64-pc-windows-msvc',
	'x86_64-unknown-linux-gnu',
	'x86_64-apple-darwin',
	'aarch64-apple-darwin'
];

export function targetFor(platform: string, arch: string): string {
	if (!['x64', 'arm64'].includes(arch)) {
		throw new Error('Unsupported extension architecture: ' + arch);
	}
	const cpu = arch === 'arm64' ? 'aarch64' : 'x86_64';
	if (platform === 'win32' && arch === 'x64') return cpu + '-pc-windows-msvc';
	if (platform === 'darwin') return cpu + '-apple-darwin';
	if (platform === 'linux' && arch === 'x64') return cpu + '-unknown-linux-gnu';
	throw new Error('Unsupported extension release platform: ' + platform + '/' + arch);
}

export function bundle(root: string, target: string, debug = false): string {
	if (!TARGETS.includes(target)) throw new Error('Unsupported extension release target: ' + target);
	const args = ['build', '-p', 'coderabbit-extension', '--bin', 'coderabbit-extension', '--target', target];
	if (!debug) args.push('--release');
	// The side worker must also run on Windows machines without the VC redistributable.
	if (target.includes('windows')) {
		args.push('--config', 'target.x86_64-pc-windows-msvc.rustflags=["-C","target-feature=+crt-static"]');
	}
	const built = spawnSync('cargo', args, { cwd: root, stdio: 'inherit' });
	if (built.status !== 0) throw new Error('The official extension worker did not build.');

	const source = resolve(
		root, process.env.CARGO_TARGET_DIR ?? 'target', target,
		debug ? 'debug' : 'release',
		target.includes('windows') ? 'coderabbit-extension.exe' : 'coderabbit-extension'
	);
	const staging = resolve(root, 'extensions', 'coderabbit', 'bundle');
	const dir = join(staging, 'spagitty.coderabbit');

	// Clear only our generated package after checking its absolute location.
	const within = relative(staging, resolve(dir));
	if (!within || within.startsWith('..') || isAbsolute(within)) {
		throw new Error('Unsafe bundle destination.');
	}
	for (const path of [staging, dir]) {
		if (existsSync(path) && lstatSync(path).isSymbolicLink()) {
			throw new Error('The bundle staging directories must not be links.');
		}
	}
	if (existsSync(dir)) rmSync(dir, { recursive: true });

	const manifest = JSON.parse(readFileSync(join(root, 'extensions', 'coderabbit', 'extension.json'), 'utf8'));
	const entrypoint = manifest.runtime.entrypoints[target];
	if (!entrypoint) throw new Error('The manifest does not declare this target.');

	// A macOS sidecar is signed after Tauri imports the app's signing keychain.
	const destination = target.includes('apple-darwin')
		? join(root, 'extensions', 'coderabbit', 'sidecar', 'coderabbit-extension-' + target)
		: join(dir, entrypoint);
	mkdirSync(resolve(destination, '..'), { recursive: true });
	copyFileSync(source, destination);
	if (!target.includes('windows')) chmodSync(destination, 0o755);

	manifest.runtime.entrypoints = { [target]: entrypoint };
	mkdirSync(dir, { recursive: true });
	writeFileSync(join(dir, 'extension.json'), JSON.stringify(manifest, null, 2) + '\n');
	copyFileSync(join(root, 'LICENSE'), join(dir, 'LICENSE'));
	copyFileSync(join(root, 'extensions', 'coderabbit', 'README.md'), join(dir, 'README.md'));
	return dir;
}

if (import.meta.main) {
	const root = resolve(import.meta.dir, '../..');
	const target = process.env.SPAGITTY_EXTENSION_TARGET
		?? process.env.TAURI_ENV_TARGET_TRIPLE
		?? targetFor(process.platform, process.arch);
	console.log('Bundled official extension: ' + bundle(root, target, process.argv.includes('--debug')));
}
