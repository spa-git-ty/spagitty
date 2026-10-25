// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Calls, the screen (FEAT-160): open calls as chips, the request's tabs,
 * closing calls, and a send from *Send* to the response.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import type { Done, Loaded } from './types';

const { handlers, notice } = vi.hoisted(() => ({
	handlers: new Map<string, (event: { payload: unknown }) => void>(),
	notice: { ok: vi.fn(), failed: vi.fn() }
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true)
}));
vi.mock('$lib/ui/notice.svelte', () => ({ notice, describe: (error: unknown) => String(error) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn(async () => null) }));
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
		handlers.set(name, handler);
		return () => handlers.delete(name);
	})
}));
vi.mock('./api', () => ({
	CHUNK_EVENT: 'calls-chunk',
	DONE_EVENT: 'calls-done',
	load: vi.fn(),
	saveCollection: vi.fn(async (_collection: unknown, stem: string | null) => stem ?? 'made'),
	deleteCollection: vi.fn(),
	saveState: vi.fn(async () => {}),
	send: vi.fn(async () => {}),
	cancel: vi.fn(async () => {}),
	saveBody: vi.fn(),
	history: vi.fn(async () => []),
	historyBody: vi.fn(),
	clearHistory: vi.fn()
}));

import * as api from './api';
import CallsScreen from './CallsScreen.svelte';
import { calls, SAVE_DELAY } from './store.svelte';

