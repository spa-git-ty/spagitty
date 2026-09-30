// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, press, render } from '../../testing/mount';
import type { Blame, BlameLine, SearchRow } from '$lib/types';

const listeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((name: string, handler: (event: { payload: unknown }) => void) => {
		listeners.set(name, handler);
		return Promise.resolve(() => listeners.delete(name));
	})
}));

vi.mock('$lib/api', () => ({
	searchStart: vi.fn(() => Promise.resolve(1)),
	searchStop: vi.fn(() => Promise.resolve()),
	blame: vi.fn(),
	commitDetail: vi.fn()
}));

import * as api from '$lib/api';
import BlameStrip from './BlameStrip.svelte';
import QueryBar from './QueryBar.svelte';
import ResultDetail from './ResultDetail.svelte';
import ResultRows from './ResultRows.svelte';
import { search } from './store.svelte';

const blameCall = vi.mocked(api.blame);
const searchStart = vi.mocked(api.searchStart);

function row(index: number, overrides: Partial<SearchRow> = {}): SearchRow {
	return {
		index,
		id: `${index}`.padStart(40, 'a'),
		short: `aaaaaa${index}`,
		summary: `Commit ${index}`,
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		initials: 'AL',
		time: Math.floor(Date.now() / 1000) - 3600,
		refs: [],
		...overrides
	};
}

function blameLine(line: number, overrides: Partial<BlameLine> = {}): BlameLine {
	return {
		line,
		text: `line ${line}`,
		commit: 'a'.repeat(40),
		short: 'aaaaaaa',
		summary: 'A commit',
		authorName: 'Ada Lovelace',
		time: Math.floor(Date.now() / 1000) - 3600,
		sourcePath: null,
		...overrides
	};
}

function blameOf(overrides: Partial<Blame> = {}): Blame {
	return {
		path: 'core.txt',
		revision: 'abcdef1234567890abcdef1234567890abcdef12',
		lines: [blameLine(1), blameLine(2)],
		refused: null,
		...overrides
	};
}

async function withRows(rows: SearchRow[]) {
	search.message = 'line';
	await search.run();
	listeners.get('search-rows')?.({ payload: { token: 1, rows } });
}

beforeEach(async () => {
	vi.clearAllMocks();
	listeners.clear();
	await search.stop();
	searchStart.mockResolvedValue(1);
	vi.mocked(api.commitDetail).mockResolvedValue({
		id: 'a'.repeat(40),
		short: 'aaaaaaa',
		summary: 'A commit',
		body: '',
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		authorTime: 1_700_000_000,
		committerName: 'Ada Lovelace',
		committerEmail: 'ada@example.com',
		commitTime: 1_700_000_000,
		signed: false,
		parents: [],
		files: []
	});
	await search.attach();
});

describe('QueryBar', () => {
	it('disables Search while nothing is being asked', () => {
		const view = render(QueryBar, {});

		expect((view.get('button') as HTMLButtonElement).disabled).toBe(true);
		view.destroy();
	});

	it('shows a chip per applied filter, and nothing when none are', async () => {
		const view = render(QueryBar, {});
		expect(view.all('.chips .chip')).toHaveLength(0);

		search.author = 'ada';
		search.path = 'core.txt';
		await Promise.resolve();

		const view2 = render(QueryBar, {});
		expect(view2.all('.chips .chip').map((c) => c.textContent?.trim())).toEqual([
			'author:ada ×',
			'path:core.txt ×'
		]);
		view.destroy();
		view2.destroy();
	});

	it('a chip removes its own filter', async () => {
		search.author = 'ada';
		search.message = 'line';
		const view = render(QueryBar, {});

		click(view.all('.chips .chip')[0]);
		await Promise.resolve();

		expect(search.author).toBe('');
		view.destroy();
	});

	it('typing a field into the box is what the chip reflects', async () => {
		const view = render(QueryBar, {});
		const input = view.all('input')[0] as HTMLInputElement;

		input.value = 'grace';
		input.dispatchEvent(new Event('input', { bubbles: true }));
		await Promise.resolve();

		expect(search.author).toBe('grace');
		view.destroy();
	});
});

describe('ResultRows', () => {
	it('draws one row per result, with its author, time and short id', async () => {
		await withRows([row(0), row(1)]);
		const view = render(ResultRows, {});

		const rows = view.all('.row');
		expect(rows).toHaveLength(2);
		expect(rows[0].textContent).toContain('Commit 0');
		expect(rows[0].textContent).toContain('Ada Lovelace');
		expect(rows[0].textContent).toContain('aaaaaa0');
		view.destroy();
	});

	it('shows the refs a result carries', async () => {
		await withRows([row(0, { refs: [{ name: 'main', kind: 'branch', current: true, local: true, remotes: [], divergence: null }] })]);
		const view = render(ResultRows, {});

		expect(view.text()).toContain('main');
		view.destroy();
	});

	it('a plain click opens the commit, not its diff', async () => {
		await withRows([row(0)]);
		const opened: string[] = [];
		const diffed: string[] = [];
		const view = render(ResultRows, {
			onopen: (id: string) => opened.push(id),
			ondiff: (id: string) => diffed.push(id)
		});

		click(view.get('.row'));

		expect(opened).toEqual([row(0).id]);
		expect(diffed).toEqual([]);
		view.destroy();
	});

	it('holding alt opens the diff instead', async () => {
		await withRows([row(0)]);
		const opened: string[] = [];
		const diffed: string[] = [];
		const view = render(ResultRows, {
			onopen: (id: string) => opened.push(id),
			ondiff: (id: string) => diffed.push(id)
		});

		click(view.get('.row'), { altKey: true });

		expect(diffed).toEqual([row(0).id]);
		expect(opened).toEqual([]);
		view.destroy();
	});

	it('enter does what the footer says, and alt-enter does the other thing', async () => {
		await withRows([row(0)]);
		const opened: string[] = [];
		const diffed: string[] = [];
		const view = render(ResultRows, {
			onopen: (id: string) => opened.push(id),
			ondiff: (id: string) => diffed.push(id)
		});

		press(view.get('.row'), 'Enter');
		press(view.get('.row'), 'Enter', { altKey: true });

		expect(opened).toEqual([row(0).id]);
		expect(diffed).toEqual([row(0).id]);
		view.destroy();
	});

	it('marks the open result', async () => {
		await withRows([row(0), row(1)]);
		await search.select(row(1).id);
		const view = render(ResultRows, {});

		expect(view.all('.row.selected')).toHaveLength(1);
		expect(view.get('.row.selected').textContent).toContain('Commit 1');
		view.destroy();
	});
});

