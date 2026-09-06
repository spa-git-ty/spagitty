// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The two ways a Spagitty learns there is a newer one, checked where they are
 * actually decided (TASK-037).
 *
 * Both live in the build lanes rather than in the application, and that is why
 * both were broken at once with every test passing:
 *
 * - **The tag baked into the binary.** `SPAGITTY_RELEASE` is the only thing
 *   that tells a running build which release it is, and only the draft lane
 *   set it. Gate 5 and the prerelease lane — the two that build what people
 *   download — did not, so every release reported itself as a development
 *   build and could never be behind anything. `crates/spagitty-core/src/update.rs`
 *   was correct throughout; there was nothing there to catch.
 * - **The update source embedded in the AppImage.** It ends with a glob, and an
 *   updater matches that glob against a release's asset names. A wrong glob is
 *   invisible at build time: the AppImage builds, runs, and reports "no
 *   suitable release" on a stranger's machine months later.
 *
 * So the assertions here are joins between files that no compiler or gate reads
 * together — the glob against the name Tauri composes, the workflows against
 * the variable they have to export, and the AppImage's update source against
 * the endpoint the in-app check reads. They read the real workflow and
 * configuration files as data, the way `record.test.ts` reads `agile/`.
 * Nothing here is a stand-in for CI.
 */

import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const ACTION = '.github/actions/appimage-update-info/action.yml';
const COLLECT = '.github/actions/release-assets/action.yml';
const CONFIG = 'src-tauri/tauri.conf.json';
const UPDATE_CHECK = 'crates/spagitty-core/src/update.rs';

/** Every lane that builds a bundle somebody could end up running. */
const LANES = [
	'.github/workflows/gates.yml',
	'.github/workflows/prerelease.yml',
	'.github/workflows/draft-release.yml'
];

const read = (path: string) => readFileSync(path, 'utf8');

/**
 * The one definition, pulled out of the composite action that exports it.
 * Asserting the match rather than defaulting: if the action is rewritten so
 * this finds nothing, that is a failure and not a quietly skipped test.
 */
function updateInformation(): string {
	const found = read(ACTION).match(/^ *information='([^']+)'$/m);
	expect(found, `${ACTION} no longer assigns information='…'`).not.toBeNull();
	return found![1];
}

/**
 * The step that builds the bundle, from its `run:` line to the next step.
 *
 * Two lanes write `bun run tauri build`; the draft lane writes
 * `bun run tauri -- build`, where the bare `--` stops bun handing the flags to
 * cargo. Steps in these files sit at six spaces, which is where one ends.
 */
function buildStep(text: string): string {
	const lines = text.split('\n');
	// Anchored on `run:` so a comment that quotes the command — the draft lane
	// has one, explaining where the `--` goes — is not mistaken for the step.
	const start = lines.findIndex((line) => /^\s*(- )?run: .*tauri (-- )?build/.test(line));
	expect(start, 'no step in this lane builds a bundle').toBeGreaterThan(-1);

	const rest = lines.slice(start + 1);
	const end = rest.findIndex((line) => /^ {6}- /.test(line));
	return lines.slice(start, end === -1 ? undefined : start + 1 + end).join('\n');
}

/**
 * The filename Tauri's AppImage bundler will write, composed the way
 * `tauri-bundler`'s `linux/appimage/linuxdeploy.rs` composes it:
 * `{productName}_{version}_{arch}.AppImage`, where x86-64 is spelled `amd64`.
 */
function bundleName(): string {
	const config = JSON.parse(read(CONFIG));
	return `${config.productName}_${config.version}_amd64.AppImage`;
}

/** An fnmatch glob of the only kind used here — literal text and `*`. */
function matches(glob: string, name: string): boolean {
	const pattern = glob.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replaceAll('*', '.*');
	return new RegExp(`^${pattern}$`).test(name);
}

describe('every lane that builds a bundle', () => {
	it.each(LANES)('%s stamps the release tag into it', (lane) => {
		// The defect this file was written for. A lane that builds without
		// `SPAGITTY_RELEASE` produces a binary that cannot tell what it is, and
		// nothing downstream — not a gate, not a test, not the release itself —
		// looks any different.
		expect(buildStep(read(lane))).toContain('SPAGITTY_RELEASE:');
	});

	it.each(LANES)('%s exports the AppImage update information', (lane) => {
		expect(read(lane)).toContain('./.github/actions/appimage-update-info');
	});

	it('collects the zsync alongside the AppImage', () => {
		// The AppImage's update source is a `.zsync` asset on the release. An
		// AppImage attached without it advertises a URL that answers 404.
		expect(read(COLLECT)).toContain("-name '*.AppImage.zsync'");
	});
});

describe('the AppImage update information', () => {
	it('is a five-field gh-releases-zsync source', () => {
		const fields = updateInformation().split('|');

		expect(fields).toHaveLength(5);
		expect(fields[0]).toBe('gh-releases-zsync');
		expect(fields[3]).toBe('latest');
	});

	it('names the project the in-app update check already reads', () => {
		// Two ways to hear about a newer Spagitty, and they have to mean the
		// same repository — one of them authorises replacing the binary.
		const [, owner, repo] = updateInformation().split('|');

		expect(read(UPDATE_CHECK)).toContain(
			`https://api.github.com/repos/${owner}/${repo}/releases/latest`
		);
	});

	it('globs for the file the bundler actually writes', () => {
		// The assertion the whole file exists for. `amd64` rather than the
		// `x86_64` other projects use is not a preference: it is what Tauri
		// names the bundle, and a glob that disagrees finds no asset.
		const glob = updateInformation().split('|')[4];

		expect(matches(glob, `${bundleName()}.zsync`)).toBe(true);
	});

	it('does not glob the AppImage itself', () => {
		// An updater fetches whatever the glob matches and reads it as a zsync
		// control file. Matching the bundle instead fails in a way that reads
		// like a corrupt download.
		const glob = updateInformation().split('|')[4];

		expect(matches(glob, bundleName())).toBe(false);
	});

	it("does not glob another platform's download", () => {
		const glob = updateInformation().split('|')[4];
		const { productName, version } = JSON.parse(read(CONFIG));

		for (const other of [
			`${productName}_${version}_x64-setup.exe`,
			`${productName}_${version}_aarch64.dmg`,
			`${productName}_${version}_amd64.deb`
		]) {
			expect(matches(glob, other), other).toBe(false);
		}
	});
});
