// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The delight store (FEAT-072).
 *
 * Three things here are worth holding onto, and none of them is "the counter
 * went up" — the engine's own suite covers that.
 *
 * **It must never break a git operation.** Every caller is a line after a
 * successful write, so `record` swallowing its own failure is the single most
 * important behaviour in the file.
 *
 * **The record is per repository.** Badges earned in one codebase must not
 * follow the user into another, because the numbers only mean anything in the
 * repository they were earned in.
 *
 * **Personality changes the volume, never the record.** A Professional install
 * still earns everything; what changes is whether a card appears.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { delight } from './store.svelte';
import { useContextFactory } from './sound';
import { settings } from '$lib/settings/store.svelte';
import { notice } from '$lib/ui/notice.svelte';
import type { DelightEvent } from './events';

const COMMIT: DelightEvent = {
	kind: 'commit',
	files: 2,
	directories: 1,
	added: 20,
	removed: 4,
	onDefaultBranch: false,
	amend: false,
	tests: false,
	refactor: false
};

/** Put the personality where a test needs it, without touching the backend. */
function personality(level: 'off' | 'professional' | 'balanced' | 'fullSpagitty'): void {
	settings.settings.personality = level;
}

beforeEach(() => {
	localStorage.clear();
	delight.clear();
	notice.dismiss();
	personality('balanced');
	delight.bind('/repos/one');
});

afterEach(() => {
	delight.clear();
	localStorage.clear();
});

describe('recording', () => {
	it('counts an event against the person at the keyboard', () => {
		delight.record(COMMIT);

		expect(delight.me.stats.commits).toBe(1);
	});

	it('credits an agent when one is named', () => {
		delight.record(COMMIT, { id: 'claude', kind: 'claude', name: 'Claude' });

		expect(delight.me.stats.commits).toBe(0);
		expect(delight.get('claude')?.stats.commits).toBe(1);
	});

	it('never throws, whatever it is handed', () => {
		// The guarantee the whole feature rests on: this call sits on the line
		// after a commit succeeded.
		expect(() =>
			delight.record({ kind: 'nonsense' } as unknown as DelightEvent)
		).not.toThrow();
	});

	it('survives storage that refuses to be written', () => {
		const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
			throw new Error('quota');
		});

		expect(() => delight.record(COMMIT)).not.toThrow();
		expect(delight.me.stats.commits, 'the count still happened').toBe(1);

		setItem.mockRestore();
	});
});

describe('the record follows the repository', () => {
	it('reads back what was earned here', () => {
		delight.record(COMMIT);
		delight.bind(null);
		delight.bind('/repos/one');

		expect(delight.me.stats.commits).toBe(1);
	});

	it('starts empty in a different repository', () => {
		delight.record(COMMIT);
		delight.bind('/repos/two');

		expect(delight.me.stats.commits).toBe(0);
	});

	it('does not reload when it is bound to the repository it is already on', () => {
		delight.record(COMMIT);
		delight.bind('/repos/one');

		expect(delight.me.stats.commits).toBe(1);
	});

	it('ignores a stored record it cannot read', () => {
		localStorage.setItem('spagitty.delight:/repos/three', 'not json');
		delight.bind('/repos/three');

		expect(delight.list).toEqual([]);
	});

	it('ignores a stored record from a version it does not know', () => {
		localStorage.setItem(
			'spagitty.delight:/repos/four',
			JSON.stringify({ version: 99, actors: { you: {} } })
		);
		delight.bind('/repos/four');

		expect(delight.list).toEqual([]);
	});
});

describe('identity', () => {
	it('keys the record on the git email, so a profile switch switches record', () => {
		delight.identify('Ada Lovelace', 'ada@example.com');
		delight.record(COMMIT);

		delight.identify('Ada at work', 'ada@work.example');
		expect(delight.me.stats.commits).toBe(0);

		delight.identify('Ada Lovelace', 'ada@example.com');
		expect(delight.me.stats.commits).toBe(1);
	});

	it('falls back to an anonymous actor when git has no email', () => {
		delight.identify(null, null);
		delight.record(COMMIT);

		expect(delight.me.name).toBe('You');
	});
});

