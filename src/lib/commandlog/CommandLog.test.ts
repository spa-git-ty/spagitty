// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, render } from '../../testing/mount';
import { GIT_COMMAND_EVENT, type ExecutedCommand } from '$lib/types';

const listeners = new Map<string, (event: { payload: unknown }) => void>();
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((name: string, handler: (event: { payload: unknown }) => void) => {
		listeners.set(name, handler);
		return Promise.resolve(() => listeners.delete(name));
	})
}));

vi.mock('$lib/api', () => ({
	inTauri: () => true,
	gitCommands: vi.fn(() => Promise.resolve([] as ExecutedCommand[])),
	clearGitCommands: vi.fn(() => Promise.resolve())
}));

import CommandLog from './CommandLog.svelte';
import { commandLog } from './store.svelte';

function emit(entry: ExecutedCommand): void {
	listeners.get(GIT_COMMAND_EVENT)?.({ payload: entry });
}

const OK: ExecutedCommand = {
	seq: 1,
	atMs: 1_700_000_000_000,
	argv: ['git', 'fetch', '--prune', '--progress', '--all'],
	outcome: { kind: 'ok' },
	durationMs: 812
};

const FAILED: ExecutedCommand = {
	seq: 2,
	atMs: 1_700_000_001_000,
	argv: ['git', 'push', 'origin', 'main'],
	outcome: { kind: 'failed', code: 1, stderr: 'error: failed to push some refs' },
	durationMs: 240
};

const CLONING: ExecutedCommand = {
	seq: 3,
	atMs: 1_700_000_002_000,
	argv: ['git', 'clone', '--progress', '--', 'https://maxmya:***@host/repo.git', '/work/repo'],
	outcome: { kind: 'started' },
	durationMs: 0
};

describe('the command log panel', () => {
	beforeEach(async () => {
		listeners.clear();
		await commandLog.clear();
		commandLog.hide();
		await commandLog.attach();
	});

	it('shows nothing at all while it is closed', () => {
		emit(OK);
		const view = render(CommandLog, {});

		expect(view.find('.drawer')).toBeNull();
		view.destroy();
	});

	it('shows the command with every flag the shell layer added', async () => {
		emit(OK);
		await commandLog.show();
		const view = render(CommandLog, {});

		expect(view.get('.line').textContent).toBe('git fetch --prune --progress --all');
		expect(view.text()).toContain('812 ms');
		view.destroy();
	});

	it("shows git's own words under a command that failed, with its exit code", async () => {
		emit(FAILED);
		await commandLog.show();
		const view = render(CommandLog, {});

		expect(view.text()).toContain('error: failed to push some refs');
		expect(view.text()).toContain('exit 1');
		view.destroy();
	});

	it('says a clone is running rather than claiming it finished', async () => {
		emit(CLONING);
		await commandLog.show();
		const view = render(CommandLog, {});

		expect(view.text()).toContain('running');
		expect(view.find('.stderr')).toBeNull();
		view.destroy();
	});

	it('shows the newest command first', async () => {
		emit(OK);
		emit(FAILED);
		await commandLog.show();
		const view = render(CommandLog, {});

		const lines = view.all('.line').map((element) => element.textContent);
		expect(lines[0]).toContain('push');
		view.destroy();
	});

	it('lists what ran with no footer explaining what did not (TASK-050)', async () => {
		await commandLog.show();
		const view = render(CommandLog, {});

		expect(view.find('.foot')).toBeNull();
		view.destroy();
	});

	it('explains an empty log instead of showing a blank panel', async () => {
		await commandLog.show();
		const view = render(CommandLog, {});

		expect(view.get('.empty').textContent?.trim()).toBe('Nothing has run yet.');
		view.destroy();
	});

	it('closes from its own header', async () => {
		await commandLog.show();
		const view = render(CommandLog, {});

		click(view.get('.close'));

		expect(commandLog.open).toBe(false);
		view.destroy();
	});
});
