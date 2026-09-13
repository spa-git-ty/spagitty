// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How the graph sequences commits.
 *
 * The store persists a name and, when a repository is open, tells the backend
 * to restart the walk. The drawing itself is asserted in Rust: this file is
 * about the preference and the wire call, not about which commit sits where.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { RepoInfo } from '$lib/types';

vi.mock('$lib/api', () => ({
	graphOrder: vi.fn((_order: string) => Promise.resolve(7))
}));

vi.mock('./store.svelte', () => ({
	graph: { adopt: vi.fn(() => Promise.resolve()) }
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import * as api from '$lib/api';
import { graph } from './store.svelte';
import { control } from '../../testing/repo-store.svelte';
import { graphOrder, ORDERS } from './order.svelte';

const graphOrderCmd = vi.mocked(api.graphOrder);
const adopt = vi.mocked(graph.adopt);

const KEY = 'spagitty.graph.order';

function stubStorage(initial: Record<string, string> = {}) {
	const store = new Map(Object.entries(initial));
	vi.stubGlobal('localStorage', {
		getItem: (key: string) => store.get(key) ?? null,
		setItem: (key: string, value: string) => void store.set(key, value)
	});
	return store;
}

function aRepo(): RepoInfo {
	return {
		path: '/repos/fixture',
		name: 'fixture',
		bare: false,
		lastFetched: null,
		head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
	};
}

beforeEach(async () => {
	stubStorage();
	control.reset();
	graphOrderCmd.mockClear();
	adopt.mockClear();
	graphOrder.init();
	await graphOrder.set('date');
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('the preference', () => {
	it('starts as date, which is what every release before this drew', () => {
		stubStorage();
		graphOrder.init();

		expect(graphOrder.id).toBe('date');
	});

	it('persists and restores', () => {
		const store = stubStorage();
		void graphOrder.set('branch');
		expect(store.get(KEY)).toBe('branch');

		stubStorage({ [KEY]: 'branch' });
		graphOrder.init();
		expect(graphOrder.id).toBe('branch');
	});

	it('ignores a stored value that is not an order', () => {
		stubStorage({ [KEY]: 'topo' });
		graphOrder.init();

		expect(graphOrder.id).toBe('date');
	});

	it('survives storage being unreadable', () => {
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new Error('denied');
			},
			setItem: () => {
				throw new Error('denied');
			}
		});

		expect(() => graphOrder.init()).not.toThrow();
		expect(() => void graphOrder.set('branch')).not.toThrow();
	});

	it('offers every order it can be set to', () => {
		expect(ORDERS.map((option) => option.id)).toEqual(['date', 'branch']);
	});
});

describe('telling the walk', () => {
	it('does not restart when nothing is open', async () => {
		await graphOrder.set('branch');

		expect(graphOrder.id).toBe('branch');
		expect(graphOrderCmd).not.toHaveBeenCalled();
		expect(adopt).not.toHaveBeenCalled();
	});

	it('restarts the walk against the new order when a repository is open', async () => {
		control.setInfo(aRepo());

		await graphOrder.set('branch');

		expect(graphOrderCmd).toHaveBeenCalledWith('branch');
		expect(adopt).toHaveBeenCalledWith(7);
	});

	it('does not restart when the order is already the one asked for', async () => {
		control.setInfo(aRepo());

		await graphOrder.set('date');

		expect(graphOrderCmd).not.toHaveBeenCalled();
		expect(adopt).not.toHaveBeenCalled();
	});
});
