// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The macOS release policy, checked where it is actually decided (TASK-040).
 *
 * A downloaded Spagitty told a Mac user their copy was **damaged**. That is not
 * the unidentified-developer dialog, and the difference is a fact about the
 * artefact rather than about the machine reading it: an app with no code
 * signature at all, or one whose seal no longer matches its own bytes, is
 * reported as damaged and offered a Move to Bin button, where an app with a
 * valid signature and an unknown developer gets a documented Open Anyway path.
 *
 * Nothing in this repository signed the bundle, nothing opened the finished
 * `.dmg`, one Intel lane was pinned to a runner GitHub has retired, and the
 * release notes told everybody to strip the quarantine attribute — which
 * silences the check rather than repairing the file.
 *
 * None of that is code, so none of it could fail a test. These assertions are
 * joins between the workflow files, the two composite actions and the Tauri
 * configuration — read as data, the way `record.test.ts` reads `agile/` and
 * `updates.test.ts` reads the AppImage lanes. **Nothing here is a stand-in for
 * a real macOS run**; `agile/testing/TASK-040-sweep.md` carries what only a Mac
 * can answer.
 */

import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const SIGNING = '.github/actions/macos-signing/action.yml';
const VERIFY = '.github/actions/macos-verify/action.yml';
const COLLECT = '.github/actions/release-assets/action.yml';
const CONFIG = 'src-tauri/tauri.conf.json';

/** Every lane that builds a macOS bundle somebody could end up running. */
const LANES = [
	'.github/workflows/gates.yml',
	'.github/workflows/prerelease.yml',
	'.github/workflows/draft-release.yml'
];

const read = (path: string) => readFileSync(path, 'utf8');

/**
 * Runner labels a lane names, gathered from `os:` entries.
 *
 * Read as plain text rather than through a YAML parser deliberately: adding a
 * parser to check four strings would be a dependency justified by one test, and
 * the shape being matched is one this repository writes by hand.
 */
function runners(text: string): string[] {
	return [...text.matchAll(/^\s*os: (\S+)$/gm)].map((match) => match[1]);
}

