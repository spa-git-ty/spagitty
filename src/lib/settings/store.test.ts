// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Identity, IdentityValue, Licenses, Settings, Update } from '$lib/types';

vi.mock('$lib/api', () => ({
	inTauri: vi.fn(() => true),
	identity: vi.fn(),
	setIdentity: vi.fn(),
	settings: vi.fn(),
	setSettings: vi.fn(() => Promise.resolve()),
	signing: vi.fn(),
	setSigning: vi.fn(),
	clearSigning: vi.fn(),
	forgeAccounts: vi.fn(() => Promise.resolve([])),
	checkUpdate: vi.fn(),
	forgeConnect: vi.fn(),
	forgeDisconnect: vi.fn(),
	licenses: vi.fn(),
	about: vi.fn()
}));

import * as api from '$lib/api';
import { settings } from './store.svelte';

const identity = vi.mocked(api.identity);
const setIdentity = vi.mocked(api.setIdentity);
const settingsCall = vi.mocked(api.settings);
const checkUpdate = vi.mocked(api.checkUpdate);
const signingCall = vi.mocked(api.signing);
const setSettings = vi.mocked(api.setSettings);
const licenses = vi.mocked(api.licenses);
const about = vi.mocked(api.about);
const inTauri = vi.mocked(api.inTauri);

function value(overrides: Partial<IdentityValue> = {}): IdentityValue {
	return { effective: null, origin: 'unset', global: null, local: null, ...overrides };
}

function anIdentity(overrides: Partial<Identity> = {}): Identity {
	return {
		name: value({ effective: 'Ada Lovelace', origin: 'global', global: 'Ada Lovelace' }),
		email: value({ effective: 'ada@example.com', origin: 'global', global: 'ada@example.com' }),
		repository: true,
		...overrides
	};
}

const STORED: Settings = {
	checkForUpdates: true,
	confirmHistoryRewrite: true,
	showGitCommands: false, pruneOnFetch: false,
fetchAvatars: false,
			personality: 'balanced',
			sound: 'off'
};

const LIST: Licenses = {
	generated: true,
	notes: [],
	rust: [{ name: 'gix', version: '0.86.0', license: 'MIT OR Apache-2.0' }],
	npm: [{ name: '@tauri-apps/api', version: '2.0.0', license: 'Apache-2.0 OR MIT' }]
};

beforeEach(() => {
	vi.clearAllMocks();
	settings.clearState();
	inTauri.mockReturnValue(true);
	identity.mockResolvedValue(anIdentity());
	settingsCall.mockResolvedValue(STORED);
	licenses.mockResolvedValue(LIST);
	about.mockResolvedValue({ version: '0.1.0', commit: 'abc1234', license: 'GPL-3.0-or-later' });
});

describe('load', () => {
	it('reads the identity, the toggles, the licenses and the build in one pass', async () => {
		await settings.load();

		expect(settings.identity).toEqual(anIdentity());
		expect(settings.settings).toEqual(STORED);
		expect(settings.licenses).toEqual(LIST);
		expect(settings.about?.commit).toBe('abc1234');
		expect(settings.loaded).toBe(true);
	});

	it('fills the fields from the scope being edited', async () => {
		await settings.load();

		expect(settings.draft('name')).toBe('Ada Lovelace');
		expect(settings.draft('email')).toBe('ada@example.com');
		expect(settings.isDirty('name')).toBe(false);
	});

	it('keeps About when the identity cannot be read, and says what failed', async () => {
		// The license and the commit are an obligation. A git configuration
		// Spagitty cannot parse must not take them off the screen.
		identity.mockRejectedValueOnce('could not read the git configuration: broken');
		await settings.load();

		expect(settings.error).toBe('could not read the git configuration: broken');
		expect(settings.about?.commit).toBe('abc1234');
		expect(settings.licenses).toEqual(LIST);
	});

	it('keeps the identity when the license list cannot be read', async () => {
		licenses.mockRejectedValueOnce('nope');
		await settings.load();

		expect(settings.identity).not.toBeNull();
		expect(settings.licenses).toBeNull();
		expect(settings.error).toBeNull();
	});

	it('asks for nothing outside the Tauri webview', async () => {
		inTauri.mockReturnValue(false);
		await settings.load();

		expect(identity).not.toHaveBeenCalled();
		expect(settings.loaded).toBe(true);
	});

	it('falls back to the global scope when no repository is open', async () => {
		identity.mockResolvedValue(anIdentity({ repository: false }));
		settings.setScope('global');
		await settings.load();

		expect(settings.canEditLocally).toBe(false);
		expect(settings.scope).toBe('global');
	});
});