function loaded(ask = false): Loaded {
	return {
		folder: '.spagitty/calls',
		collections: [
			{
				stem: 'orders-api',
				file: '.spagitty/calls/orders-api.json',
				error: null,
				value: {
					v: 1,
					name: 'orders-api',
					items: [
						{
							folder: 'Auth',
							items: [
								{
									id: 'c_login',
									name: 'Log in',
									kind: 'rest',
									method: 'POST',
									url: '{{baseUrl}}/auth/login',
									headers: [
										['Accept', 'application/json'],
										['X-Trace', '1']
									],
									body: { type: 'json', text: '{"email":"{{email}}"}' },
									keep: [{ var: 'token', from: '$.data.accessToken', every: true }]
								}
							]
						},
						{ id: 'c_user', name: 'Get user', kind: 'rest', method: 'GET', url: '{{baseUrl}}/users/{{userId}}' }
					]
				}
			},
			{ stem: 'broken', file: '.spagitty/calls/broken.json', error: 'expected value at line 1', value: null }
		],
		environments: [
			{
				stem: 'staging',
				file: '.spagitty/calls/env/staging.json',
				error: null,
				value: {
					v: 1,
					name: 'staging',
					color: ask ? 'danger' : 'warn',
					askBeforeSending: ask,
					vars: [
						{ name: 'baseUrl', value: 'https://staging.orders.local/api' },
						{ name: 'email', value: 'mira@example.com' },
						{ name: 'password', secret: true }
					]
				}
			}
		],
		shared: { v: 1, vars: [] },
		sharedError: null,
		state: {
			active: 'staging',
			open: [{ stem: 'orders-api', id: 'c_login' }],
			current: { stem: 'orders-api', id: 'c_login' },
			collapsed: [],
			filter: ''
		},
		settings: null as unknown as Loaded['settings'],
		overridden: [],
		today: 3
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

async function show(ask = false): Promise<Mounted> {
	vi.mocked(api.load).mockResolvedValue(loaded(ask));
	view = render(CallsScreen, {});
	await settle();
	return view;
}

const chips = () => view!.all('.chip .open').map((chip) => chip.textContent?.replace(/\s+/g, ' ').trim());
const button = (text: string) =>
	view!.all('button').find((found) => found.textContent?.replace(/\s+/g, ' ').trim().startsWith(text)) as HTMLButtonElement;

beforeEach(() => {
	vi.clearAllMocks();
	handlers.clear();
	calls.reset();
	control.reset();
	control.setInfo({
		path: '/repos/orders-service',
		name: 'orders-service',
		bare: false,
		lastFetched: null,
		head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
	});
});

afterEach(() => {
	view?.destroy();
	view = null;
});

describe('Calls', () => {
	it('shows collections, the open call, and the environment’s variables', async () => {
		await show();
		expect(chips()).toEqual(['POST Log in']);
		expect(view!.text()).toContain('orders-api');
		expect(view!.text()).toContain('.spagitty/calls/orders-api.json');
		// A file that could not be read is listed with why.
		expect(view!.text()).toContain('Not read: expected value at line 1');
		expect((view!.get('input[aria-label="URL"]') as HTMLInputElement).value).toBe('{{baseUrl}}/auth/login');
		// The secret is dots, never its value.
		expect(view!.get('.vars').textContent).toContain('••••••••••');
		expect(view!.text()).toContain('3 today');
		// The URL's variable is a pill.
		expect(view!.all('.pill').map((pill) => pill.textContent)).toContain('{{baseUrl}}');
	});

	it('opens a call as a chip, and closes it with ✕, Ctrl+W and a middle-click', async () => {
		await show();
		click(view!.all('.call').find((call) => call.textContent?.includes('Get user'))!);
		expect(chips()).toEqual(['POST Log in', 'GET Get user']);
		expect((view!.get('input[aria-label="URL"]') as HTMLInputElement).value).toBe('{{baseUrl}}/users/{{userId}}');

		press(window, 'w', { ctrlKey: true });
		expect(chips()).toEqual(['POST Log in']);

		click(view!.all('.call').find((call) => call.textContent?.includes('Get user'))!);
		const chip = view!.all('.chip .open').find((found) => found.textContent?.includes('Get user'))!;
		chip.dispatchEvent(new MouseEvent('auxclick', { bubbles: true, button: 1 }));
		flushSync();
		expect(chips()).toEqual(['POST Log in']);

		click(view!.get('button[aria-label="Close Log in"]'));
		expect(chips()).toEqual([]);
		expect(view!.text()).toContain('Open a call on the left');
		await new Promise((done) => setTimeout(done, SAVE_DELAY + 50));
		expect(vi.mocked(api.saveState).mock.calls.at(-1)?.[0]).toMatchObject({ open: [], current: null });
	});

	it('counts what each tab holds and switches between them', async () => {
		await show();
		const tabs = view!.all('.request [role="tab"]').map((tab) => tab.textContent?.replace(/\s+/g, ' ').trim());
		expect(tabs).toEqual(['Params', 'Headers2', 'Body', 'Auth', 'Keep1']);
		// The body is shown first, as the call has one.
		expect(view!.find('textarea[aria-label="Body"]')).not.toBeNull();
		click(view!.all('.request [role="tab"]')[1]);
		expect(view!.all('input[aria-label="header name"]').map((input) => (input as HTMLInputElement).value)).toEqual([
			'Accept',
			'X-Trace',
			''
		]);
	});

	it('writes an edit to the collection a moment later', async () => {
		await show();
		const name = view!.get('input[aria-label="The call\'s name"]') as HTMLInputElement;
		name.value = 'Sign in';
		fire(name, 'input');
		expect(chips()).toEqual(['POST Sign in']);
		expect(api.saveCollection).not.toHaveBeenCalled();
		await new Promise((done) => setTimeout(done, SAVE_DELAY + 50));
		expect(api.saveCollection).toHaveBeenCalledTimes(1);
		const [written, stem] = vi.mocked(api.saveCollection).mock.calls[0];
		expect(stem).toBe('orders-api');
		expect(JSON.stringify(written)).toContain('"name":"Sign in"');
	});

	it('refuses to send a call whose variable nothing defines, and names it', async () => {
		await show();
		click(view!.all('.call').find((call) => call.textContent?.includes('Get user'))!);
		click(button('Send'));
		await settle();
		expect(api.send).not.toHaveBeenCalled();
		expect(view!.get('.refusal').textContent).toBe('Not sent: {{userId}} has no value in staging.');
		expect(view!.all('.pill.unknown').map((pill) => pill.textContent)).toEqual(['{{userId}}']);
	});

	it('sends, can cancel, and shows the response formatted', async () => {
		await show();
		press(window, 'Enter', { ctrlKey: true });
		await settle();
		expect(api.send).toHaveBeenCalledTimes(1);
		const ask = vi.mocked(api.send).mock.calls[0][0];
		expect(ask).toMatchObject({ stem: 'orders-api', environment: 'staging', call: { id: 'c_login' } });
		expect(view!.text()).toContain('Waiting for the answer');

		click(button('Cancel'));
		await settle();
		expect(api.cancel).toHaveBeenCalledWith(ask.sendId);

		const done: Done = {
			sendId: ask.sendId,
			callId: 'c_login',
			error: null,
			cancelled: false,
			sent: { method: 'POST', url: 'https://staging.orders.local/api/auth/login', headers: [], body: null },
			historyId: 'h_1',
			response: {
				status: 200,
				reason: 'OK',
				headers: [
					['Content-Type', 'application/json'],
					['Set-Cookie', 'sid=1; HttpOnly']
				],
				millis: 142,
				size: 1434,
				truncated: false,
				shape: 'json',
				mediaType: 'application/json',
				text: '{"data":{"accessToken":"ey.J"}}',
				plain: false,
				url: 'https://staging.orders.local/api/auth/login'
			}
		};
		handlers.get('calls-done')!({ payload: done });
		await settle();
		expect(view!.get('.status').textContent?.replace(/\s+/g, ' ').trim()).toBe('200 OK');
		expect(view!.text()).toContain('142 ms · 1.4 KB');
		const lines = () => view!.all('.response .line').map((line) => line.textContent);
		expect(lines()).toEqual(['{', '  "data": {', '    "accessToken": "ey.J"', '  }', '}']);
		expect(view!.text()).toContain('4 today');

		// Beautify off shows the body as it arrived.
		click(button('{ } Beautify'));
		expect(lines()).toEqual(['{"data":{"accessToken":"ey.J"}}']);
		expect(view!.text()).toContain('As received');
	});

	it('asks before sending in an environment that says to', async () => {
		await show(true);
		click(button('Send'));
		await settle();
		expect(api.send).not.toHaveBeenCalled();
		const bar = view!.get('[role="alertdialog"]');
		expect(bar.textContent?.replace(/\s+/g, ' ').trim()).toContain('Send Log in to staging?');
		click([...bar.querySelectorAll('button')].find((found) => found.textContent === 'Send') as HTMLElement);
		await settle();
		expect(api.send).toHaveBeenCalledTimes(1);
		expect(view!.find('[role="alertdialog"]')).toBeNull();
	});
});