describe('both Mac architectures are built, on runners that exist', () => {
	/**
	 * `macos-13`'s hosted image was closed down. A lane pinned to it does not
	 * produce a stale download — it produces no download, and it does so long
	 * after somebody decided the release was cut.
	 */
	it.each(LANES)('%s names no retired runner image', (lane) => {
		expect(runners(read(lane))).not.toContain('macos-13');
		expect(runners(read(lane))).not.toContain('macos-12');
	});

	/**
	 * The defect this catches is an absence, which is why it went unnoticed for
	 * every release: `macos-latest` is Apple silicon, so a lane with one macOS
	 * job silently ships an arm64-only release and nothing reports a problem.
	 * An Intel Mac simply has nothing to download.
	 */
	it.each(LANES)('%s builds Intel as well as Apple silicon', (lane) => {
		const text = read(lane);
		expect(text, 'no x86_64 Darwin target').toContain('x86_64-apple-darwin');
		expect(text, 'no aarch64 Darwin target').toContain('aarch64-apple-darwin');
	});

	/**
	 * Two runners, one product name, one version — so two identically named
	 * `.dmg` files. A GitHub release asset is keyed by its basename, and
	 * discovering that from a 422 after the tag has been pushed is how v0.2.0
	 * became unreleasable.
	 */
	it.each(LANES)('%s keeps the two macOS downloads apart by name', (lane) => {
		expect(read(lane)).toMatch(/suffix: \$\{\{ runner\.os == 'macOS'/);
	});
});

describe('every lane decides a signing policy before it builds', () => {
	it.each(LANES)('%s uses the shared policy action', (lane) => {
		expect(read(lane)).toContain('./.github/actions/macos-signing');
	});

	it.each(LANES)('%s verifies the artefact before uploading it', (lane) => {
		const text = read(lane);
		expect(text).toContain('./.github/actions/macos-verify');

		// Order is the whole point: a check that runs after the upload has
		// already handed the file to somebody.
		expect(
			text.indexOf('./.github/actions/macos-verify'),
			'verification must precede collection, or a broken bundle is already an asset'
		).toBeLessThan(text.indexOf('./.github/actions/release-assets'));
	});

	/**
	 * The lane that publishes what people are pointed at is the one that may
	 * not fall back. `interim` exists so a draft can still be cut without an
	 * Apple account; a release that quietly took the same path would be an
	 * unsigned build with notes claiming it was signed.
	 */
	it('gate 5 declares itself the production lane', () => {
		expect(read('.github/workflows/gates.yml')).toMatch(/lane: production/);
	});

	it.each(['.github/workflows/prerelease.yml', '.github/workflows/draft-release.yml'])(
		'%s declares itself an interim lane',
		(lane) => {
			expect(read(lane)).toMatch(/lane: interim/);
			expect(read(lane), 'a draft or alpha may not claim the production policy').not.toMatch(
				/lane: production/
			);
		}
	);

	/** Without this the refusal is decoration: the job would simply build. */
	it('the policy action refuses a production lane with no certificate', () => {
		const text = read(SIGNING);
		const refusal = text.slice(text.indexOf('# No certificate.'));

		expect(refusal).toMatch(/LANE.*=.*production/s);
		expect(refusal).toMatch(/exit 1/);
	});

	/** `-` is codesign's own spelling, and the only thing that gives an app a seal
	 *  when no identity is available. */
	it('an interim lane with no certificate still signs ad-hoc', () => {
		expect(read(SIGNING)).toContain('APPLE_SIGNING_IDENTITY=-');
	});
});

describe('the identity is a property of the lane, not of the configuration', () => {
	/**
	 * The single most tempting shortcut here, and the one that reintroduces the
	 * defect: `"signingIdentity": "-"` in `tauri.conf.json` ad-hoc signs *every*
	 * lane, production included. Such a build bundles, verifies, and is refused
	 * by Gatekeeper on every machine but the one that built it.
	 */
	it('no signing identity is written into tauri.conf.json', () => {
		const config = JSON.parse(read(CONFIG));
		expect(config.bundle.macOS?.signingIdentity).toBeUndefined();
		expect(config.bundle.macOS?.providerShortName).toBeUndefined();
	});

	/** Stated, so that "wrong minimum OS" stops being an unknown a diagnosis has
	 *  to rule out by asking the user what they are running. */
	it('states the minimum macOS it claims to run on', () => {
		expect(JSON.parse(read(CONFIG)).bundle.macOS.minimumSystemVersion).toBeTruthy();
	});
});

describe('the artefact is opened and asked three different questions', () => {
	const verify = read(VERIFY);

	it.each([
		['hdiutil verify', 'the container'],
		['codesign --verify', 'the seal over the bytes'],
		['spctl --assess', "the machine's policy verdict"]
	])('runs %s — %s', (tool) => {
		expect(verify).toContain(tool);
	});

	/**
	 * A build that took the host's architecture instead of its `--target` is a
	 * download named for one architecture and built for the other. It fails on
	 * a user's Mac with a message that looks like all the others, which is
	 * exactly why it has to be caught where the two facts are both available.
	 */
	it('confirms the architecture the filename claims', () => {
		expect(verify).toMatch(/file "\$binary"/);
		expect(verify).toMatch(/WANT_ARCH/);
	});

	/**
	 * An ad-hoc signature is *supposed* to be refused by Gatekeeper. A lane
	 * that failed on that could never be green, and one that failed on nothing
	 * would let a notarized build ship rejected.
	 */
	it('fails on a Gatekeeper rejection only where the lane notarized', () => {
		const assess = verify.slice(verify.indexOf('spctl --assess'));
		expect(assess).toMatch(/NOTARIZED.*=.*"true"/s);
		expect(assess).toMatch(/expected for a build that is not notarized/);
	});

	/**
	 * `--deep` reads nested code and is useful for verification. The matching
	 * *signing* flag produces the state macOS calls damaged — an outer seal
	 * over resources changed after they were sealed — so it must never appear
	 * as a repair.
	 */
	it('never re-signs anything', () => {
		expect(verify).not.toMatch(/codesign\s+(?!-d|--verify)[^\n]*--force/);
		expect(verify).not.toMatch(/--deep --force/);
	});
});

describe('what a person is told', () => {
	const draft = read('.github/workflows/draft-release.yml');
	const notes = draft.slice(draft.indexOf('--notes "'));

	/**
	 * The promise this replaces. Quarantine removal silences the check that
	 * would have caught a genuinely broken download; as *the* installation step
	 * it taught every Mac user to disarm it, and it cannot be an acceptance
	 * criterion for anything.
	 */
	it('no lane offers quarantine stripping as the way to install', () => {
		for (const lane of LANES) {
			expect(read(lane), `${lane} still prescribes xattr`).not.toMatch(
				/xattr -d com\.apple\.quarantine [^\n]*\n[^\n]*\bor\b/
			);
		}
		expect(notes).not.toMatch(/or run .xattr/);
	});

	it('says which dialog to expect, and that damaged is a different one', () => {
		expect(notes).toMatch(/unidentified developer/i);
		expect(notes).toMatch(/damaged/i);
		expect(notes).toMatch(/Open Anyway/);
	});

	/** Ad-hoc is not notarization, and a release note that implied otherwise
	 *  would be the same overclaim in a new place. */
	it('does not claim the build is notarized', () => {
		expect(notes).toMatch(/not notarized/i);
	});

	/** The first question any diagnosis of a damaged-app dialog has to answer. */
	it('publishes checksums for what it publishes', () => {
		expect(read(COLLECT)).toMatch(/SHA256SUMS/);
		expect(notes).toMatch(/SHA256SUMS/);
	});
});
