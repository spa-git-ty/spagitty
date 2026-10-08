// SPDX-License-Identifier: GPL-3.0-or-later

import { readFileSync } from 'node:fs';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import type { Identity, IdentityValue, Licenses } from '$lib/types';

/**
 * Everything a reader can get to: the visible text, plus every `title`.
 *
 * TASK-044 moved the long-form claims onto hovers. The contract was never
 * "these words are on the screen" — it was "a reader who asks can find this
 * out", and a claim only in a comment would fail that while a claim in a title
 * passes it. So the assertions read both, and what changed is the *volume* on
 * the screen rather than what the application is willing to tell you.
 */
function readable(view: { text(): string; all(selector: string): HTMLElement[] }): string {
	const titles = view.all('[title]').map((element) => element.getAttribute('title') ?? '');
	return [view.text(), ...titles].join(' ');
}

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
import { theme } from '$lib/theme.svelte';
import { FAMILIES, paletteOf } from '$lib/themes';
import AccountsSection from './AccountsSection.svelte';
import LicenseSection from './LicenseSection.svelte';
import AppearanceSection from './AppearanceSection.svelte';
import BehaviourSection from './BehaviourSection.svelte';
import UpdateSection from './UpdateSection.svelte';
import { SECTIONS } from './store.svelte';
import IdentitySection from './IdentitySection.svelte';
import { settings } from './store.svelte';

const identity = vi.mocked(api.identity);
const forgeAccounts = vi.mocked(api.forgeAccounts);
const setIdentity = vi.mocked(api.setIdentity);
const settingsCall = vi.mocked(api.settings);
const setSettings = vi.mocked(api.setSettings);
const licenses = vi.mocked(api.licenses);
const about = vi.mocked(api.about);

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

const LIST: Licenses = {
	generated: true,
	notes: [],
	rust: [
		{ name: 'gix', version: '0.86.0', license: 'MIT OR Apache-2.0' },
		{ name: 'mystery', version: '1.0.0', license: null }
	],
	npm: [{ name: '@tauri-apps/api', version: '2.0.0', license: 'Apache-2.0 OR MIT' }]
};

/** Type into a field the way the component listens for it. */
function type(input: HTMLElement, text: string): void {
	(input as HTMLInputElement).value = text;
	input.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
}

beforeEach(async () => {
	vi.clearAllMocks();
	settings.clearState();
	identity.mockResolvedValue(anIdentity());
	settingsCall.mockResolvedValue({
		checkForUpdates: true,
		confirmHistoryRewrite: true,
		showGitCommands: false, pruneOnFetch: false,
fetchAvatars: false,
			personality: 'balanced',
			sound: 'off'
	});
	licenses.mockResolvedValue(LIST);
	about.mockResolvedValue({ version: '0.1.0', commit: 'abc1234', license: 'GPL-3.0-or-later' });
	theme.setFamily('pomodoro');
	theme.setMode('light');
});

/**
 * Every chip has somewhere to go.
 *
 * `accounts` was a chip with no branch behind it: the Settings screen's final
 * `{:else}` renders the License section, so pressing Accounts showed licences
 * while the accounts themselves were drawn under You the whole time. Nothing
 * failed, nothing warned, and the screen had been shipped like that.
 *
 * The assertion reads the route's source because a route is not a component
 * this suite mounts — the same arrangement `graph/store.test.ts` uses for the
 * nav rail's copy.
 */
describe('the section index and the screen agree', () => {
	const screen = readFileSync('src/routes/settings/+page.svelte', 'utf8');

	it('gives every chip a branch that renders it', () => {
		const missing = SECTIONS.filter(
			(section) => !screen.includes(`settings.section === '${section.id}'`)
		).map((section) => section.id);

		expect(
			missing,
			'these chips fall through to the catch-all and draw the wrong section'
		).toEqual([]);
	});

	it('names no section the index does not have', () => {
		const branched = [...screen.matchAll(/settings\.section === '([a-z]+)'/g)].map(
			(match) => match[1]
		);
		const known = new Set<string>(SECTIONS.map((section) => section.id));

		expect(branched.filter((id) => !known.has(id))).toEqual([]);
	});

	it('has no Accounts chip, because connecting a host is part of You', () => {
		expect(SECTIONS.map((section) => section.id)).not.toContain('accounts');
		expect(screen, 'and the section is still drawn').toContain('<AccountsSection />');
	});
});

