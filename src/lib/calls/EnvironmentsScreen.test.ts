// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Calls › Environments (FEAT-161): the list, one environment's variables by
 * kind, the values kept from responses, and making one.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import type { Environment, Environments, Loaded } from './types';

const { notice, goto } = vi.hoisted(() => ({
	notice: { ok: vi.fn(), failed: vi.fn() },
	goto: vi.fn()
}));

vi.mock('$app/navigation', () => ({ goto }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true)
}));
vi.mock('$lib/ui/notice.svelte', () => ({ notice, describe: (error: unknown) => String(error) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock('./api', () => ({
	CHUNK_EVENT: 'calls-chunk',
	DONE_EVENT: 'calls-done',
	load: vi.fn(),
	saveCollection: vi.fn(),
	saveState: vi.fn(async () => {}),
	environments: vi.fn(),
	saveEnvironment: vi.fn(),
	deleteEnvironment: vi.fn(),
	saveShared: vi.fn(),
	privateOf: vi.fn(),
	reveal: vi.fn(async () => 'hunter2'),
	setPrivate: vi.fn(async () => {}),
	keep: vi.fn(),
	forgetKept: vi.fn()
}));

import * as api from './api';
import EnvironmentsScreen from './EnvironmentsScreen.svelte';
import { calls, SAVE_DELAY } from './store.svelte';

const env = (name: string, extra: Partial<Environment> = {}): Environment => ({
	v: 1,
	name,
	color: 'ok',
	askBeforeSending: false,
	vars: [],
	...extra
});

const STAGING = env('staging', {
	color: 'warn',
	vars: [
		{ name: 'baseUrl', value: 'https://staging.orders.local/api' },
		{ name: 'password', secret: true },
		{ name: 'token', kept: true }
	]
});

function listed(environments: Environment[]): Environments {
	return {
		environments: environments.map((value) => ({
			stem: value.name,
			file: `.spagitty/calls/env/${value.name}.json`,
			value,
			error: null
		})),
		shared: { v: 1, vars: [{ name: 'tenant', value: 'acme' }] },
		sharedError: null
	};
}

function loaded(): Loaded {
	const all = listed([env('local', { vars: [{ name: 'baseUrl', value: 'http://localhost:8080' }] }), STAGING]);
	return {
		folder: '.spagitty/calls',
		collections: [],
		environments: all.environments,
		shared: all.shared,
		sharedError: null,
		state: { active: 'staging', open: [], current: null, collapsed: [], filter: '' },
		settings: null as unknown as Loaded['settings'],
		overridden: [],
		today: 0
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
	vi.mocked(api.load).mockResolvedValue(loaded());
	view = render(EnvironmentsScreen, {});
	await settle();
	return view;
}

const button = (text: string) =>
	view!.all('button').find((found) => found.textContent?.replace(/\s+/g, ' ').trim() === text) as HTMLButtonElement;

beforeEach(() => {
	vi.clearAllMocks();
	calls.reset();
	control.reset();
	control.setInfo({
		path: '/repos/orders-service',
		name: 'orders-service',
		bare: false,
		lastFetched: null,
		head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
	});
	vi.mocked(api.privateOf).mockImplementation(async (environment) =>
		environment === 'staging'
			? {
					secrets: ['password'],
					kept: [
						{
							name: 'token',
							value: 'ey.J',
							at: Date.now() - 120_000,
							callId: 'c_login',
							call: 'Log in',
							method: 'POST',
							from: '$.data.accessToken',
							stale: false
						}
					]
				}
			: { kept: [], secrets: [] }
	);
});

afterEach(() => {
	view?.destroy();
	view = null;
});

describe('Environments', () => {
	it('lists each environment with its host, and the one in use', async () => {
		await show();
		const rows = view!.all('.env').map((row) => row.textContent?.replace(/\s+/g, ' ').trim());
		expect(rows).toEqual(['local localhost:8080', 'staging staging.orders.local in use']);
		expect(view!.text()).toContain('Repository: .spagitty/calls/env/staging.json');
		expect(view!.text()).toContain('This computer: secrets and kept values, in the system keychain');
	});

	it('shows each variable by kind, never a secret in the clear', async () => {
		await show();
		const kinds = view!.all('.one .kind').map((kind) => kind.textContent?.replace(/\s+/g, ' ').trim());
		expect(kinds).toEqual(['Plain', 'Secret', 'Kept · every call', 'Plain']);
		const secret = view!.get('input[aria-label="The value of password"]') as HTMLInputElement;
		expect(secret.type).toBe('password');
		expect(secret.value).toBe('');
		expect(view!.get('.kept-value').textContent).toBe('ey.J');

		// Held, the eye shows it; let go, it is gone.
		const eye = view!.get('button[aria-label="Hold to show password"]');
		fire(eye, 'pointerdown');
		await settle();
		expect(api.reveal).toHaveBeenCalledWith('staging', 'password');
		expect((view!.get('input[aria-label="The value of password"]') as HTMLInputElement).value).toBe('hunter2');
		fire(eye, 'pointerup');
		expect((view!.get('input[aria-label="The value of password"]') as HTMLInputElement).value).toBe('');
	});

	it('lists what is kept from responses, and freezes one with its switch', async () => {
		await show();
		const card = view!.get('.kept .card');
		expect(card.textContent?.replace(/\s+/g, ' ')).toContain('token');
		expect(card.textContent?.replace(/\s+/g, ' ')).toContain('from POST Log in');
		expect(card.textContent?.replace(/\s+/g, ' ')).toContain('$.data.accessToken');
		expect(card.textContent?.replace(/\s+/g, ' ')).toContain('Updated 2 minutes ago in staging');

		vi.mocked(api.saveEnvironment).mockResolvedValue(listed([STAGING]));
		const toggle = card.querySelector('input[role="switch"]') as HTMLInputElement;
		toggle.checked = false;
		fire(toggle, 'change');
		await new Promise((done) => setTimeout(done, SAVE_DELAY + 50));
		const [written, was] = vi.mocked(api.saveEnvironment).mock.calls[0];
		expect(was).toBe('staging');
		expect(written.vars.find((variable) => variable.name === 'token')).toMatchObject({ kept: true, frozen: true });
	});

	it('makes production red and asking before sending', async () => {
		await show();
		vi.mocked(api.saveEnvironment).mockResolvedValue(
			listed([STAGING, env('production', { color: 'danger', askBeforeSending: true })])
		);
		click(button('+ New environment'));
		const input = view!.get('input[aria-label="The new environment\'s name"]') as HTMLInputElement;
		input.value = 'Production';
		fire(input, 'input');
		press(input, 'Enter');
		await settle();
		expect(api.saveEnvironment).toHaveBeenCalledWith(
			{ v: 1, name: 'production', color: 'danger', askBeforeSending: true, vars: [] },
			null,
			null
		);
		expect(view!.get('.one .title').textContent).toBe('production');
	});

	it('duplicates an environment with its keychain values', async () => {
		await show();
		vi.mocked(api.saveEnvironment).mockResolvedValue(listed([STAGING, { ...STAGING, name: 'staging-copy' }]));
		click(button('Duplicate'));
		await settle();
		const [written, was, copyFrom] = vi.mocked(api.saveEnvironment).mock.calls[0];
		expect(written.name).toBe('staging-copy');
		expect(written.vars).toEqual(STAGING.vars);
		expect(was).toBeNull();
		expect(copyFrom).toBe('staging');
	});

	it('goes back to Calls with Done', async () => {
		await show();
		click(button('Done'));
		expect(goto).toHaveBeenCalledWith('/calls');
	});
});
