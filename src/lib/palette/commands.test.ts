// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';

// Hoisted, because `vi.mock`'s factory runs before the module body does.
const { goto } = vi.hoisted(() => ({ goto: vi.fn() }));
vi.mock('$app/navigation', () => ({ goto }));

// Only `settings` is replaced: the rest of the API surface is what the other
// commands in this file call, and stubbing it wholesale would test the stub.
vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: () => true,
	settings: () => Promise.resolve(toggles),
	identity: () => Promise.reject(new Error('not under test')),
	licenses: () => Promise.reject(new Error('not under test')),
	about: () => Promise.reject(new Error('not under test'))
}));

let toggles = { confirmHistoryRewrite: true, showGitCommands: false };

import { registerCommands } from './commands';
import { settings } from '../settings/store.svelte';
import { palette } from './store.svelte';
import { columns } from '../graph/columns.svelte';
import { scale } from '../scale.svelte';

/** The registry is module-global, so each test starts from a known state. */
beforeEach(() => {
	palette.clear();
	goto.mockClear();
	registerCommands();
});

function find(id: string) {
	const command = palette.matches.find((match) => match.command.id === id)?.command;
	if (!command) throw new Error(`no command registered with id ${id}`);
	return command;
}

describe('registerCommands', () => {
	it('registers every command with a unique id', () => {
		const ids = palette.matches.map((match) => match.command.id);
		expect(ids.length).toBeGreaterThan(0);
		expect(new Set(ids).size).toBe(ids.length);
	});

	it('is idempotent, so a second call does not duplicate anything', () => {
		const before = palette.matches.length;
		registerCommands();
		expect(palette.matches.length).toBe(before);
	});

	it('groups every command, since the palette renders by heading', () => {
		for (const { command } of palette.matches) {
			expect(command.group).not.toBe('');
		}
	});

	it('navigates when a Go command runs', async () => {
		await find('go.settings').run();
		expect(goto).toHaveBeenCalledWith('/settings');
	});

	it('disables the repository commands with a reason when none is open', () => {
		const fetch = find('repo.fetch');
		expect(fetch.enabled?.()).toBe(false);
		expect(fetch.unavailable?.()).toBe('No repository open');
	});

	it('offers the command log only once the Settings toggle is on', async () => {
		toggles = { ...toggles, showGitCommands: false };
		await settings.load();

		const command = find('repo.commands');
		expect(command.enabled?.()).toBe(false);
		expect(command.unavailable?.()).toContain('Settings');

		toggles = { ...toggles, showGitCommands: true };
		await settings.load();

		expect(command.enabled?.()).toBe(true);
		expect(command.unavailable?.()).toBeNull();
	});

	it('leaves navigation enabled without a repository', () => {
		const go = find('go.repos');
		expect(go.enabled?.() ?? true).toBe(true);
	});

	it('toggles a column through the palette', () => {
		const before = columns.isShown('sha');
		find('view.column.sha').run();
		expect(columns.isShown('sha')).toBe(!before);
		find('view.column.sha').run();
		expect(columns.isShown('sha')).toBe(before);
	});

	it('offers to clear the author filter only when one is set', () => {
		const clear = find('view.author.clear');
		columns.setAuthor('');
		expect(clear.enabled?.()).toBe(false);

		columns.setAuthor('ada');
		expect(clear.enabled?.()).toBe(true);
		clear.run();
		expect(columns.author).toBe('');
	});

	it('offers both graph orders', () => {
		expect(find('view.order.date').title).toBe('Order graph by date');
		expect(find('view.order.branch').title).toBe('Order graph by branch');
	});

	it('moves the zoom, and resets both dials', () => {
		scale.setZoom(1);
		find('appearance.zoom.in').run();
		expect(scale.zoom).toBeGreaterThan(1);

		find('appearance.text.bigger').run();
		const grown = scale.text;
		expect(grown).toBeGreaterThan(1);

		find('appearance.zoom.reset').run();
		expect(scale.zoom).toBe(1);
		expect(scale.text).toBe(1);
	});

	it('finds a command by its initials, which is the point of the palette', () => {
		palette.setQuery('gtc');
		expect(palette.active?.id).toBe('go.changes');

		// Several commands share the same initials; the shorter title wins the
		// tie, which is what makes the ranking predictable rather than
		// arbitrary. `gtb` is Badges and Branches (FEAT-072), and `gts` is
		// Search, Stashes and Settings.
		palette.setQuery('gtb');
		expect(palette.active?.id).toBe('go.badges');

		palette.setQuery('gts');
		expect(palette.active?.id).toBe('go.search');
	});
});
