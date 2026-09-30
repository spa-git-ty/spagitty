// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import type { ChangedFile, CommitDetail as Detail, GraphRow } from '$lib/types';

vi.mock('$lib/graph/store.svelte', async () => await import('../../testing/graph-store.svelte'));

const goto = vi.fn();
vi.mock('$app/navigation', () => ({ goto: (path: string) => goto(path) }));

vi.mock('$lib/repo.svelte', () => ({
	repo: {
		get info() {
			return {
				path: '/repos/fixture',
				name: 'fixture',
				bare: false,
				lastFetched: null,
				head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
			};
		}
	}
}));

import { control } from '../../testing/graph-store.svelte';
import CommitDetail from './CommitDetail.svelte';

function graphRow(): GraphRow {
	return {
		index: 0,
		id: 'b'.repeat(40),
		short: 'bbbbbbb',
		summary: 'Rewrite line 3 in core',
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		initials: 'AL',
		time: 1_700_000_000,
		lane: 0,
		color: 0,
		signed: false,
		parents: ['c'.repeat(40)],
		refs: [],
		edges: []
	};
}

function file(path: string, status: ChangedFile['status'] = 'modified'): ChangedFile {
	return { path, status };
}

function detail(overrides: Partial<Detail> = {}): Detail {
	return {
		id: 'b'.repeat(40),
		short: 'bbbbbbb',
		summary: 'Rewrite line 3 in core',
		body: 'A longer explanation.',
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		authorTime: 1_700_000_000,
		committerName: 'Charles Babbage',
		committerEmail: 'charles@example.com',
		commitTime: 1_700_000_600,
		signed: false,
		parents: ['c'.repeat(40)],
		files: [file('core.txt'), file('src/deep/nested/main.rs'), file('new.txt', 'added')],
		...overrides
	};
}

beforeEach(() => {
	control.reset();
	goto.mockClear();
});

describe('empty and loading states', () => {
	it('asks for a commit when none is selected', () => {
		const view = render(CommitDetail, {});
		expect(view.text()).toContain('Select a commit to see it here');
		view.destroy();
	});

	it('shows the short SHA while the detail is still loading', () => {
		control.setRows([graphRow()]);
		control.setSelected(0);

		const view = render(CommitDetail, {});

		expect(view.text()).toContain('bbbbbbb');
		expect(view.text()).toContain('Loading…');
		view.destroy();
	});

	it('shows an error instead of the detail', () => {
		control.setRows([graphRow()]);
		control.setSelected(0);
		control.setDetailError('no commit bbbbbbb');

		const view = render(CommitDetail, {});

		expect(view.text()).toContain('no commit bbbbbbb');
		expect(view.text()).not.toContain('Loading…');
		view.destroy();
	});
});

