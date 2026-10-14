// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Fetch the terminal multiplexer Spagitty ships, before Tauri reads its
 * resources (FEAT-130).
 *
 * The Terminal screen runs the shell inside tmux — psmux on Windows — when
 * one can be run. On macOS and Linux a system tmux of 3.2 or later is used,
 * and this one is for the machines that have none; on Windows this is the one.
 *
 * Nothing is built here. Each target has one pinned release asset, published
 * by the project itself, and its SHA-256. A download whose digest is not the
 * one written below is deleted and the build fails: a different binary under
 * the same name is exactly what a checksum is for.
 *
 * - **tmux** (ISC) is the tmux project's own static build, `tmux/tmux-builds`,
 *   linked against libevent (BSD-3-Clause), ncurses (MIT-style), utf8proc
 *   (MIT) and, on Linux, musl (MIT). Its licence texts are shipped beside it.
 * - **psmux** (MIT) is `psmux/psmux`: tmux's commands on Windows' ConPTY.
 *
 * Both are compatible with distribution beside a GPL-3.0-or-later program,
 * and both are separate programs that Spagitty runs, not code it links.
 *
 * Raising a version is a change to this table and nothing else: the new
 * asset's digest is on its release page, and from `gh api
 * repos/<owner>/<repo>/releases/tags/<tag>`.
 */
import { createHash } from 'node:crypto';
import {
	chmodSync, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync
} from 'node:fs';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

export interface Asset {
	url: string;
	sha256: string;
	/** The files taken from the archive, as they are named in it. */
	take: string[];
}

export interface Pinned {
	name: 'tmux' | 'psmux';
	version: string;
	asset: Asset;
	/** An archive of licence texts shipped beside the binary, when the project publishes one. */
	licences?: Asset;
}

const TMUX = 'https://github.com/tmux/tmux-builds/releases/download/v3.7c';
const TMUX_LICENCES: Asset = {
	url: `${TMUX}/LICENSES.tar.gz`,
	sha256: '094905d3ba42397c65fecd817dc29736dd14bd35e75600441403a365125f47d1',
	take: ['COPYING.tmux', 'LICENSE.libevent', 'COPYING.ncurses', 'LICENSE.utf8proc', 'COPYRIGHT.musl']
};
const tmux = (file: string, sha256: string): Pinned => ({
	name: 'tmux',
	version: '3.7c',
	asset: { url: `${TMUX}/${file}`, sha256, take: ['tmux'] },
	licences: TMUX_LICENCES
});

const PSMUX = 'https://github.com/psmux/psmux/releases/download/v3.3.8';
const psmux = (file: string, sha256: string): Pinned => ({
	name: 'psmux',
	version: '3.3.8',
	// One program under three names. `psmux.exe` is the one Spagitty runs.
	asset: { url: `${PSMUX}/${file}`, sha256, take: ['psmux.exe', 'LICENSE'] }
});

/** What ships for each target Spagitty is released for. */
export const PINNED: Record<string, Pinned> = {
	'x86_64-unknown-linux-gnu': tmux(
		'tmux-3.7c-linux-x86_64.tar.gz',
		'cc56bd1cc873eb6089c615f0496b072385bda8a6d944069f38564a5d49c128aa'
	),
	'aarch64-unknown-linux-gnu': tmux(
		'tmux-3.7c-linux-arm64.tar.gz',
		'29dcb978da2a4b0cf6790f9e004865dac239a3ba03bbf776dddacce23d03831b'
	),
	'x86_64-apple-darwin': tmux(
		'tmux-3.7c-macos-x86_64.tar.gz',
		'1c21f9ade964e4a539be05ffe3a4e95854ab76a0a9c4c05d0ffbe7fc6c4d783f'
	),
	'aarch64-apple-darwin': tmux(
		'tmux-3.7c-macos-arm64.tar.gz',
		'0a763dd0380aa980d239509654da1bc7455843706a3c050f6709c8cd2e13d12d'
	),
	'x86_64-pc-windows-msvc': psmux(
		'psmux-v3.3.8-windows-x64.zip',
		'1ad127ba937194a890b933a73d9b023e297bd73dc742abd841bf159984c2effe'
	),
	'aarch64-pc-windows-msvc': psmux(
		'psmux-v3.3.8-windows-arm64.zip',
		'092cf01b4fcac55c9b80597af7c0196bcbd43de1dd4efb8f25ef1aee060176ab'
	)
};

/** The target triple this machine builds for, when Tauri does not say. */
export function targetFor(platform: string, arch: string): string {
	if (!['x64', 'arm64'].includes(arch)) throw new Error('Unsupported architecture: ' + arch);
	const cpu = arch === 'arm64' ? 'aarch64' : 'x86_64';
	if (platform === 'win32') return cpu + '-pc-windows-msvc';
	if (platform === 'darwin') return cpu + '-apple-darwin';
	if (platform === 'linux') return cpu + '-unknown-linux-gnu';
	throw new Error('Unsupported platform: ' + platform);
}

export function sha256(bytes: Uint8Array): string {
	return createHash('sha256').update(bytes).digest('hex');
}

/** Refuse bytes that are not the ones pinned. */
export function verify(bytes: Uint8Array, asset: Asset): void {
	const found = sha256(bytes);
	if (found !== asset.sha256) {
		throw new Error(
			`${asset.url} is not the file that was pinned.\n  expected ${asset.sha256}\n  found    ${found}`
		);
	}
}

/** What was staged, so a second run with the same pin downloads nothing. */
const STAMP = '.staged';

export function stampOf(target: string, pinned: Pinned): string {
	return [target, pinned.name, pinned.version, pinned.asset.sha256, pinned.licences?.sha256]
		.filter((part) => part !== undefined)
		.join(' ');
}

/** True when `dir` already holds exactly what `pinned` would put there. */
export function isStaged(dir: string, target: string, pinned: Pinned): boolean {
	const stamp = join(dir, STAMP);
	if (!existsSync(stamp) || readFileSync(stamp, 'utf8').trim() !== stampOf(target, pinned)) return false;
	return [...pinned.asset.take, ...(pinned.licences?.take ?? [])].every((file) => existsSync(join(dir, file)));
}

async function download(url: string): Promise<Uint8Array> {
	const response = await fetch(url, { redirect: 'follow' });
	if (!response.ok) throw new Error(`${url}: ${response.status} ${response.statusText}`);
	return new Uint8Array(await response.arrayBuffer());
}

/**
 * Unpack `archive` into `into`. `tar` reads both kinds on every platform
 * Spagitty builds on: GNU and BSD tar read `.tar.gz`, and Windows' own tar —
 * named by its full path, because a Git Bash earlier on the `PATH` has a tar
 * that does not — reads `.zip`.
 */
function unpack(archive: string, into: string): void {
	const tar =
		process.platform === 'win32'
			? join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'tar.exe')
			: 'tar';
	const done = spawnSync(tar, ['-xf', archive, '-C', into], { stdio: 'inherit' });
	if (done.status !== 0) throw new Error(`Could not unpack ${archive}`);
}

/** Download, check and unpack one asset, and copy what is taken from it into `dir`. */
async function stage(asset: Asset, dir: string, work: string): Promise<void> {
	const bytes = await download(asset.url);
	verify(bytes, asset);
	const archive = join(work, asset.url.slice(asset.url.lastIndexOf('/') + 1));
	const unpacked = join(work, sha256(new TextEncoder().encode(asset.url)).slice(0, 12));
	mkdirSync(unpacked, { recursive: true });
	writeFileSync(archive, bytes);
	unpack(archive, unpacked);
	for (const file of asset.take) {
		const from = join(unpacked, file);
		if (!existsSync(from)) throw new Error(`${asset.url} has no ${file} in it`);
		copyFileSync(from, join(dir, file));
	}
}

/** Put the multiplexer for `target` into `root/multiplexer/bundle`, and say what is there. */
export async function fetchMultiplexer(root: string, target: string): Promise<string> {
	const dir = resolve(root, 'multiplexer', 'bundle');
	mkdirSync(dir, { recursive: true });
	const pinned = PINNED[target];

	// Clear what another target left: one bundle holds one platform's binary.
	const clear = () => {
		for (const entry of readdirSync(dir)) {
			if (entry !== '.gitkeep') rmSync(join(dir, entry), { recursive: true, force: true });
		}
	};
	if (!pinned) {
		clear();
		return `No multiplexer is pinned for ${target}: the terminal uses the system's, or none.`;
	}
	if (isStaged(dir, target, pinned)) return `${pinned.name} ${pinned.version} is already staged for ${target}.`;

	clear();
	const work = resolve(root, 'multiplexer', '.download');
	rmSync(work, { recursive: true, force: true });
	mkdirSync(work, { recursive: true });
	try {
		await stage(pinned.asset, dir, work);
		if (pinned.licences) await stage(pinned.licences, dir, work);
		if (!target.includes('windows')) chmodSync(join(dir, pinned.asset.take[0]), 0o755);
		writeFileSync(join(dir, STAMP), stampOf(target, pinned) + '\n');
	} catch (error) {
		// Half a bundle is worse than none: the terminal would find a binary
		// without its licence, or a licence without its binary.
		clear();
		throw error;
	} finally {
		rmSync(work, { recursive: true, force: true });
	}
	return `Staged ${pinned.name} ${pinned.version} for ${target}.`;
}

if (import.meta.main) {
	const root = resolve(import.meta.dir, '..');
	const target =
		process.env.SPAGITTY_MULTIPLEXER_TARGET ??
		process.env.TAURI_ENV_TARGET_TRIPLE ??
		targetFor(process.platform, process.arch);
	try {
		console.log(await fetchMultiplexer(root, target));
	} catch (error) {
		// A developer with no network still gets a build: the terminal falls
		// back to the system's multiplexer, or to none. A release must not,
		// and says so with SPAGITTY_MULTIPLEXER_REQUIRED.
		if (process.env.SPAGITTY_MULTIPLEXER_REQUIRED) throw error;
		console.warn(`The multiplexer was not staged: ${error instanceof Error ? error.message : error}`);
	}
}