describe('scope', () => {
	it('refuses the local scope when there is no repository to write to', async () => {
		identity.mockResolvedValue(anIdentity({ repository: false }));
		await settings.load();

		settings.setScope('local');

		expect(settings.scope).toBe('global');
	});

	it('refills the fields so a typed value cannot be saved into the other file', async () => {
		// The quiet mistake this screen exists to prevent: a name typed against
		// the global scope landing in a repository because a chip was flipped.
		identity.mockResolvedValue(
			anIdentity({
				email: value({
					effective: 'ada@work.example',
					origin: 'local',
					global: 'ada@example.com',
					local: 'ada@work.example'
				})
			})
		);
		await settings.load();
		settings.setDraft('email', 'typed@example.com');

		settings.setScope('local');

		expect(settings.draft('email')).toBe('ada@work.example');
		expect(settings.isDirty('email')).toBe(false);
	});

	it('shows an empty field for a scope that holds nothing', async () => {
		await settings.load();
		settings.setScope('local');

		expect(settings.draft('name')).toBe('');
	});
});

describe('saving an identity', () => {
	it('writes the field to the chosen scope and takes the result as the new state', async () => {
		await settings.load();
		const written = anIdentity({
			name: value({ effective: 'Grace Hopper', origin: 'global', global: 'Grace Hopper' })
		});
		setIdentity.mockResolvedValueOnce(written);

		settings.setDraft('name', 'Grace Hopper');
		await settings.save('name');

		expect(setIdentity).toHaveBeenCalledWith('global', 'name', 'Grace Hopper');
		expect(settings.identity).toEqual(written);
		expect(settings.draft('name')).toBe('Grace Hopper');
		expect(settings.isDirty('name')).toBe(false);
	});

	it('names the scope the user chose and never the other one', async () => {
		await settings.load();
		setIdentity.mockResolvedValueOnce(anIdentity());
		settings.setScope('local');

		settings.setDraft('email', 'ada@work.example');
		await settings.save('email');

		expect(setIdentity).toHaveBeenCalledWith('local', 'email', 'ada@work.example');
	});

	it('sends the empty string through, which is what unsets the key', async () => {
		await settings.load();
		setIdentity.mockResolvedValueOnce(anIdentity({ email: value() }));

		settings.clear('email');
		await settings.save('email');

		expect(setIdentity).toHaveBeenCalledWith('global', 'email', '');
	});

	it('records a failed write without losing what was typed', async () => {
		await settings.load();
		setIdentity.mockRejectedValueOnce('git config --global failed: read-only file system');

		settings.setDraft('name', 'Grace Hopper');
		await settings.save('name');

		expect(settings.writeError).toContain('read-only file system');
		expect(settings.draft('name')).toBe('Grace Hopper');
	});

	it('does not start a second write while one is running', async () => {
		await settings.load();
		let release!: (identity: Identity) => void;
		setIdentity.mockReturnValueOnce(new Promise((resolve) => (release = resolve)));

		const first = settings.save('name');
		await settings.save('email');
		release(anIdentity());
		await first;

		expect(setIdentity).toHaveBeenCalledTimes(1);
	});
});

describe('behaviour toggles', () => {
	it('flips and stores the whole settings object', async () => {
		await settings.load();

		await settings.toggle('showGitCommands');

		expect(settings.settings.showGitCommands).toBe(true);
		expect(setSettings).toHaveBeenCalledWith({ ...STORED, showGitCommands: true });
	});

	it('puts the switch back when the write fails, and says why', async () => {
		// A toggle showing one state while another is stored is worse than one
		// that visibly refuses.
		await settings.load();
		setSettings.mockRejectedValueOnce('there is no configuration directory to write to');

		await settings.toggle('confirmHistoryRewrite');

		expect(settings.settings.confirmHistoryRewrite).toBe(true);
		expect(settings.writeError).toContain('no configuration directory');
	});

	it('asking before a history rewrite is on before anything is stored', () => {
		expect(settings.settings.confirmHistoryRewrite).toBe(true);
		expect(settings.settings.showGitCommands).toBe(false);
	});
});

