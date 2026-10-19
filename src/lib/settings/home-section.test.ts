// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Settings › Home (FEAT-148): show, hide and order Home's cards.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, press, render } from '../../testing/mount';
import { drag } from '../../testing/pointer';
import type { Settings } from '$lib/types';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true),
	settings: vi.fn(),
	setSettings: vi.fn(() => Promise.resolve()),
	identity: vi.fn(() => Promise.reject(new Error('not under test'))),
	signing: vi.fn(() => Promise.reject(new Error('not under test'))),
	forgeAccounts: vi.fn(() => Promise.resolve([])),
	licenses: vi.fn(() => Promise.reject(new Error('not under test'))),
	about: vi.fn(() => Promise.reject(new Error('not under test')))
}));

import * as api from '$lib/api';
import { HOME_CARDS, type HomeLayout } from '$lib/home/layout';
import HomeSection from './HomeSection.svelte';
import { settings } from './store.svelte';

const settingsCall = vi.mocked(api.settings);
const setSettings = vi.mocked(api.setSettings);

function stored(home: Partial<HomeLayout> = {}): Settings {
	return {
		checkForUpdates: true,
		confirmHistoryRewrite: true,
		showGitCommands: false,
		pruneOnFetch: false,
		fetchAvatars: true,
		personality: 'off',
		sound: 'off',
		notifyPullRequests: false,
		notifyDesktop: true,
		notifyMerged: true,
		notifyComments: true,
		notifyReviewRequests: true,
		notifyChecks: true,
		notifyEveryMinutes: 2,
		rail: { hidden: [], order: [], where: 'more' },
		home: { hidden: [], order: [], ...home },
		terminalShell: '',
		terminalMultiplexer: true
	};
}

async function mounted(home: Partial<HomeLayout> = {}) {
	settingsCall.mockResolvedValue(stored(home));
	await settings.load();
	return render(HomeSection, {});
}

function written(): HomeLayout {
	const call = setSettings.mock.calls.at(-1);
	if (!call) throw new Error('nothing was written');
	return call[0].home;
}

async function settle(): Promise<void> {
	await Promise.resolve();
	await Promise.resolve();
	flushSync();
}

const radio = (row: HTMLElement, name: string) =>
	[...row.querySelectorAll<HTMLElement>('[role="radio"]')].find((b) => b.textContent?.trim() === name)!;

beforeEach(() => {
	vi.clearAllMocks();
	settings.clearState();
	vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => {}, removeItem: () => {} });
});

describe('Settings › Home', () => {
	it('lists every card, shown, in the designed order', async () => {
		const view = await mounted();

		expect(view.all('.row').map((row) => row.dataset.card)).toEqual(HOME_CARDS.map((card) => card.id));
		expect(view.text()).toContain(`${HOME_CARDS.length} shown · 0 hidden`);
		expect(view.all('.tile')).toHaveLength(HOME_CARDS.length);
		view.destroy();
	});

	it('hides a card, and the preview loses it', async () => {
		const view = await mounted();

		click(radio(view.get('.row[data-card="heat"]'), 'Hide'));
		await settle();

		expect(written().hidden).toEqual(['heat']);
		expect(view.all('.tile').map((tile) => tile.textContent)).not.toContain('Commits, last 16 weeks');
		view.destroy();
	});

	it('moves a card by its grip, and with Alt and the arrows', async () => {
		const view = await mounted();
		const releases = view.get('.row[data-card="releases"]');

		drag(releases.querySelector<HTMLElement>('[data-sort-grip]')!, view.get('.row[data-card="needs"]'));
		await settle();
		expect(written().order[0]).toBe('releases');
		expect(view.all('.row')[0].dataset.card).toBe('releases');

		press(radio(view.get('.row[data-card="releases"]'), 'Show'), 'ArrowDown', { altKey: true });
		await settle();
		expect(written().order.slice(0, 2)).toEqual(['needs', 'releases']);
		view.destroy();
	});

	it('resets to every card in the designed order', async () => {
		const view = await mounted({ hidden: ['farm'], order: ['farm', 'needs'] });

		const reset = view.all('button').find((button) => button.textContent?.trim() === 'Reset')!;
		click(reset);
		await settle();
		expect(written()).toEqual({ hidden: [], order: [] });
		view.destroy();
	});
});