describe('personality', () => {
	/** The default (TASK-045). */
	it('shows nothing at all at Off, and still earns the badge', () => {
		personality('off');
		const pulse = delight.pulse;
		delight.record(COMMIT);

		expect(delight.on).toBe(false);
		expect(delight.waiting).toBe(0);
		expect(notice.current).toBeNull();
		expect(delight.pulse).toBe(pulse);
		expect(delight.me.earned.map((entry) => entry.id)).toContain('cook');
	});

	it('keeps a shame badge quiet at Off too', () => {
		personality('off');
		delight.record({ ...COMMIT, onDefaultBranch: true });

		expect(delight.showsShame).toBe(false);
		expect(notice.current).toBeNull();
	});

	it('queues a reward moment at Balanced', () => {
		delight.record(COMMIT);

		expect(delight.waiting).toBeGreaterThan(0);
	});

	it('says it in the corner instead at Professional', () => {
		personality('professional');
		delight.record(COMMIT);

		expect(delight.waiting).toBe(0);
		expect(notice.current?.title).toContain('Cook');
	});

	it('earns the badge at Professional all the same', () => {
		personality('professional');
		delight.record(COMMIT);

		expect(delight.me.earned.map((entry) => entry.id)).toContain('cook');
	});

	it('never gives a shame badge a reward moment', () => {
		personality('fullSpagitty');
		delight.record({ ...COMMIT, onDefaultBranch: true });

		expect(delight.me.earned.map((entry) => entry.id)).toContain('main-character');
		expect(
			delight.waiting === 0 ||
				!delight.showing?.badge.shame
		).toBe(true);
		expect(notice.current?.title).toContain('Main Character');
	});

	it('keeps a shame badge quiet below Full Spagitty', () => {
		delight.record({ ...COMMIT, onDefaultBranch: true });

		expect(delight.me.earned.map((entry) => entry.id)).toContain('main-character');
		expect(notice.current).toBeNull();
	});

	it('hides the Hall of Shame at Professional only', () => {
		personality('professional');
		expect(delight.showsShame).toBe(false);
		personality('balanced');
		expect(delight.showsShame).toBe(true);
	});
});

describe('titles', () => {
	beforeEach(() => {
		delight.record(COMMIT);
	});

	it('equips a badge that has been earned', () => {
		delight.equip(delight.me.id, 'cook');

		expect(delight.title(delight.me.id)?.id).toBe('cook');
	});

	it('refuses one that has not', () => {
		delight.equip(delight.me.id, 'git-lord');

		expect(delight.title(delight.me.id)).toBeNull();
	});

	it('refuses a shame badge, because the Hall of Shame is not a wardrobe', () => {
		delight.record({ ...COMMIT, onDefaultBranch: true });
		delight.equip(delight.me.id, 'main-character');

		expect(delight.title(delight.me.id)).toBeNull();
	});

	it('takes a title off again', () => {
		delight.equip(delight.me.id, 'cook');
		delight.equip(delight.me.id, null);

		expect(delight.title(delight.me.id)).toBeNull();
	});
});

describe('the queue', () => {
	it('hands out one unlock at a time, oldest first', () => {
		delight.record(COMMIT);
		const waiting = delight.waiting;
		expect(waiting).toBeGreaterThan(0);

		delight.advance();
		expect(delight.showing).not.toBeNull();
		expect(delight.waiting).toBe(waiting - 1);

		delight.dismiss();
		expect(delight.showing).toBeNull();
	});
});

describe('forgetting', () => {
	it('empties the record and the storage behind it', () => {
		delight.record(COMMIT);
		delight.forget();

		expect(delight.list).toEqual([]);
		expect(localStorage.getItem('spagitty.delight:/repos/one')).toBeNull();
	});
});

describe('sound', () => {
	afterEach(() => {
		settings.settings.sound = 'off';
		useContextFactory(() => null);
	});

	it('is its own setting: Off personality with sound chosen still plays the cue', () => {
		// The personality decides what is shown, never what is heard.
		const asked = vi.fn(() => null);
		useContextFactory(asked);
		personality('off');
		settings.settings.sound = 'full';

		delight.record(COMMIT);

		expect(asked).toHaveBeenCalled();
	});

	it('stays silent when sound is off, whatever the personality', () => {
		const asked = vi.fn(() => null);
		useContextFactory(asked);
		personality('fullSpagitty');
		settings.settings.sound = 'off';

		delight.record(COMMIT);

		expect(asked).not.toHaveBeenCalled();
	});
});
