// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
vi.mock('$lib/api', () => ({ watchPullRequests: vi.fn(async () => []), notifyDesktop: vi.fn() }));
vi.mock('$lib/settings/store.svelte', () => ({ settings: { settings: {} } }));
vi.mock('$lib/delight/sound', () => ({ play: vi.fn() }));
import { watcher } from './watcher.svelte';
import * as api from '$lib/api';

const clean: (() => void)[] = [];
beforeEach(() => { vi.useFakeTimers(); vi.clearAllMocks(); });
afterEach(() => { clean.splice(0).forEach((stop) => stop()); watcher.stop(); vi.useRealTimers(); vi.restoreAllMocks(); });

it('shares one clock and keeps screen refresh alive when notifications stop', async () => {
	const fast = vi.fn(), slow = vi.fn();
	clean.push(watcher.subscribe(fast, 5000), watcher.subscribe(slow));
	watcher.start(1); await vi.advanceTimersByTimeAsync(10_000);
	expect(vi.getTimerCount()).toBe(1); expect(fast).toHaveBeenCalledTimes(2); expect(slow).not.toHaveBeenCalled();
	watcher.stop(); await vi.advanceTimersByTimeAsync(50_000);
	expect(slow).toHaveBeenCalledTimes(1); expect(api.watchPullRequests).toHaveBeenCalledTimes(1);
});

it('does not read hidden screens and resumes on the next visible tick', async () => {
	const visibility = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
	const read = vi.fn(); clean.push(watcher.subscribe(read, 5000));
	await vi.advanceTimersByTimeAsync(20_000); expect(read).not.toHaveBeenCalled();
	visibility.mockReturnValue('visible'); await vi.advanceTimersByTimeAsync(5000); expect(read).toHaveBeenCalledOnce();
});

it('a failed subscriber does not stop another screen or later ticks', async () => {
	const read = vi.fn(); clean.push(watcher.subscribe(() => { throw new Error('offline'); }, 5000), watcher.subscribe(read, 5000));
	await vi.advanceTimersByTimeAsync(10_000); expect(read).toHaveBeenCalledTimes(2);
});
