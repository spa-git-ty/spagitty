// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Signing section (FEAT-019, covered under FEAT-072).
 *
 * The section's claim is that it says whether a signature would *actually*
 * happen, not only whether one was asked for — so the tests that matter are
 * the ones where `commit.gpgsign` is true and the answer is still no. Each of
 * those is a different sentence, and each is a different branch.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from '../../testing/mount';
import type { Signing } from '$lib/types';

const state = {
	signing: null as Signing | null,
	scope: 'global' as 'global' | 'local',
	busy: false,
	setSigning: vi.fn(),
	clearSigning: vi.fn()
};

vi.mock('./store.svelte', () => ({
	settings: {
		get signing() {
			return state.signing;
		},
		get scope() {
			return state.scope;
		},
		get busy() {
			return state.busy;
		},
		setSigning: (...args: unknown[]) => state.setSigning(...args),
		clearSigning: (...args: unknown[]) => state.clearSigning(...args)
	}
}));

import SigningSection from './SigningSection.svelte';

function aSigning(overrides: Partial<Signing> = {}): Signing {
	return {
		enabled: true,
		origin: 'global',
		format: 'openPgp',
		key: 'ABCD1234',
		program: 'gpg',
		problem: null,
		repository: true,
		global: true,
		local: null,
		...overrides
	};
}

beforeEach(() => {
	vi.clearAllMocks();
	state.signing = aSigning();
	state.scope = 'global';
	state.busy = false;
});

describe('before the configuration has been read', () => {
	it('says it is reading rather than showing signing as off', () => {
		// "off" would be a claim, and the wrong one.
		state.signing = null;
		const view = render(SigningSection, {});

		expect(view.text()).toContain('Reading the git configuration');
		expect(view.text()).not.toContain('Sign my commits');

		view.destroy();
	});
});

describe('what a commit made now would do', () => {
	it('shows the switch as on, and says so in git’s own words', () => {
		const view = render(SigningSection, {});

		expect(view.text()).toContain('commit.gpgsign = true');
		expect(view.text()).toContain('on');

		view.destroy();
	});

	it('shows the switch as off', () => {
		state.signing = aSigning({ enabled: false, global: false });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('commit.gpgsign = false');

		view.destroy();
	});

	it('names the key that would sign, and where it came from', () => {
		const view = render(SigningSection, {});

		expect(view.text()).toContain('ABCD1234');
		expect(view.text()).toContain('user.signingkey');

		view.destroy();
	});

	it('explains what GPG does with no key rather than only noting the absence', () => {
		state.signing = aSigning({ key: null, format: 'openPgp' });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('No user.signingkey');
		expect(view.text()).toContain('committer address');

		view.destroy();
	});

	it('does not offer the GPG explanation for a format it is not true of', () => {
		// ssh signing has no fallback to the committer address; saying it does
		// would be advice that does not work.
		state.signing = aSigning({ key: null, format: 'ssh' });
		const view = render(SigningSection, {});

		expect(view.text()).not.toContain('committer address');

		view.destroy();
	});
});

describe('signing that is on and will not work', () => {
	it('says so when the signing program is missing', () => {
		state.signing = aSigning({ problem: { kind: 'missingProgram', detail: 'gpg' } });
		const view = render(SigningSection, {});

		expect(view.all('.warn').length).toBeGreaterThan(0);

		view.destroy();
	});

	it('says so when the format needs a key and there is none', () => {
		state.signing = aSigning({ format: 'ssh', key: null, problem: { kind: 'noSigningKey' } });
		const view = render(SigningSection, {});

		expect(view.all('.warn').length).toBeGreaterThan(0);

		view.destroy();
	});
});

describe('editing a file that is not the one deciding', () => {
	it('warns when the repository overrides the global file being edited', () => {
		state.scope = 'global';
		state.signing = aSigning({ origin: 'local', local: false, global: true });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('This repository sets its own');

		view.destroy();
	});

	it('warns when something outside both files is winning', () => {
		state.signing = aSigning({ origin: 'environment' });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('Something outside both of these files');

		view.destroy();
	});

	it('warns for a system-level setting for the same reason', () => {
		state.signing = aSigning({ origin: 'system' });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('Something outside both of these files');

		view.destroy();
	});

	it('says nothing when the file being edited is the one deciding', () => {
		state.scope = 'global';
		state.signing = aSigning({ origin: 'global' });
		const view = render(SigningSection, {});

		expect(view.text()).not.toContain('will not take effect');
		expect(view.text()).not.toContain('This repository sets its own');

		view.destroy();
	});
});

describe('what the chosen scope actually holds', () => {
	it('names the global file and what is in it', () => {
		state.scope = 'global';
		state.signing = aSigning({ global: true });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('Global holds');

		view.destroy();
	});

	it('names the repository when that is the scope', () => {
		state.scope = 'local';
		state.signing = aSigning({ local: true });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('This repository holds');

		view.destroy();
	});

	it('says a scope that holds nothing holds nothing', () => {
		// The "In effect" line above already names where the live value came
		// from, so this line only has to say that it did not come from here.
		state.scope = 'local';
		state.signing = aSigning({ local: null });
		const view = render(SigningSection, {});

		expect(view.text()).toContain('This repository holds nothing');

		view.destroy();
	});

	it('cannot clear a file that has nothing in it to clear', () => {
		state.scope = 'local';
		state.signing = aSigning({ local: null });
		const view = render(SigningSection, {});

		const clear = view.all('button').find((b) => b.textContent?.includes('Clear'));
		expect(clear?.hasAttribute('disabled')).toBe(true);

		view.destroy();
	});

	it('can clear a file that holds a value', () => {
		state.scope = 'local';
		state.signing = aSigning({ local: false });
		const view = render(SigningSection, {});

		const clear = view.all('button').find((b) => b.textContent?.includes('Clear'));
		expect(clear?.hasAttribute('disabled')).toBe(false);

		view.destroy();
	});

	it('disables everything while a write is in flight', () => {
		state.busy = true;
		state.signing = aSigning({ local: false });
		state.scope = 'local';
		const view = render(SigningSection, {});

		expect(view.all('button').every((b) => b.hasAttribute('disabled'))).toBe(true);

		view.destroy();
	});
});