describe('IdentitySection', () => {
	it('shows each value in its field, with no line repeating where it came from (TASK-050)', async () => {
		await settings.load();
		const mounted = render(IdentitySection, {});

		expect((mounted.get('#identity-name') as HTMLInputElement).value).toBe('Ada Lovelace');
		expect(mounted.text()).not.toContain('From your global configuration.');

		mounted.destroy();
	});

	it('cannot save until the field differs from what is stored', async () => {
		await settings.load();
		const mounted = render(IdentitySection, {});
		const save = mounted.all('button').find((button) => button.textContent?.includes('Save'));

		expect((save as HTMLButtonElement).disabled).toBe(true);
		type(mounted.get('#identity-name'), 'Grace Hopper');
		expect((save as HTMLButtonElement).disabled).toBe(false);

		mounted.destroy();
	});

	it('writes the field to the scope the chips say is being edited', async () => {
		await settings.load();
		setIdentity.mockResolvedValue(anIdentity());
		const mounted = render(IdentitySection, {});

		type(mounted.get('#identity-email'), 'grace@example.com');
		const save = mounted
			.all('button')
			.filter((button) => button.textContent?.includes('Save'))[1];
		click(save);

		expect(setIdentity).toHaveBeenCalledWith('global', 'email', 'grace@example.com');
		mounted.destroy();
	});

	it('turns the repository scope off with no repository open, and says why on it', async () => {
		identity.mockResolvedValue(anIdentity({ repository: false }));
		await settings.load();
		const mounted = render(IdentitySection, {});
		const local = mounted
			.all('button')
			.find((button) => button.textContent?.includes('this repository'));

		expect(local?.hasAttribute('disabled')).toBe(true);
		expect(local?.getAttribute('title')).toBe('No repository is open');

		mounted.destroy();
	});

	it('warns that a repository override is what will actually be committed with', async () => {
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
		const mounted = render(IdentitySection, {});

		expect(mounted.text()).toContain('This repository sets its own');

		mounted.destroy();
	});
});

describe('BehaviourSection', () => {
	it('says a toggle is not honoured yet, without naming a work item', async () => {
		// A switch that silently does nothing is worse than one that says it is
		// waiting on something.
		await settings.load();
		const mounted = render(BehaviourSection, {});

		expect(mounted.text()).toContain('Not honoured yet');
		expect(mounted.text()).toContain('rewrites history');
		expect(mounted.text()).not.toMatch(/FEAT-\d/);

		mounted.destroy();
	});

	it('stores the whole settings object when a toggle is flipped', async () => {
		await settings.load();
		const mounted = render(BehaviourSection, {});

		click(mounted.all('button.chip')[0]);

		expect(setSettings).toHaveBeenCalledWith({
			checkForUpdates: true,
			confirmHistoryRewrite: false,
			showGitCommands: false,
			pruneOnFetch: false,
			fetchAvatars: false,
			personality: 'balanced',
			sound: 'off'
		});
		mounted.destroy();
	});

	it('shows the stored state of every toggle', async () => {
		settingsCall.mockResolvedValue({
			checkForUpdates: true,
			confirmHistoryRewrite: false,
			showGitCommands: true,
			pruneOnFetch: false,
			fetchAvatars: false,
			personality: 'balanced',
			sound: 'off'
		});
		await settings.load();
		const mounted = render(BehaviourSection, {});

		// Four toggles, in the order they are declared: FEAT-018 added pruning,
		// FEAT-019 took signing away — it is now `commit.gpgsign` under You
		// rather than a preference here — and FEAT-079 added the author
		// pictures.
		expect(mounted.all('button.chip').map((chip) => chip.textContent?.trim())).toEqual([
			'off',
			'on',
			'off',
			'off'
		]);
		mounted.destroy();
	});
});

