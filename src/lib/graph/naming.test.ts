// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fire, flushSync, press, render } from '../../testing/mount';
import type { GraphRow } from '$lib/types';

vi.mock('$lib/graph/store.svelte', async () => await import('../../testing/graph-store.svelte'));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/graph/actions', async (original) => ({
	...(await original<typeof import('./actions')>()),
	createBranchNamed: vi.fn(async () => true)
}));

import { control } from '../../testing/graph-store.svelte';
import { columns } from './columns.svelte';
import { createBranchNamed } from './actions';
import { branching } from './branching.svelte';
import CommitRows from './CommitRows.svelte';

function row(index: number): GraphRow {
	return {
		index,
		id: `${index}`.padStart(40, 'a'),
		short: `${index}`.padStart(7, 'a'),
		summary: `commit ${index}`,
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		initials: 'AL',
		time: 1_700_000_000 - index * 86_400,
		lane: 0,
		color: 0,
		signed: false,
		parents: [],
		refs: [],
		edges: []
	};
}

beforeEach(() => {
	control.reset();
	columns.reset();
	branching.cancel();
	vi.mocked(createBranchNamed).mockClear();
	control.setRows([row(0), row(1), row(2)]);
	vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => {}, removeItem: () => {} });
});

/** FEAT-104: the toolbar's Branch names a branch in HEAD's own row. */
describe('a branch named in the graph', () => {
	it('opens a focused name field in the row it was asked for, and only there', () => {
		const view = render(CommitRows, {});
		branching.start(row(1).id);
		flushSync();

		const fields = view.all('.name-field');
		expect(fields).toHaveLength(1);
		expect(fields[0].closest('.row')?.id).toBe('commit-1');
		expect(document.activeElement).toBe(fields[0]);

		view.destroy();
	});

	it('creates the typed name there on Enter, spaces as dashes', async () => {
		const view = render(CommitRows, {});
		branching.start(row(1).id);
		flushSync();

		const field = view.get('.name-field') as HTMLInputElement;
		field.value = 'my feature';
		fire(field, 'input');
		press(field, 'Enter');
		await Promise.resolve();
		await Promise.resolve();
		flushSync();

		expect(createBranchNamed).toHaveBeenCalledWith('my-feature', row(1).id);
		expect(branching.at).toBeNull();
		expect(view.all('.name-field')).toHaveLength(0);

		view.destroy();
	});

	it('puts the field away on Escape and creates nothing', () => {
		const view = render(CommitRows, {});
		branching.start(row(0).id);
		flushSync();

		press(view.get('.name-field'), 'Escape');

		expect(branching.at).toBeNull();
		expect(createBranchNamed).not.toHaveBeenCalled();
		view.destroy();
	});

	it('creates nothing for an empty name', () => {
		const view = render(CommitRows, {});
		branching.start(row(0).id);
		flushSync();

		press(view.get('.name-field'), 'Enter');

		expect(createBranchNamed).not.toHaveBeenCalled();
		expect(branching.at).toBe(row(0).id);
		view.destroy();
	});
});
