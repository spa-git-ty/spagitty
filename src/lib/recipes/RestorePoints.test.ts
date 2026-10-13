// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The restore points on the Reflog screen (FEAT-133).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render, type Mounted } from '../../testing/mount';
import { calls, control } from '../../testing/repo-store.svelte';
import type { RestorePoint } from './types';

const { confirm, notice } = vi.hoisted(() => ({
	confirm: vi.fn(),
	notice: { ok: vi.fn(), failed: vi.fn() }
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true)
}));
vi.mock('$lib/ui/dialog.svelte', () => ({ dialog: { confirm } }));
vi.mock('$lib/ui/notice.svelte', () => ({ notice }));
vi.mock('./api', () => ({
	restorePoints: vi.fn(),
	restoreTo: vi.fn(async () => {}),
	dropRestorePoint: vi.fn()
}));

import * as api from './api';
import RestorePoints from './RestorePoints.svelte';

const restorePoints = vi.mocked(api.restorePoints);
const restoreTo = vi.mocked(api.restoreTo);
const dropRestorePoint = vi.mocked(api.dropRestorePoint);

function point(id: string, recipe: string, branch: string | null = 'feature/avatars'): RestorePoint {
	return {
		id,
		created: Math.floor(Date.now() / 1000) - 120,
		recipe,
		branch,
		head: '9d02b7f' + '0'.repeat(33),
		tips: [{ name: 'feature/avatars', id: '9d02b7f' + '0'.repeat(33) }],
		makes: [],
		stashes: []
	};
}

let view: Mounted | null = null;

async function settle(): Promise<void> {
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	flushSync();
}

async function show(): Promise<Mounted> {
	view = render(RestorePoints, {});
	await settle();
	return view;
}

const rows = () => view!.all('.row').map((row) => row.textContent?.replace(/\s+/g, ' ').trim());
const chip = (row: number, name: string) =>
	[...view!.all('.row')[row].querySelectorAll('button')].find((b) => b.textContent?.trim() === name) as HTMLButtonElement;

beforeEach(() => {
	vi.clearAllMocks();
	control.reset();
	control.setInfo({
		path: '/repos/thing',
		name: 'thing',
		bare: false,
		lastFetched: null,
		head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
	});
	confirm.mockResolvedValue(true);
	restorePoints.mockResolvedValue([point('2-b', 'sync-and-ship'), point('1-a', 'squash-for-pr', null)]);
});

afterEach(() => {
	view?.destroy();
	view = null;
});

describe('restore points', () => {
	it('draws nothing until a recipe has made one', async () => {
		restorePoints.mockResolvedValue([]);

		await show();

		expect(view!.find('.points')).toBeNull();
	});

	it('lists each with its recipe, where HEAD was, and how long ago', async () => {
		await show();

		expect(view!.get('.points').getAttribute('aria-label')).toBe('Restore points');
		expect(view!.get('h3').textContent).toContain('the newest 20, for 7 days');
		expect(rows()).toEqual([
			'sync-and-ship on feature/avatars at 9d02b7f 2 minutes ago undo to here drop',
			'squash-for-pr on a detached HEAD at 9d02b7f 2 minutes ago undo to here drop'
		]);
	});

	it('goes back to one after asking, and reads the repository again', async () => {
		await show();

		click(chip(0, 'undo to here'));
		await settle();

		expect(confirm.mock.calls[0][0].title).toBe('Go back to before sync-and-ship?');
		expect(confirm.mock.calls[0][0].body).toContain('Changes made since are lost. What was pushed stays pushed.');
		expect(confirm.mock.calls[0][0].danger).toBe(true);
		expect(restoreTo).toHaveBeenCalledWith('2-b');
		expect(notice.ok).toHaveBeenCalledWith('Back to before sync-and-ship');
		expect(calls.refreshed).toBe(1);
	});

	it('does nothing when the answer is no', async () => {
		confirm.mockResolvedValue(false);
		await show();

		click(chip(0, 'undo to here'));
		await settle();

		expect(restoreTo).not.toHaveBeenCalled();
	});

	it('says why it could not go back', async () => {
		restoreTo.mockRejectedValue('A recipe is running here. Undo it from Recipes.');
		await show();

		click(chip(1, 'undo to here'));
		await settle();

		expect(notice.failed).toHaveBeenCalledWith('Could not go back', 'A recipe is running here. Undo it from Recipes.');
	});

	it('drops one, and shows what is left', async () => {
		dropRestorePoint.mockResolvedValue([point('1-a', 'squash-for-pr', null)]);
		await show();

		click(chip(0, 'drop'));
		await settle();

		expect(dropRestorePoint).toHaveBeenCalledWith('2-b');
		expect(rows()).toHaveLength(1);
		// Dropping is not undoing: nobody was asked, and nothing was put back.
		expect(confirm).not.toHaveBeenCalled();
		expect(restoreTo).not.toHaveBeenCalled();
	});

	it('draws nothing when they cannot be listed, or with no repository', async () => {
		restorePoints.mockRejectedValue('no');
		await show();
		expect(view!.find('.points')).toBeNull();

		view!.destroy();
		control.setInfo(null);
		restorePoints.mockClear();
		await show();
		expect(restorePoints).not.toHaveBeenCalled();
	});
});
