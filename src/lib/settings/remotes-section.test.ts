// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Remotes section (FEAT-049).
 *
 * A file of its own rather than a block in `sections.test.ts`: this is the one
 * section that is about the *open repository* instead of about Spagitty, so it
 * needs the repository double the other sections have no use for.
 *
 * What is asserted is mostly what the section says when there is nothing to
 * show. A repository with no remotes and a repository that is not open look the
 * same if the screen only renders a list, and they are entirely different
 * problems.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from '../../testing/mount';
import type { Remote } from '$lib/types';

vi.mock('$lib/api', () => ({
	inTauri: () => true,
	remotes: vi.fn(),
	remoteAdd: vi.fn(() => Promise.resolve()),
	remoteRename: vi.fn(() => Promise.resolve()),
	remoteRemove: vi.fn(() => Promise.resolve()),
	remoteSetUrl: vi.fn(() => Promise.resolve())
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import * as api from '$lib/api';
import { control as repoControl } from '../../testing/repo-store.svelte';
import { remotes } from '$lib/remotes/store.svelte';
import RemotesSection from './RemotesSection.svelte';

const list = vi.mocked(api.remotes);

function remote(name: string, overrides: Partial<Remote> = {}): Remote {
	return {
		name,
		url: `https://example.com/${name}.git`,
		pushUrl: null,
		host: 'generic',
		refs: 0,
		...overrides
	};
}

/** The section loads on mount; give that read a turn to land. */
async function show() {
	const view = render(RemotesSection, {});
	await Promise.resolve();
	await Promise.resolve();
	return view;
}

function openRepository() {
	repoControl.setInfo({
		path: '/repos/fixture',
		name: 'fixture',
		bare: false,
		head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' },
		lastFetched: null
	});
}

beforeEach(() => {
	vi.clearAllMocks();
	remotes.clear();
	repoControl.reset();
	list.mockResolvedValue([]);
});

describe('when there is nothing to show', () => {
	it('says remotes belong to a repository when none is open', async () => {
		const view = await show();

		expect(view.text()).toContain('No repository is open');
		// And offers no form: there is nothing for an added remote to belong to.
		expect(view.all('input')).toHaveLength(0);

		view.destroy();
	});

	it('says a repository has none, and still offers the form', async () => {
		// The other empty state, and the opposite response: this one is fixable
		// right here.
		openRepository();
		const view = await show();

		expect(view.text()).toContain('No remotes');
		expect(view.all('input')).toHaveLength(2);

		view.destroy();
	});
});

describe('the list', () => {
	it('shows each remote with its URL and its forge', async () => {
		openRepository();
		list.mockResolvedValue([
			remote('origin', { host: 'gitHub', refs: 4 }),
			remote('backup', { host: 'gitLab' })
		]);

		const view = await show();

		expect(view.all('.remote')).toHaveLength(2);
		expect(view.text()).toContain('origin');
		expect(view.text()).toContain('GitHub');
		expect(view.text()).toContain('https://example.com/origin.git');

		view.destroy();
	});

	it('says a remote has never been fetched rather than showing a zero', async () => {
		// The number that tells a remote somebody just added from one whose URL
		// stopped working months ago.
		openRepository();
		list.mockResolvedValue([remote('origin', { refs: 0 })]);

		const view = await show();

		expect(view.text()).toContain('never fetched');
		expect(view.text()).not.toContain('0 refs');

		view.destroy();
	});

	it('counts refs when there are some', async () => {
		openRepository();
		list.mockResolvedValue([remote('origin', { refs: 1 }), remote('backup', { refs: 7 })]);

		const view = await show();

		expect(view.text()).toContain('1 ref');
		expect(view.text()).toContain('7 refs');

		view.destroy();
	});

	it('shows a push URL only when one is configured', async () => {
		openRepository();
		list.mockResolvedValue([
			remote('origin'),
			remote('backup', { pushUrl: 'git@example.com:repo.git' })
		]);

		const view = await show();

		expect(view.text()).toContain('pushes to git@example.com:repo.git');
		expect(view.all('.url').filter((u) => u.textContent?.includes('pushes to'))).toHaveLength(1);

		view.destroy();
	});

	it('offers rename, change URL and remove on each one', async () => {
		openRepository();
		list.mockResolvedValue([remote('origin')]);

		const view = await show();

		const chips = view.all('.chip').map((c) => c.textContent?.trim());
		expect(chips).toEqual(['rename', 'change URL', 'remove']);
		// Only the destructive one is painted as destructive.
		expect(view.all('.chip.danger')).toHaveLength(1);

		view.destroy();
	});
});