describe('BlameStrip', () => {
	it('shows nothing before it has been asked anything (TASK-050)', () => {
		const view = render(BlameStrip, {});

		expect(view.text()).not.toContain('who last touched each line');
		view.destroy();
	});

	it('groups a run of lines from one commit into one block', async () => {
		// A blame is read by change, not by line.
		blameCall.mockResolvedValue(
			blameOf({
				lines: [
					blameLine(1),
					blameLine(2),
					blameLine(3, { commit: 'b'.repeat(40), short: 'bbbbbbb' })
				]
			})
		);
		await search.loadBlame('core.txt');
		const view = render(BlameStrip, {});

		expect(view.all('.group')).toHaveLength(2);
		expect(view.all('.lines li')).toHaveLength(3);
		view.destroy();
	});

	it('names a binary file rather than showing an empty list', async () => {
		blameCall.mockResolvedValue(blameOf({ lines: [], refused: 'binary' }));
		await search.loadBlame('logo.bin');
		const view = render(BlameStrip, {});

		expect(view.text()).toContain('binary file');
		expect(view.all('.lines li')).toHaveLength(0);
		view.destroy();
	});

	it('says a path that is not there is not there', async () => {
		blameCall.mockResolvedValue(blameOf({ lines: [], refused: 'notAFile' }));
		await search.loadBlame('gone.txt');
		const view = render(BlameStrip, {});

		expect(view.text()).toContain('No such file at that revision');
		view.destroy();
	});

	it('says a file that is too large is too large', async () => {
		blameCall.mockResolvedValue(blameOf({ lines: [], refused: 'tooLarge' }));
		await search.loadBlame('huge.txt');
		const view = render(BlameStrip, {});

		expect(view.text()).toContain('too large');
		view.destroy();
	});

	it('shows where a line lived before, when it arrived under another name', async () => {
		blameCall.mockResolvedValue(
			blameOf({ lines: [blameLine(1, { sourcePath: 'notes.md' })] })
		);
		await search.loadBlame('journal.md');
		const view = render(BlameStrip, {});

		expect(view.text()).toContain('was notes.md');
		view.destroy();
	});

	it('shows a read failure', async () => {
		blameCall.mockRejectedValue(new Error('no repository is open'));
		await search.loadBlame('core.txt');
		const view = render(BlameStrip, {});

		expect(view.text()).toContain('no repository is open');
		view.destroy();
	});
});

describe('ResultDetail', () => {
	it('says what it is for before a result is opened', () => {
		const view = render(ResultDetail, {});

		expect(view.text()).toContain('Open a result to read it');
		view.destroy();
	});

	it('shows the message, the person and the files the commit touched', async () => {
		vi.mocked(api.commitDetail).mockResolvedValue({
			id: 'a'.repeat(40),
			short: 'aaaaaaa',
			summary: 'Merge feature/split-view',
			body: 'Why it was done.',
			authorName: 'Ada Lovelace',
			authorEmail: 'ada@example.com',
			authorTime: Math.floor(Date.now() / 1000) - 3600,
			committerName: 'Ada Lovelace',
			committerEmail: 'ada@example.com',
			commitTime: Math.floor(Date.now() / 1000) - 3600,
			signed: false,
			parents: [],
			files: [
				{ path: 'core.txt', status: 'modified' },
				{ path: 'src/deep/nested/main.rs', status: 'added' }
			]
		});
		await withRows([row(0)]);
		await search.select(row(0).id);
		const view = render(ResultDetail, {});

		expect(view.text()).toContain('Merge feature/split-view');
		expect(view.text()).toContain('Why it was done.');
		expect(view.text()).toContain('ada@example.com');
		expect(view.text()).toContain('2 files');
		expect(view.all('.filelist li')).toHaveLength(2);
		view.destroy();
	});

	it('offers the full diff, which is the other question and the other screen', async () => {
		await withRows([row(0)]);
		await search.select(row(0).id);
		const asked: string[] = [];
		const view = render(ResultDetail, { ondiff: (id: string) => asked.push(id) });

		click(view.get('button'));

		expect(asked).toEqual(['a'.repeat(40)]);
		view.destroy();
	});

	it('shows a read failure rather than an empty card', async () => {
		vi.mocked(api.commitDetail).mockRejectedValue(new Error('no commit abc'));
		await withRows([row(0)]);
		await search.select(row(0).id);
		const view = render(ResultDetail, {});

		expect(view.text()).toContain('no commit abc');
		view.destroy();
	});
});
