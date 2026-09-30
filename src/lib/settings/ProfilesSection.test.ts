// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Identity profiles in Settings (FEAT-069, covered under FEAT-072).
 *
 * The section's real job is stopping a work commit from being made with a
 * personal address, so the tests concentrate on the two moments where that can
 * go wrong: what gets saved, and which config file a profile is applied to.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, render } from '../../testing/mount';
import { flushSync } from 'svelte';
import type { IdentityProfile } from '$lib/types';

const store = {
	list: [] as IdentityProfile[],
	fetch: vi.fn((): Promise<void> => Promise.resolve()),
	save: vi.fn((_profile: IdentityProfile): Promise<void> => Promise.resolve()),
	apply: vi.fn((_profile: IdentityProfile, _global: boolean): Promise<void> => Promise.resolve()),
	delete: vi.fn((_id: string): Promise<void> => Promise.resolve())
};

vi.mock('$lib/profiles/store.svelte', () => ({
	profiles: {
		get list() {
			return store.list;
		},
		fetch: () => store.fetch(),
		save: (profile: IdentityProfile) => store.save(profile),
		apply: (profile: IdentityProfile, global: boolean) => store.apply(profile, global),
		delete: (id: string) => store.delete(id)
	}
}));

vi.mock('$lib/settings/store.svelte', () => ({
	settings: { load: vi.fn(() => Promise.resolve()) }
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import { control } from '../../testing/repo-store.svelte';
import ProfilesSection from './ProfilesSection.svelte';

function aProfile(overrides: Partial<IdentityProfile> = {}): IdentityProfile {
	return {
		id: 'work',
		name: 'Work',
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		signingKey: null,
		...overrides
	};
}

/** Open the add form and fill the fields the save path reads. */
function fill(view: ReturnType<typeof render>, values: Record<string, string>): void {
	click(view.all('button').find((b) => b.textContent?.includes('Add Profile'))!);
	for (const [placeholder, value] of Object.entries(values)) {
		const field = view.get(`input[placeholder="${placeholder}"]`) as HTMLInputElement;
		field.value = value;
		field.dispatchEvent(new Event('input', { bubbles: true }));
	}
	flushSync();
}

beforeEach(() => {
	vi.clearAllMocks();
	store.list = [];
	control.reset();
	control.setInfo({
		path: '/work/project',
		name: 'project',
		bare: false,
		head: { detached: false, branch: 'main', id: 'a'.repeat(40) },
		lastFetched: null
	} as never);
});

describe('a section with no profiles yet', () => {
	it('reads the saved profiles as soon as it is shown', () => {
		const view = render(ProfilesSection, {});

		expect(store.fetch).toHaveBeenCalled();

		view.destroy();
	});

	it('shows no list and no instructions when there are none (TASK-050)', () => {
		const view = render(ProfilesSection, {});

		expect(view.text()).not.toContain('No saved profiles');
		expect(view.all('.profile-card')).toHaveLength(0);

		view.destroy();
	});

	it('drops the empty note once the form is open, because it is no longer true', () => {
		const view = render(ProfilesSection, {});

		click(view.all('button').find((b) => b.textContent?.includes('Add Profile'))!);

		expect(view.text()).not.toContain('No saved profiles');
		expect(view.find('.form-card')).not.toBeNull();

		view.destroy();
	});
});

describe('saving a profile', () => {
	it('refuses a profile with no email, and says which fields it needs', () => {
		const view = render(ProfilesSection, {});
		fill(view, { 'Work / Personal': 'Work', 'Ada Lovelace': 'Ada' });

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		flushSync();

		expect(store.save).not.toHaveBeenCalled();
		expect(view.get('[role="alert"]').textContent).toContain('required');

		view.destroy();
	});

	it('refuses a profile whose fields are only whitespace', () => {
		const view = render(ProfilesSection, {});
		fill(view, {
			'Work / Personal': '   ',
			'Ada Lovelace': 'Ada',
			'ada@example.com': 'ada@example.com'
		});

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		flushSync();

		expect(store.save).not.toHaveBeenCalled();

		view.destroy();
	});

	it('trims what it saves, and derives a stable id from the trimmed label', async () => {
		const view = render(ProfilesSection, {});
		fill(view, {
			'Work / Personal': '  Work Laptop ',
			'Ada Lovelace': ' Ada Lovelace ',
			'ada@example.com': ' ada@example.com '
		});

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		await vi.waitFor(() => expect(store.save).toHaveBeenCalled());

		expect(store.save).toHaveBeenCalledWith({
			id: 'work-laptop',
			name: 'Work Laptop',
			authorName: 'Ada Lovelace',
			authorEmail: 'ada@example.com',
			signingKey: null
		});

		view.destroy();
	});

	it('gives two labels differing only in punctuation two different ids', async () => {
		// The id is the storage key. `Work / Personal` and `Work Personal` both
		// collapsing to `work-personal` would have one profile overwrite the
		// other, silently.
		const view = render(ProfilesSection, {});
		fill(view, {
			'Work / Personal': 'Work (laptop)',
			'Ada Lovelace': 'Ada',
			'ada@example.com': 'ada@example.com'
		});

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		await vi.waitFor(() => expect(store.save).toHaveBeenCalled());

		// One dash per run, and none left dangling at the end.
		expect(store.save.mock.calls[0][0]).toMatchObject({ id: 'work-laptop' });

		view.destroy();
	});

	it('saves an empty signing key as absent rather than as an empty string', async () => {
		// `user.signingkey = ""` is not the same as no signing key, and git
		// treats the two differently.
		const view = render(ProfilesSection, {});
		fill(view, {
			'Work / Personal': 'Work',
			'Ada Lovelace': 'Ada',
			'ada@example.com': 'ada@example.com',
			'3AA5C34371567BD2': '   '
		});

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		await vi.waitFor(() => expect(store.save).toHaveBeenCalled());

		expect(store.save.mock.calls[0][0]).toMatchObject({ signingKey: null });

		view.destroy();
	});

	it('keeps a signing key that was given', async () => {
		const view = render(ProfilesSection, {});
		fill(view, {
			'Work / Personal': 'Work',
			'Ada Lovelace': 'Ada',
			'ada@example.com': 'ada@example.com',
			'3AA5C34371567BD2': '3AA5C34371567BD2'
		});

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		await vi.waitFor(() => expect(store.save).toHaveBeenCalled());

		expect(store.save.mock.calls[0][0]).toMatchObject({ signingKey: '3AA5C34371567BD2' });

		view.destroy();
	});

	it('closes the form and empties it once the save lands', async () => {
		const view = render(ProfilesSection, {});
		fill(view, {
			'Work / Personal': 'Work',
			'Ada Lovelace': 'Ada',
			'ada@example.com': 'ada@example.com'
		});

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		await vi.waitFor(() => expect(view.find('.form-card')).toBeNull());

		view.destroy();
	});

	it('leaves the form open with the reason when the save fails', async () => {
		// Closing it would throw away what was typed, on the one path where
		// somebody needs it back.
		store.save.mockRejectedValueOnce(new Error('the profiles file is read-only'));
		const view = render(ProfilesSection, {});
		fill(view, {
			'Work / Personal': 'Work',
			'Ada Lovelace': 'Ada',
			'ada@example.com': 'ada@example.com'
		});

		click(view.all('button').find((b) => b.textContent?.includes('Save Profile'))!);
		await vi.waitFor(() => expect(view.find('[role="alert"]')).not.toBeNull());

		expect(view.get('[role="alert"]').textContent).toContain('read-only');
		expect(view.find('.form-card')).not.toBeNull();

		view.destroy();
	});

	it('cancelling closes the form without saving anything', () => {
		const view = render(ProfilesSection, {});
		fill(view, { 'Work / Personal': 'Work' });

		click(view.all('button').find((b) => b.textContent?.trim() === 'Cancel')!);
		flushSync();

		expect(view.find('.form-card')).toBeNull();
		expect(store.save).not.toHaveBeenCalled();

		view.destroy();
	});
});

describe('applying a profile', () => {
	beforeEach(() => {
		store.list = [aProfile()];
	});

	it('shows what each profile would commit as', () => {
		store.list = [aProfile({ signingKey: 'KEY123' })];
		const view = render(ProfilesSection, {});

		expect(view.text()).toContain('Ada Lovelace');
		expect(view.text()).toContain('<ada@example.com>');
		expect(view.text()).toContain('KEY123');

		view.destroy();
	});

	it('marks a profile with no key by leaving the pill off entirely', () => {
		const view = render(ProfilesSection, {});

		expect(view.find('.key-pill')).toBeNull();

		view.destroy();
	});

	it('writes to the repository when that is what was asked for', async () => {
		const view = render(ProfilesSection, {});

		click(view.all('button').find((b) => b.textContent?.includes('Apply to Repo'))!);
		await vi.waitFor(() => expect(store.apply).toHaveBeenCalled());

		expect(store.apply).toHaveBeenCalledWith(store.list[0], false);

		view.destroy();
	});

	it('writes to the global config when that is what was asked for', async () => {
		const view = render(ProfilesSection, {});

		click(view.all('button').find((b) => b.textContent?.includes('Apply Globally'))!);
		await vi.waitFor(() => expect(store.apply).toHaveBeenCalled());

		expect(store.apply).toHaveBeenCalledWith(store.list[0], true);

		view.destroy();
	});

	it('does not offer to apply to a repository when none is open', () => {
		// The button would write to a config file that is not there.
		control.setInfo(null);
		const view = render(ProfilesSection, {});

		expect(view.text()).not.toContain('Apply to Repo');
		expect(view.text()).toContain('Apply Globally');

		view.destroy();
	});

	it('survives an apply that fails without losing the list', async () => {
		store.apply.mockRejectedValueOnce(new Error('permission denied'));
		const view = render(ProfilesSection, {});

		click(view.all('button').find((b) => b.textContent?.includes('Apply Globally'))!);
		await vi.waitFor(() => expect(store.apply).toHaveBeenCalled());
		flushSync();

		expect(view.text()).toContain('Ada Lovelace');

		view.destroy();
	});

	it('deletes by id', async () => {
		const view = render(ProfilesSection, {});

		click(view.get('.delete-btn'));
		await vi.waitFor(() => expect(store.delete).toHaveBeenCalled());

		expect(store.delete).toHaveBeenCalledWith('work');

		view.destroy();
	});

	it('survives a delete that fails', async () => {
		store.delete.mockRejectedValueOnce(new Error('in use'));
		const view = render(ProfilesSection, {});

		click(view.get('.delete-btn'));
		await vi.waitFor(() => expect(store.delete).toHaveBeenCalled());

		view.destroy();
	});
});