describe('sections', () => {
	it('opens on You', () => {
		expect(settings.section).toBe('you');
	});

	it('follows a fragment, which is how Pull requests links to Accounts', () => {
		// `accounts` is not a section any more — it is a block inside You, where
		// it had always been rendered. The two buttons on the Pull requests
		// screen still point at `#accounts`, so the fragment has to keep
		// arriving somewhere the accounts actually are.
		settings.showFromHash('#accounts');

		expect(settings.section).toBe('you');
	});

	it('ignores a fragment that names nothing, rather than blanking the screen', () => {
		settings.show('behaviour');
		settings.showFromHash('#nowhere');

		expect(settings.section).toBe('behaviour');
	});

	it('takes a fragment with no hash character', () => {
		settings.showFromHash('appearance');

		expect(settings.section).toBe('appearance');
	});

	/**
	 * The section was called `advanced` until it was renamed to `license`, which
	 * is what it had always actually held. A link written before the rename has
	 * to keep working — a fragment that silently selects nothing is worse than
	 * one that is simply wrong.
	 */
	it('still accepts the old #advanced fragment', () => {
		settings.showFromHash('#advanced');
		expect(settings.section).toBe('license');

		settings.show('you');
		settings.showFromHash('advanced');
		expect(settings.section).toBe('license');
	});
});

/**
 * FEAT-054. The check is a network request, which makes the preference that
 * governs it the interesting part rather than the result.
 */
describe('checking for a newer Spagitty', () => {
	const RELEASED = {
		channel: 'released' as const,
		current: 'v0.1.0-preview.1',
		latest: 'v0.1.0-preview.4',
		newer: true,
		url: 'https://github.com/Spa-git-ty/spagitty/releases/tag/v0.1.0-preview.4'
	};

	it('reports what the project answered', async () => {
		checkUpdate.mockResolvedValueOnce(RELEASED);

		await settings.checkForUpdate();

		expect(settings.update).toEqual(RELEASED);
		expect(settings.updateError).toBeNull();
	});

	it('asks when the button is pressed, whatever the startup preference says', async () => {
		// The preference governs the check at startup. A button that silently
		// did nothing because of a setting on the same screen would be worse
		// than not having the button.
		settingsCall.mockResolvedValue({
			checkForUpdates: false,
			confirmHistoryRewrite: true,
			showGitCommands: false,
			pruneOnFetch: false,
			fetchAvatars: false,
			personality: 'balanced',
			sound: 'off'
		});
		await settings.load();
		checkUpdate.mockResolvedValueOnce(RELEASED);

		await settings.checkForUpdate();

		expect(checkUpdate).toHaveBeenCalled();
		expect(settings.update).toEqual(RELEASED);
	});

	it('keeps a failed check out of the way of the identity fields', async () => {
		// `busy` gates the writes. A check that could not reach the network
		// must not leave Save disabled.
		checkUpdate.mockRejectedValueOnce('could not reach api.github.com');

		await settings.checkForUpdate();

		expect(settings.updateError).toContain('could not reach');
		expect(settings.busy).toBe(false);
		expect(settings.update).toBeNull();
	});

	it('refuses to ask twice at once', async () => {
		let release: (value: Update) => void = () => {};
		checkUpdate.mockReturnValueOnce(new Promise<Update>((resolve) => (release = resolve)));

		const first = settings.checkForUpdate();
		const second = settings.checkForUpdate();

		expect(checkUpdate).toHaveBeenCalledTimes(1);

		release(RELEASED);
		await first;
		await second;
		expect(settings.checking).toBe(false);
	});

	it('does nothing outside the application', async () => {
		inTauri.mockReturnValueOnce(false);

		await settings.checkForUpdate();

		expect(checkUpdate).not.toHaveBeenCalled();
	});

	it('forgets what it found when the screen is cleared', async () => {
		checkUpdate.mockResolvedValueOnce(RELEASED);
		await settings.checkForUpdate();

		settings.clearState();

		expect(settings.update).toBeNull();
		expect(settings.updateError).toBeNull();
	});
});
