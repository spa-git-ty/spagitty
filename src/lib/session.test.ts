// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { resumeSession, type ResumePort } from './session';
import type { Place } from './workspace.svelte';

/**
 * A port that writes down what it was asked to do, in order.
 *
 * The bug this module exists for was a call that never happened, so the log is
 * the assertion: a test that only checked a return value would have passed
 * against the broken shell.
 */
function recorder(
	overrides: {
		launch?: string | null;
		active?: string | null;
		places?: Record<string, Place>;
		route?: string;
		opens?: (path: string) => boolean;
		cancelAfter?: number;
	} = {}
) {
	const calls: string[] = [];
	let checks = 0;

	const port: ResumePort = {
		async launchPath() {
			calls.push('launchPath');
			return overrides.launch ?? null;
		},
		async open(path) {
			calls.push(`open ${path}`);
			return overrides.opens ? overrides.opens(path) : true;
		},
		active() {
			calls.push('active');
			return overrides.active ?? null;
		},
		placeOf(path) {
			calls.push(`placeOf ${path}`);
			return overrides.places?.[path] ?? null;
		},
		route() {
			return overrides.route ?? '/';
		},
		async goto(route) {
			calls.push(`goto ${route}`);
		},
		want(id) {
			calls.push(`want ${id}`);
		},
		cancelled() {
			checks += 1;
			return overrides.cancelAfter !== undefined && checks > overrides.cancelAfter;
		}
	};

	return { port, calls };
}

describe('a path on the command line', () => {
	it('wins over whatever the last session was doing', async () => {
		const { port, calls } = recorder({
			launch: '/tmp/asked-for',
			active: '/tmp/last-session'
		});

		await resumeSession(port);

		expect(calls).toEqual(['launchPath', 'open /tmp/asked-for', 'goto /home']);
	});

	it('lands on Home, and wants no commit, because the launch has no stored place', async () => {
		const { port, calls } = recorder({
			launch: '/tmp/asked-for',
			places: { '/tmp/asked-for': { route: '/branches', selected: 'abc' } }
		});

		await resumeSession(port);

		expect(calls.filter((call) => call.startsWith('goto'))).toEqual(['goto /home']);
		expect(calls.some((call) => call.startsWith('want'))).toBe(false);
	});
});

describe('the last session', () => {
	it('opens the tab the strip came back with — the call BUG-013 was missing', async () => {
		const { port, calls } = recorder({ active: '/tmp/last-session' });

		await resumeSession(port);

		expect(calls).toContain('open /tmp/last-session');
	});

	it('lands on Home, with the commit it was left on (FEAT-140)', async () => {
		const { port, calls } = recorder({
			active: '/tmp/last-session',
			places: { '/tmp/last-session': { route: '/branches', selected: 'c0ffee' } }
		});

		await resumeSession(port);

		expect(calls).toEqual([
			'launchPath',
			'active',
			'placeOf /tmp/last-session',
			'open /tmp/last-session',
			'goto /home',
			'want c0ffee'
		]);
	});

	it('reads the place before the open, so an open that changes it cannot win', async () => {
		const { port, calls } = recorder({
			active: '/tmp/last-session',
			places: { '/tmp/last-session': { route: '/branches', selected: null } }
		});

		await resumeSession(port);

		expect(calls.indexOf('placeOf /tmp/last-session')).toBeLessThan(
			calls.indexOf('open /tmp/last-session')
		);
	});

	it('stays put when Home is already on screen', async () => {
		const { port, calls } = recorder({
			active: '/tmp/last-session',
			route: '/home',
			places: { '/tmp/last-session': { route: '/branches', selected: null } }
		});

		await resumeSession(port);

		expect(calls.some((call) => call.startsWith('goto'))).toBe(false);
	});

	it('wants nothing when the tab had no commit selected', async () => {
		const { port, calls } = recorder({
			active: '/tmp/last-session',
			places: { '/tmp/last-session': { route: '/branches', selected: null } }
		});

		await resumeSession(port);

		expect(calls.some((call) => call.startsWith('want'))).toBe(false);
	});

	it('opens the tab even when no place was stored for it', async () => {
		const { port, calls } = recorder({ active: '/tmp/last-session' });

		await resumeSession(port);

		expect(calls).toEqual([
			'launchPath',
			'active',
			'placeOf /tmp/last-session',
			'open /tmp/last-session',
			'goto /home'
		]);
	});
});

describe('a tab that no longer resolves', () => {
	it('lands on Home, where the failure is said, and wants nothing', async () => {
		const { port, calls } = recorder({
			active: '/tmp/moved',
			places: { '/tmp/moved': { route: '/branches', selected: 'c0ffee' } },
			opens: () => false
		});

		await resumeSession(port);

		expect(calls).toEqual(['launchPath', 'active', 'placeOf /tmp/moved', 'open /tmp/moved', 'goto /home']);
	});
});

describe('an empty launch', () => {
	it('opens nothing when there is no path and no tab, and lands on Home', async () => {
		const { port, calls } = recorder();

		await resumeSession(port);

		expect(calls).toEqual(['launchPath', 'active', 'goto /home']);
	});
});

describe('a shell torn down mid-resume', () => {
	it('never asks for a launch path once it is already gone', async () => {
		const { port, calls } = recorder({ launch: '/tmp/asked-for', cancelAfter: 0 });

		await resumeSession(port);

		expect(calls).toEqual([]);
	});

	it('does not open a tab when the shell went away while the path was read', async () => {
		const { port, calls } = recorder({ active: '/tmp/last-session', cancelAfter: 1 });

		await resumeSession(port);

		expect(calls).toEqual(['launchPath', 'active']);
	});

	it('does not navigate when the shell went away while the repository opened', async () => {
		const { port, calls } = recorder({
			active: '/tmp/last-session',
			places: { '/tmp/last-session': { route: '/branches', selected: 'c0ffee' } },
			cancelAfter: 2
		});

		await resumeSession(port);

		expect(calls).toEqual([
			'launchPath',
			'active',
			'placeOf /tmp/last-session',
			'open /tmp/last-session'
		]);
	});
});
