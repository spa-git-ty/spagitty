// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Settings › Rail (FEAT-134): pin or hide what is on the rail.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, press, render } from '../../testing/mount';
import type { Settings } from '$lib/types';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const { page } = vi.hoisted(() => ({ page: { url: new URL('http://localhost/settings') } }));
vi.mock('$app/state', () => ({ page }));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

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
import { NAV_ITEMS, RAIL_DEFAULT, type RailSettings } from '$lib/nav';
import { control as repoControl } from '../../testing/repo-store.svelte';
import RailSection from './RailSection.svelte';
import { settings } from './store.svelte';

const settingsCall = vi.mocked(api.settings);
const setSettings = vi.mocked(api.setSettings);

function stored(rail: Partial<RailSettings> = {}): Settings {
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
		rail: { ...RAIL_DEFAULT, ...rail },
		terminalShell: '',
		terminalMultiplexer: true
	};
}

async function mounted(rail: Partial<RailSettings> = {}) {
	settingsCall.mockResolvedValue(stored(rail));
	await settings.load();
	return render(RailSection, {});
}

/** The last rail the section asked to be stored. */
function written(): RailSettings {
	const call = setSettings.mock.calls.at(-1);
	if (!call) throw new Error('nothing was written');
	return call[0].rail;
}

async function settle(): Promise<void> {
	await Promise.resolve();
	await Promise.resolve();
	flushSync();
}

const code = (href: string) => NAV_ITEMS.find((item) => item.href === href)!.code;
const rowOf = (view: { get(selector: string): HTMLElement }, href: string) =>
	view.get(`.row[data-code="${code(href)}"]`);
const radio = (row: HTMLElement, name: string) =>
	[...row.querySelectorAll<HTMLElement>('[role="radio"]')].find(
		(button) => button.textContent?.trim() === name
	)!;

beforeEach(() => {
	vi.clearAllMocks();
	settings.clearState();
	repoControl.reset();
	page.url = new URL('http://localhost/settings');
	vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => {}, removeItem: () => {} });
});