describe('a loaded commit', () => {
	function open(overrides: Partial<Detail> = {}) {
		control.setRows([graphRow()]);
		control.setSelected(0);
		control.setDetail(detail(overrides));
		return render(CommitDetail, {});
	}

	it('says a signed commit is signed, and that it is not verified', () => {
		// Said in full here because there is room for the caveat, where the
		// graph row has a letter and a tooltip (FEAT-019).
		const view = open({ signed: true });

		expect(view.text()).toContain('Signed, not verified here.');

		view.destroy();
	});

	it('says nothing at all about an unsigned commit', () => {
		// "not signed" on every commit in a repository nobody signs is a line of
		// noise on every commit; the absence already says it.
		const view = open({ signed: false });

		expect(view.text()).not.toContain('Signed');
		expect(view.text()).not.toContain('verify');

		view.destroy();
	});

	it('shows the message, both people and the parent', () => {
		const view = open();

		expect(view.text()).toContain('Rewrite line 3 in core');
		expect(view.text()).toContain('A longer explanation.');
		expect(view.text()).toContain('Ada Lovelace');
		expect(view.text()).toContain('Charles Babbage');
		expect(view.text()).toContain('parent ccccccc');

		view.destroy();
	});

	it('distinguishes the author from the committer', () => {
		const view = open();
		expect(view.text()).toContain('authored');
		expect(view.text()).toContain('committed');
		view.destroy();
	});

	it('says when a commit is a root rather than showing no parent at all', () => {
		const view = open({ parents: [] });
		expect(view.text()).toContain('root commit');
		view.destroy();
	});

	it('counts the changed files, singular and plural', () => {
		const many = open();
		expect(many.text()).toContain('3 files changed');
		many.destroy();

		const one = open({ files: [file('core.txt')] });
		expect(one.text()).toContain('1 file changed');
		one.destroy();
	});

	it('says so when a commit changed nothing', () => {
		const view = open({ files: [] });
		expect(view.text()).toContain('No file changes');
		view.destroy();
	});

	it('lists full paths by default and basenames grouped by directory in tree mode', () => {
		const view = open();

		expect(view.all('.file')).toHaveLength(3);
		// The name, then its folder (TASK-046); the whole path is the row's title.
		const row = view.all('.file').find((f) => f.getAttribute('title') === 'src/deep/nested/main.rs');
		expect(row?.querySelector('.path')?.textContent).toBe('main.rs');
		expect(row?.querySelector('.folder')?.textContent).toBe('src/deep/nested');

		const [, tree] = view.all('.toggle button');
		click(tree);

		expect(view.text()).toContain('src/deep/nested/');
		// The basename is what is listed under the directory heading.
		expect(view.all('.file.indent').some((f) => f.textContent?.includes('main.rs'))).toBe(true);

		view.destroy();
	});

	it('groups files at the repository root under ./', () => {
		const view = open({ files: [file('core.txt')] });
		click(view.all('.toggle button')[1]);

		expect(view.text()).toContain('./');
		view.destroy();
	});

	it('keeps a dotfile reading as a dotfile', () => {
		const view = open({ files: [file('.gitignore')] });
		expect(view.get('.path').textContent).toBe('.gitignore');
		view.destroy();
	});

	it('opens the diff from a file row and from the button', () => {
		const onopen = vi.fn();
		control.setRows([graphRow()]);
		control.setSelected(0);
		control.setDetail(detail());
		const view = render(CommitDetail, { onopen });

		click(view.all('.file')[0]);
		expect(onopen).toHaveBeenCalledWith('b'.repeat(40));

		onopen.mockClear();
		const openFull = view
			.all('button')
			.find((b) => b.textContent?.includes('Open full diff'));
		click(openFull as HTMLElement);
		expect(onopen).toHaveBeenCalledWith('b'.repeat(40));

		view.destroy();
	});

	/**
	 * Every commit action works (TASK-046). "Merge into" and "Revert" were
	 * chips titled "Not built yet" that did nothing.
	 */
	it('offers only commit actions that do something', () => {
		const view = open();
		const labels = view.all('.actions .chip').map((c) => c.textContent?.trim());

		expect(labels).toEqual(['Cherry-pick', 'Revert', 'Interactive rebase', 'Copy SHA']);
		for (const chip of view.all('.actions .chip')) {
			expect(chip.tagName).toBe('BUTTON');
			expect(chip.getAttribute('title') ?? '').not.toMatch(/not built/i);
		}
		expect(view.all('.actions .chip')[0].getAttribute('title')).toContain('main');
		view.destroy();
	});

	it('reaches the rebase screen', () => {
		const view = open();
		const rebase = view.all('.chip').find((c) => c.textContent?.includes('Interactive rebase'));
		click(rebase as HTMLElement);

		expect(goto).toHaveBeenCalledWith('/rebase');
		view.destroy();
	});
});

describe('copying the SHA', () => {
	function open() {
		control.setRows([graphRow()]);
		control.setSelected(0);
		control.setDetail(detail());
		return render(CommitDetail, {});
	}

	it('copies the full SHA, not the short one', async () => {
		const writeText = vi.fn(() => Promise.resolve());
		vi.stubGlobal('navigator', { clipboard: { writeText } });

		const view = open();
		click(view.get('.sha'));
		await Promise.resolve();

		expect(writeText).toHaveBeenCalledWith('b'.repeat(40));
		view.destroy();
	});

	it('confirms briefly, then goes back', async () => {
		vi.useFakeTimers();
		vi.stubGlobal('navigator', { clipboard: { writeText: () => Promise.resolve() } });

		const view = open();
		click(view.get('.sha'));
		await vi.advanceTimersByTimeAsync(0);
		flushSync();
		expect(view.text()).toContain('Copied');

		await vi.advanceTimersByTimeAsync(1500);
		flushSync();
		expect(view.text()).not.toContain('Copied');

		view.destroy();
		vi.useRealTimers();
	});

	it('does not claim success when the clipboard refuses', async () => {
		vi.stubGlobal('navigator', {
			clipboard: { writeText: () => Promise.reject(new Error('denied')) }
		});

		const view = open();
		click(view.get('.sha'));
		await Promise.resolve();
		await Promise.resolve();
		flushSync();

		expect(view.text()).not.toContain('Copied');
		view.destroy();
	});
});
