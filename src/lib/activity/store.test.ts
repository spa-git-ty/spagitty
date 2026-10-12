// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What is running, and what has to wait for it (FEAT-121).
 *
 * The store decides two things the chrome then only draws: what to say, and
 * which actions would collide. The second is the one that matters when it is
 * wrong in either direction: too little, and two gits fight for `index.lock`;
 * too much, and a fetch freezes the whole toolbar, which is the full-screen
 * blocker by another route.
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { activity, ActivityBusy } from './store.svelte';

beforeEach(() => activity.reset());

describe('saying what is running', () => {
	it('is quiet when nothing is', () => {
		expect(activity.running).toBe(false);
		expect(activity.label).toBe('');
		expect(activity.blocked('repository')).toBeNull();
		expect(activity.blocked('network')).toBeNull();
	});

	it('names the operation, with the dots that mean it is still going', () => {
		const step = activity.begin('checkout', 'Switching to main');

		expect(activity.running).toBe(true);
		expect(activity.label).toBe('Switching to main…');
		expect(activity.is('checkout')).toBe(true);
		expect(activity.is('pull')).toBe(false);

		step?.end();
		expect(activity.running).toBe(false);
		expect(activity.label).toBe('');
	});

	it('moves on to the next part of the same operation', () => {
		const step = activity.begin('pull', 'Stashing your changes', ['repository', 'network']);

		step?.say('Pulling');
		expect(activity.label).toBe('Pulling…');
		step?.say('Putting your changes back');
		expect(activity.label).toBe('Putting your changes back…');
		expect(activity.is('pull')).toBe(true);
	});

	it("shows git's own progress in place of the name, and the name again without it", () => {
		const step = activity.begin('fetch', 'Fetching', ['network']);

		step?.progress('Receiving objects 45%');
		expect(activity.label).toBe('Receiving objects 45%');

		step?.progress(null);
		expect(activity.label).toBe('Fetching…');
	});

	it('drops the progress of the part before when the next part starts', () => {
		const step = activity.begin('pull', 'Pulling', ['repository', 'network']);
		step?.progress('Receiving objects 90%');

		step?.say('Putting your changes back');

		expect(activity.label).toBe('Putting your changes back…');
	});

	it('shows the newest when both lanes are taken', () => {
		activity.begin('fetch', 'Fetching', ['network']);
		activity.begin('stash', 'Stashing your changes', ['repository']);

		expect(activity.label).toBe('Stashing your changes…');
	});
});

describe('what has to wait', () => {
	it('holds the repository against a second write, and says what for', () => {
		activity.begin('stash', 'Stashing your changes');

		expect(activity.blocked('repository')).toBe('Stashing your changes is still running');
		expect(activity.begin('checkout', 'Switching to main')).toBeNull();
	});

	it('does not hold the network for a write to the repository', () => {
		// The point of lanes: a stash is no reason to refuse a fetch.
		activity.begin('stash', 'Stashing your changes');

		expect(activity.blocked('network')).toBeNull();
		expect(activity.begin('fetch', 'Fetching', ['network'])).not.toBeNull();
	});

	it('does not hold the repository for a fetch', () => {
		activity.begin('fetch', 'Fetching', ['network']);

		expect(activity.blocked('repository')).toBeNull();
		expect(activity.begin('stash', 'Stashing your changes')).not.toBeNull();
	});

	it('holds both for a pull, which fetches and then writes', () => {
		activity.begin('pull', 'Pulling', ['repository', 'network']);

		expect(activity.blocked('repository')).toBe('Pulling is still running');
		expect(activity.blocked('network')).toBe('Pulling is still running');
	});

	it('refuses a pull while either of its lanes is taken', () => {
		activity.begin('fetch', 'Fetching', ['network']);

		expect(activity.blocked('repository', 'network')).toBe('Fetching is still running');
		expect(activity.begin('pull', 'Pulling', ['repository', 'network'])).toBeNull();
	});

	it('lets the lane go when the operation ends, and only its own', () => {
		const fetching = activity.begin('fetch', 'Fetching', ['network']);
		const stashing = activity.begin('stash', 'Stashing your changes');

		stashing?.end();
		expect(activity.blocked('repository')).toBeNull();
		expect(activity.blocked('network')).toBe('Fetching is still running');

		fetching?.end();
		expect(activity.running).toBe(false);
	});

	it('ends once: a second end does not let go of somebody else', () => {
		const first = activity.begin('stash', 'Stashing your changes');
		first?.end();
		activity.begin('checkout', 'Switching to main');

		first?.end();

		expect(activity.blocked('repository')).toBe('Switching to main is still running');
	});

	it('names the sentence after the part running now', () => {
		const step = activity.begin('pull', 'Stashing your changes', ['repository', 'network']);
		step?.say('Pulling');

		expect(activity.blocked('repository')).toBe('Pulling is still running');
	});
});

describe('run', () => {
	it('holds the lane for as long as the work takes, and no longer', async () => {
		let finish!: () => void;
		const work = new Promise<void>((resolve) => (finish = resolve));

		const running = activity.run('stash', 'Stashing your changes', () => work);
		expect(activity.label).toBe('Stashing your changes…');

		finish();
		await running;
		expect(activity.running).toBe(false);
	});

	it('answers what the work answered', async () => {
		await expect(activity.run('other', 'Reading', async () => 7)).resolves.toBe(7);
	});

	it('lets go when the work fails, and passes the failure on', async () => {
		await expect(
			activity.run('checkout', 'Switching to main', async () => {
				throw 'your local changes would be overwritten';
			})
		).rejects.toBe('your local changes would be overwritten');

		expect(activity.running).toBe(false);
	});

	it('runs nothing while the lane is taken, and says what is in the way', async () => {
		activity.begin('pull', 'Pulling', ['repository', 'network']);
		let ran = false;

		const refused = activity.run('stash', 'Stashing your changes', async () => {
			ran = true;
		});

		await expect(refused).rejects.toBeInstanceOf(ActivityBusy);
		expect(ran).toBe(false);
		// Stores show a failure as `String(error)`: the sentence, and no class name.
		await refused.catch((error) => expect(String(error)).toBe('Pulling is still running'));
	});
});
