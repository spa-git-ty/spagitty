// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Fetching and pushing while they run (FEAT-018).
 *
 * Three things are worth pinning here. Pruning is read from the settings in one
 * place, so no caller can quietly prune when the setting says not to. The
 * progress line is git's own words rather than something invented. And a
 * finished worker is released, because the worker cannot let go of itself —
 * dropping it joins its own thread — so a leak here would refuse every
 * subsequent fetch with "already running".
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NETWORK_DONE_EVENT, NETWORK_PROGRESS_EVENT } from '$lib/types';

/** Handlers registered by `network.attach`, keyed by event name. */
const handlers = new Map<string, (event: { payload: unknown }) => void>();

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((name: string, handler: (event: { payload: unknown }) => void) => {
		handlers.set(name, handler);
		return Promise.resolve(vi.fn());
	})
}));

vi.mock('$lib/api', () => ({
	inTauri: () => true,
	fetch: vi.fn(),
	push: vi.fn(),
	networkRelease: vi.fn(() => Promise.resolve())
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

let pruneOnFetch = false;
vi.mock('$lib/settings/store.svelte', () => ({
	settings: {
		get settings() {
			return { confirmHistoryRewrite: true, showGitCommands: false, pruneOnFetch };
		}
	}
}));

const ok = vi.fn();
const failed = vi.fn();
vi.mock('$lib/ui/notice.svelte', () => ({
	notice: {
		ok: (title: string, detail?: string | null) => ok(title, detail),
		failed: (title: string, error: unknown) => failed(title, error)
	}
}));

import * as api from '$lib/api';
import { calls as repoCalls, control as repoControl } from '../../testing/repo-store.svelte';
import { network } from './store.svelte';

const fetchCall = vi.mocked(api.fetch);
const pushCall = vi.mocked(api.push);
const release = vi.mocked(api.networkRelease);

function emit(name: string, payload: unknown): void {
	handlers.get(name)?.({ payload });
}

/** Finish whatever is running, the way the application would. */
function finish(overrides: Record<string, unknown> = {}): void {
	emit(NETWORK_DONE_EVENT, {
		token: 1,
		operation: network.operation ?? 'fetch',
		ok: true,
		error: null,
		summary: null,
		...overrides
	});
}

beforeEach(async () => {
	vi.clearAllMocks();
	pruneOnFetch = false;
	repoControl.reset();
	handlers.clear();
	await network.attach();
	fetchCall.mockResolvedValue(1);
	pushCall.mockResolvedValue(1);
	// The store deliberately keeps a running token across `clear()`, so a test
	// that started one finishes it the way the application would.
	if (network.running) finish();
	network.clear();
});

describe('starting', () => {
	it('resolves when it has started, not when it has finished', async () => {
		expect(await network.fetch()).toBe(true);

		expect(network.running).toBe(true);
		expect(network.summary).toBeNull();
	});

	it('refuses a second operation while one is running', async () => {
		await network.fetch();
		pushCall.mockClear();

		expect(await network.push()).toBe(false);
		expect(pushCall).not.toHaveBeenCalled();
	});

	it('reports a start that failed at all', async () => {
		fetchCall.mockRejectedValueOnce(new Error('no repository is open'));

		expect(await network.fetch()).toBe(false);
		expect(network.running).toBe(false);
		expect(network.error).toContain('no repository is open');
	});
});

describe('pruning', () => {
	it('is off unless the setting says otherwise', async () => {
		await network.fetch();

		expect(fetchCall).toHaveBeenCalledWith('', false);
	});

	it('follows the setting, in one place for every caller', async () => {
		// The whole point of reading it here: a button that passed its own
		// value could prune when the setting said not to.
		pruneOnFetch = true;
		await network.fetch();

		expect(fetchCall).toHaveBeenCalledWith('', true);
	});

	it('carries a named remote through', async () => {
		await network.fetch('origin');

		expect(fetchCall).toHaveBeenCalledWith('origin', false);
	});
});

describe('while it runs', () => {
	it('says something before git has said anything', async () => {
		await network.push();

		expect(network.label).toBe('Pushing…');
	});

	it('shows git’s phase and percentage', async () => {
		await network.fetch();

		emit(NETWORK_PROGRESS_EVENT, {
			token: 1,
			operation: 'fetch',
			phase: 'Receiving objects',
			percent: 42,
			line: 'Receiving objects: 42% (420/1000)'
		});

		expect(network.label).toBe('Receiving objects 42%');
	});

	it('falls back to git’s own line when there is no percentage', async () => {
		// "Enumerating objects" has no number and is still worth showing: it
		// says the thing is alive.
		await network.fetch();

		emit(NETWORK_PROGRESS_EVENT, {
			token: 1,
			operation: 'fetch',
			phase: 'Enumerating objects',
			percent: null,
			line: 'Enumerating objects: 1200, done.'
		});

		expect(network.label).toBe('Enumerating objects: 1200, done.');
	});

	it('ignores progress from an operation that is not the one running', async () => {
		await network.fetch();

		emit(NETWORK_PROGRESS_EVENT, {
			token: 99,
			operation: 'fetch',
			phase: 'Receiving objects',
			percent: 90,
			line: 'x'
		});

		expect(network.step).toBeNull();
	});
});

describe('waiting for it to end (BUG-076)', () => {
	it('says how a push went once git has said so, not when it started', async () => {
		await network.push('origin', 'pr-61:feat/thing');
		let ended: boolean | null = null;
		void network.whenDone().then((ok) => (ended = ok));
		await Promise.resolve();
		expect(ended).toBeNull();

		finish();
		await Promise.resolve();

		expect(ended).toBe(true);
	});

	it('says a rejected push did not go well, with git’s words kept', async () => {
		await network.push('origin', 'pr-61:feat/thing');
		const waited = network.whenDone();

		finish({ ok: false, error: '! [rejected] pr-61 -> feat/thing (fetch first)' });

		expect(await waited).toBe(false);
		expect(network.error).toContain('rejected');
	});

	it('answers at once when nothing is running', async () => {
		expect(await network.whenDone()).toBe(true);
	});
});

describe('when it finishes', () => {
	it('releases the worker, so the next one may start', async () => {
		// The worker cannot let go of itself; a leak here would refuse every
		// later fetch with "already running".
		await network.fetch();
		finish();

		expect(release).toHaveBeenCalled();
		expect(network.running).toBe(false);
	});

	it('keeps git’s last words and re-reads the repository', async () => {
		await network.fetch();
		finish({ summary: 'Everything up-to-date' });

		expect(network.summary).toBe('Everything up-to-date');
		expect(ok).toHaveBeenCalledWith('Fetched', 'Everything up-to-date');
		// A fetch moves remote-tracking refs, which changes what every
		// divergence on screen means.
		expect(repoCalls.refreshed).toBeGreaterThan(0);
	});

	it('names the operation in the notice', async () => {
		await network.push();
		finish({ operation: 'push', summary: null });

		expect(ok).toHaveBeenCalledWith('Pushed', null);
	});

	it('reports a failure with git’s own message', async () => {
		await network.push();
		finish({ operation: 'push', ok: false, error: 'non-fast-forward', summary: null });

		expect(network.error).toBe('non-fast-forward');
		expect(network.summary).toBeNull();
		expect(failed).toHaveBeenCalledWith('Could not push', 'non-fast-forward');
	});

	it('ignores a done event for an operation that is not the one running', async () => {
		await network.fetch();
		emit(NETWORK_DONE_EVENT, {
			token: 99,
			operation: 'fetch',
			ok: true,
			error: null,
			summary: 'not ours'
		});

		expect(network.running).toBe(true);
		expect(network.summary).toBeNull();
	});
});