describe('AppearanceSection', () => {
	it('offers no family to choose, only light, dark or the system (TASK-051)', () => {
		const mounted = render(AppearanceSection, {});

		expect(mounted.all('.family')).toHaveLength(0);
		expect(mounted.text()).toContain('Light');
		expect(mounted.text()).toContain('Dark');

		mounted.destroy();
	});

	it('marks the mode in use and switches to the other one', () => {
		const mounted = render(AppearanceSection, {});

		const [light, dark] = mounted.all('button.chip');
		expect(light.classList.contains('active')).toBe(true);

		click(dark);
		expect(theme.mode).toBe('dark');

		mounted.destroy();
	});

});

describe('AccountsSection', () => {
	it('lists nothing with no account connected, and offers the two fields that connect one', () => {
		const mounted = render(AccountsSection, {});

		expect(
			mounted.all('button').some((button) => button.textContent?.includes('Disconnect'))
		).toBe(false);
		expect(mounted.get('#account-host')).toBeTruthy();
		expect(mounted.get('#account-token')).toBeTruthy();
		expect(mounted.text()).not.toMatch(/FEAT-\d/);

		mounted.destroy();
	});

	it('never puts the token in a field anyone can read', () => {
		// Shoulder-reading, and a screenshot in a bug report. It is also never
		// read back out of the keychain into this screen at all.
		const mounted = render(AccountsSection, {});

		expect(mounted.get('#account-token').getAttribute('type')).toBe('password');
		expect(mounted.get('#account-token').getAttribute('autocomplete')).toBe('off');

		mounted.destroy();
	});

	it('will not connect with an empty token or an empty host', () => {
		const mounted = render(AccountsSection, {});
		const connect = mounted
			.all('button')
			.find((button) => button.textContent?.includes('Connect'));

		// The host is prefilled and the token is not, so the button starts dead
		// rather than sending an empty secret to a host.
		expect((connect as HTMLButtonElement).disabled).toBe(true);

		mounted.destroy();
	});

	it('lists a connected account by host and login, and offers to disconnect it', async () => {
		forgeAccounts.mockResolvedValueOnce([
			{ kind: 'gitHub' as const, host: 'github.com', user: 'ada' }
		]);
		await settings.load();
		const mounted = render(AccountsSection, {});

		expect(mounted.text()).toContain('ada');
		expect(mounted.text()).toContain('github.com');
		expect(
			mounted.all('button').some((button) => button.textContent?.includes('Disconnect'))
		).toBe(true);

		mounted.destroy();
	});

	it('offers God mode only to the author's GitHub account', async () => {
		forgeAccounts.mockResolvedValueOnce([
			{ kind: 'gitHub' as const, host: 'github.com', user: 'ada' }
		]);
		await settings.load();
		expect(settings.godMode).toBe(false);

		forgeAccounts.mockResolvedValueOnce([
			{ kind: 'gitHub' as const, host: 'github.example.com', user: 'maxmya' }
		]);
		await settings.load();
		expect(settings.godMode, 'the login must be on github.com itself').toBe(false);

		forgeAccounts.mockResolvedValueOnce([
			{ kind: 'gitHub' as const, host: 'github.com', user: 'MaxMya' }
		]);
		await settings.load();
		expect(settings.godMode).toBe(true);
	});

	it('says which token scopes it needs, and nothing more (TASK-050)', () => {
		const mounted = render(AccountsSection, {});
		const text = readable(mounted);

		expect(text).toContain('Pull requests: read');
		expect(text).not.toContain('uploads no repository');

		mounted.destroy();
	});
});