describe('Settings › Rail', () => {
	it('gives every screen a row, in its group, with its code', async () => {
		const view = await mounted();

		const shown = NAV_ITEMS.filter((item) => !item.delight);
		expect(view.all('.row').map((row) => row.dataset.code)).toEqual(shown.map((item) => item.code));
		expect(view.all('.card').map((card) => card.getAttribute('aria-label'))).toEqual([
			'Farm',
			'Repository',
			'Tools',
			'Issues',
			'Spagitty'
		]);
		expect(rowOf(view, '/stash').textContent).toContain('Stash');
		expect(rowOf(view, '/stash').textContent).toContain(code('/stash'));

		view.destroy();
	});

	it('starts with everything pinned and nothing to reset', async () => {
		const view = await mounted();

		expect(view.text()).toContain('0 hidden');
		for (const row of view.all('.row')) {
			const pin = radio(row, 'Pin');
			if (pin) expect(pin.getAttribute('aria-checked')).toBe('true');
		}
		const reset = view.all('button').find((button) => button.textContent?.trim() === 'Reset')!;
		expect((reset as HTMLButtonElement).disabled).toBe(true);

		view.destroy();
	});

	/** Settings has to stay reachable to undo a hide. */
	it('reads Always on Farm and Settings, with no choice to make', async () => {
		const view = await mounted();

		for (const href of ['/farm', '/settings']) {
			const row = rowOf(view, href);
			expect(row.textContent).toContain('Always');
			expect(row.querySelector('[role="radiogroup"]')).toBeNull();
		}

		view.destroy();
	});

	it('shows the rule Conflicts keeps in place of the choice', async () => {
		const view = await mounted();

		const row = rowOf(view, '/conflicts');
		expect(row.textContent).toContain('When there are any');
		expect(row.querySelector('[role="radiogroup"]')).toBeNull();

		view.destroy();
	});

	it('hides a screen, and pins it again', async () => {
		const view = await mounted();

		click(radio(rowOf(view, '/stash'), 'Hide'));
		await settle();

		expect(written().hidden).toEqual([code('/stash')]);
		expect(radio(rowOf(view, '/stash'), 'Hide').getAttribute('aria-checked')).toBe('true');
		expect(rowOf(view, '/stash').classList.contains('off')).toBe(true);
		expect(view.text()).toContain('1 hidden');

		click(radio(rowOf(view, '/stash'), 'Pin'));
		await settle();

		expect(written().hidden).toEqual([]);

		view.destroy();
	});

	it('writes nothing when the choice is the one already made', async () => {
		const view = await mounted();

		click(radio(rowOf(view, '/stash'), 'Pin'));
		await settle();

		expect(setSettings).not.toHaveBeenCalled();

		view.destroy();
	});

	it('chooses where hidden screens go', async () => {
		const view = await mounted({ hidden: [code('/stash')] });

		const palette = view
			.all('[role="radio"]')
			.find((button) => button.textContent?.trim() === 'Palette only')!;
		click(palette);
		await settle();

		expect(written().where).toBe('palette');
		expect(written().hidden).toEqual([code('/stash')]);

		view.destroy();
	});

	it('moves a row with Alt and the arrows, within its group only', async () => {
		const view = await mounted();

		const event = press(radio(rowOf(view, '/rebase'), 'Pin'), 'ArrowDown', { altKey: true });
		await settle();

		expect(event.defaultPrevented).toBe(true);
		const tools = view
			.all('.card[aria-label="Tools"] .row')
			.map((row) => row.dataset.code);
		expect(tools[1]).toBe(code('/rebase'));
		expect(written().order).toContain(code('/rebase'));

		// The first row of Repository has nowhere to go but out, so it stays.
		setSettings.mockClear();
		press(radio(rowOf(view, '/'), 'Pin'), 'ArrowUp', { altKey: true });
		await settle();
		expect(setSettings).not.toHaveBeenCalled();

		view.destroy();
	});

	it('leaves the arrows alone without Alt', async () => {
		const view = await mounted();

		const event = press(radio(rowOf(view, '/rebase'), 'Pin'), 'ArrowDown');
		await settle();

		expect(event.defaultPrevented).toBe(false);
		expect(setSettings).not.toHaveBeenCalled();

		view.destroy();
	});

	it('resets to everything pinned, the built-in order, More', async () => {
		const view = await mounted({
			hidden: [code('/stash'), code('/tags')],
			order: [code('/tags'), code('/rebase')],
			where: 'palette'
		});

		const reset = view.all('button').find((button) => button.textContent?.trim() === 'Reset')!;
		expect((reset as HTMLButtonElement).disabled).toBe(false);
		click(reset);
		await settle();

		expect(written()).toEqual(RAIL_DEFAULT);
		expect(view.text()).toContain('0 hidden');

		view.destroy();
	});

	describe('the preview', () => {
		it('is the rail itself, drawn from what is chosen', async () => {
			const view = await mounted({ hidden: [code('/stash'), code('/tags')] });

			const preview = view.get('nav[aria-label="Preview of the rail"]');
			const labels = [...preview.querySelectorAll('.item')].map((item) =>
				item.getAttribute('aria-label')
			);

			expect(labels).not.toContain('Stash');
			expect(labels).not.toContain('Tags');
			expect(labels).toContain('Rebase');
			expect(labels).toContain('More');
			// It is there to be looked at: nothing in it can be pressed.
			expect(preview.hasAttribute('inert')).toBe(true);

			view.destroy();
		});

		it('lists what is under More beside it, and nothing when that is the palette', async () => {
			const view = await mounted({ hidden: [code('/stash'), code('/tags')] });

			expect(view.all('.under li').map((item) => item.textContent?.trim())).toEqual([
				'Stash',
				'Tags'
			]);
			view.destroy();

			settings.clearState();
			const palette = await mounted({ hidden: [code('/stash')], where: 'palette' });
			expect(palette.find('.under')).toBeNull();
			expect(
				[...palette.get('nav[aria-label="Preview of the rail"]').querySelectorAll('.item')].map(
					(item) => item.getAttribute('aria-label')
				)
			).not.toContain('More');
			palette.destroy();
		});

		it('follows a change as it is made', async () => {
			const view = await mounted();
			const labels = () =>
				[...view.get('nav[aria-label="Preview of the rail"]').querySelectorAll('.item')].map(
					(item) => item.getAttribute('aria-label')
				);

			expect(labels()).toContain('Reflog');
			click(radio(rowOf(view, '/reflog'), 'Hide'));
			await settle();
			expect(labels()).not.toContain('Reflog');

			view.destroy();
		});
	});
});
