// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * The multiplexer Spagitty ships is pinned (FEAT-130): one asset for each
 * target, from the project that makes it, with the digest it must have.
 */
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { isStaged, PINNED, sha256, stampOf, targetFor, verify } from './multiplexer';

const made: string[] = [];
function aDir(): string {
	const dir = mkdtempSync(join(tmpdir(), 'spagitty-mux-'));
	made.push(dir);
	return dir;
}
afterEach(() => {
	for (const dir of made.splice(0)) rmSync(dir, { recursive: true, force: true });
});

describe('what is pinned', () => {
	it('covers every target Spagitty is released for', () => {
		// The four the official extension is built for, which are the release's.
		for (const target of [
			'x86_64-pc-windows-msvc',
			'x86_64-unknown-linux-gnu',
			'x86_64-apple-darwin',
			'aarch64-apple-darwin'
		]) {
			expect(PINNED[target], target).toBeDefined();
		}
	});

	it('is psmux on Windows and tmux everywhere else', () => {
		for (const [target, pinned] of Object.entries(PINNED)) {
			expect(pinned.name, target).toBe(target.includes('windows') ? 'psmux' : 'tmux');
		}
	});

	/** A binary is run on the person's machine. Where it comes from is not left open. */
	it('comes only from the project that makes it, at a tag, over https', () => {
		for (const [target, pinned] of Object.entries(PINNED)) {
			const from =
				pinned.name === 'tmux'
					? 'https://github.com/tmux/tmux-builds/releases/download/'
					: 'https://github.com/psmux/psmux/releases/download/';
			for (const asset of [pinned.asset, pinned.licences].filter((a) => a !== undefined)) {
				expect(asset.url.startsWith(from), `${target}: ${asset.url}`).toBe(true);
				expect(asset.url).toContain(pinned.version);
				expect(asset.url).not.toContain('latest');
			}
		}
	});

	it('has a whole SHA-256 for every file it downloads, and no two the same by accident', () => {
		const digests = new Map<string, string>();
		for (const pinned of Object.values(PINNED)) {
			for (const asset of [pinned.asset, pinned.licences].filter((a) => a !== undefined)) {
				expect(asset.sha256).toMatch(/^[0-9a-f]{64}$/);
				// One digest, one file.
				expect(digests.get(asset.sha256) ?? asset.url).toBe(asset.url);
				digests.set(asset.sha256, asset.url);
			}
		}
	});

	it('takes the binary the terminal looks for, and its licence with it', () => {
		for (const [target, pinned] of Object.entries(PINNED)) {
			expect(pinned.asset.take[0], target).toBe(pinned.name === 'tmux' ? 'tmux' : 'psmux.exe');
			const licences = [...pinned.asset.take, ...(pinned.licences?.take ?? [])].filter((file) =>
				/LICEN[CS]E|COPYING|COPYRIGHT/.test(file)
			);
			expect(licences.length, `${target} ships no licence`).toBeGreaterThan(0);
		}
		// tmux is linked against these, and each one's terms go with it.
		expect(PINNED['x86_64-unknown-linux-gnu'].licences?.take).toEqual(
			expect.arrayContaining(['COPYING.tmux', 'LICENSE.libevent', 'COPYING.ncurses', 'LICENSE.utf8proc'])
		);
	});

	it('is written into NOTICE, by name and version', () => {
		const notice = readFileSync('NOTICE', 'utf8');
		for (const pinned of Object.values(PINNED)) {
			expect(notice).toContain(`${pinned.name} ${pinned.version}`);
		}
		for (const linked of ['libevent', 'ncurses', 'utf8proc']) expect(notice).toContain(linked);
	});
});

describe('the target', () => {
	it('is named the way Tauri names it', () => {
		expect(targetFor('win32', 'x64')).toBe('x86_64-pc-windows-msvc');
		expect(targetFor('win32', 'arm64')).toBe('aarch64-pc-windows-msvc');
		expect(targetFor('linux', 'x64')).toBe('x86_64-unknown-linux-gnu');
		expect(targetFor('darwin', 'arm64')).toBe('aarch64-apple-darwin');
		expect(targetFor('darwin', 'x64')).toBe('x86_64-apple-darwin');
	});

	it('is refused when it is not one Spagitty builds for', () => {
		expect(() => targetFor('freebsd', 'x64')).toThrow();
		expect(() => targetFor('linux', 'ia32')).toThrow();
	});
});

describe('the checksum', () => {
	const bytes = new TextEncoder().encode('not a multiplexer');
	const asset = { url: 'https://example.com/tmux.tar.gz', sha256: sha256(bytes), take: ['tmux'] };

	it('passes the file that was pinned', () => {
		expect(() => verify(bytes, asset)).not.toThrow();
	});

	/** A different binary under the same name is what a checksum is for. */
	it('refuses any other, naming both digests', () => {
		const other = new TextEncoder().encode('not a multiplexer!');

		expect(() => verify(other, asset)).toThrow(/is not the file that was pinned/);
		expect(() => verify(other, asset)).toThrow(asset.sha256);
		expect(() => verify(other, asset)).toThrow(sha256(other));
		expect(() => verify(new Uint8Array(), asset)).toThrow();
	});
});

describe('what is already staged', () => {
	const target = 'x86_64-unknown-linux-gnu';
	const pinned = PINNED[target];

	function staged(): string {
		const dir = aDir();
		for (const file of [...pinned.asset.take, ...(pinned.licences?.take ?? [])]) {
			writeFileSync(join(dir, file), 'x');
		}
		writeFileSync(join(dir, '.staged'), stampOf(target, pinned) + '\n');
		return dir;
	}

	it('is not downloaded again', () => {
		expect(isStaged(staged(), target, pinned)).toBe(true);
	});

	it('is downloaded again for another target, another pin, or a missing file', () => {
		const dir = staged();

		expect(isStaged(dir, 'aarch64-apple-darwin', PINNED['aarch64-apple-darwin'])).toBe(false);
		expect(isStaged(dir, target, { ...pinned, version: '9.9' })).toBe(false);
		expect(isStaged(dir, target, { ...pinned, asset: { ...pinned.asset, sha256: '0'.repeat(64) } })).toBe(false);

		rmSync(join(dir, 'tmux'));
		expect(isStaged(dir, target, pinned)).toBe(false);
	});

	it('is nothing in a folder that was never staged', () => {
		const dir = aDir();
		mkdirSync(join(dir, 'empty'));

		expect(isStaged(join(dir, 'empty'), target, pinned)).toBe(false);
	});
});