describe('LicenseSection', () => {
	it('keeps everything the About footer carried', async () => {
		// The GPL-3 obligations predate this screen and must not regress while
		// it is rebuilt.
		await settings.load();
		const mounted = render(LicenseSection, {});
		const text = mounted.text();

		expect(text).toContain('Spagitty v0.1.0');
		expect(text).toContain('abc1234');
		expect(text).toContain('GPL-3.0-or-later');
		expect(text).toContain('Software Freedom Conservancy');

		mounted.destroy();
	});

	it('lists both trees and names a package that declares nothing', async () => {
		await settings.load();
		const mounted = render(LicenseSection, {});
		const text = mounted.text();

		expect(text).toContain('gix');
		expect(text).toContain('@tauri-apps/api');
		expect(text).toContain('not declared');

		mounted.destroy();
	});

	it('filters both lists by package or license', async () => {
		await settings.load();
		const mounted = render(LicenseSection, {});

		type(mounted.get('input'), 'gix');

		expect(mounted.all('.entry').length).toBe(1);
		expect(mounted.text()).toContain('gix');

		mounted.destroy();
	});

	it('says the list was not generated instead of showing an empty one', async () => {
		// An empty list reads as "no dependencies", which would be a false claim
		// about what the binary is made of.
		licenses.mockResolvedValue({
			generated: false,
			notes: ['The Rust dependency list was not generated: cargo metadata failed.'],
			rust: [],
			npm: []
		});
		await settings.load();
		const mounted = render(LicenseSection, {});

		expect(mounted.text()).toContain('did not generate a dependency license list');
		expect(mounted.text()).toContain('cargo metadata failed');
		expect(mounted.all('.entry').length).toBe(0);

		mounted.destroy();
	});

	it('still shows the version and the commit when the list is missing', async () => {
		licenses.mockResolvedValue({ generated: false, notes: [], rust: [], npm: [] });
		await settings.load();
		const mounted = render(LicenseSection, {});

		expect(mounted.text()).toContain('abc1234');
		expect(mounted.text()).toContain('GPL-3.0-or-later');

		mounted.destroy();
	});
});

describe('UpdateSection', () => {
	const checkUpdate = vi.mocked(api.checkUpdate);

	// The store is module state and outlives a test, so what the last check
	// found would otherwise still be on screen for the next one.
	beforeEach(() => settings.clearState());

	it('shows no result before anything has been checked', () => {
		const mounted = render(UpdateSection, {});

		expect(mounted.text()).not.toContain('Up to date');
		expect(mounted.text()).not.toContain('has been released');
		mounted.destroy();
	});

	it('says what leaves the machine, beside the switch that stops it', () => {
		// The one preference in the application that causes a network request,
		// so the sentence explaining it belongs where the decision is made.
		const mounted = render(UpdateSection, {});
		const text = readable(mounted);

		expect(text).toContain('Turning it off stops every request');

		mounted.destroy();
	});

	it('reports a newer release with a link that can be copied', async () => {
		checkUpdate.mockResolvedValueOnce({
			channel: 'released',
			current: 'v0.1.0-preview.1',
			latest: 'v0.1.0-preview.4',
			newer: true,
			url: 'https://github.com/Spa-git-ty/spagitty/releases/tag/v0.1.0-preview.4'
		});
		await settings.checkForUpdate();
		const mounted = render(UpdateSection, {});

		expect(mounted.text()).toContain('v0.1.0-preview.4');
		expect(mounted.text()).toContain('has been released');
		expect(mounted.text()).toContain('https://github.com/Spa-git-ty/spagitty/releases/tag/');

		mounted.destroy();
	});

	it('says a build compiled here is not out of date', async () => {
		// It has no tag to be behind, and it is usually ahead of every release.
		checkUpdate.mockResolvedValueOnce({
			channel: 'development',
			current: null,
			latest: 'v0.1.0-preview.9',
			newer: false,
			url: 'https://github.com/Spa-git-ty/spagitty/releases'
		});
		await settings.checkForUpdate();
		const mounted = render(UpdateSection, {});

		expect(readable(mounted)).toContain('Development build');
		expect(mounted.text()).toContain('v0.1.0-preview.9');
		// No download offered: the only thing that appears for a newer release
		// is the link and its copy button, and this is not one.
		expect(
			mounted.all('button').some((button) => button.textContent?.includes('Copy link'))
		).toBe(false);

		mounted.destroy();
	});

	it('says so when there is nothing newer', async () => {
		checkUpdate.mockResolvedValueOnce({
			channel: 'released',
			current: 'v0.1.0-preview.4',
			latest: 'v0.1.0-preview.4',
			newer: false,
			url: 'https://github.com/Spa-git-ty/spagitty/releases/tag/v0.1.0-preview.4'
		});
		await settings.checkForUpdate();
		const mounted = render(UpdateSection, {});

		expect(mounted.text()).toContain('Up to date');

		mounted.destroy();
	});

	it('shows the failure rather than a stale answer', async () => {
		checkUpdate.mockRejectedValueOnce('could not reach api.github.com');
		await settings.checkForUpdate();
		const mounted = render(UpdateSection, {});

		expect(mounted.text()).toContain('could not reach');

		mounted.destroy();
	});
});
