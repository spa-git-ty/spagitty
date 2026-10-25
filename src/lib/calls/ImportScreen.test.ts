// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Calls › Import (FEAT-162): sources, endpoints with their tags, the
 * filters, what is ticked, and *Bring in*.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import type { Call, Endpoint, Found, Loaded } from './types';

const { notice, handlers } = vi.hoisted(() => ({
	notice: { ok: vi.fn(), failed: vi.fn() },
	handlers: new Map<string, (event: { payload: unknown }) => void>()
}));

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true)
}));
vi.mock('$lib/ui/notice.svelte', () => ({ notice, describe: (error: unknown) => String(error) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(async () => 'C:/Downloads/shop.postman_collection.json') }));
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
		handlers.set(name, handler);
		return () => handlers.delete(name);
	})
}));
vi.mock('./api', () => ({
	CHUNK_EVENT: 'calls-chunk',
	DONE_EVENT: 'calls-done',
	SCAN_EVENT: 'calls-scan',
	SCAN_DONE_EVENT: 'calls-scan-done',
	load: vi.fn(),
	saveState: vi.fn(async () => {}),
	privateOf: vi.fn(async () => ({ kept: [], secrets: [] })),
	foundFiles: vi.fn(),
	found: vi.fn(),
	scan: vi.fn(async () => {}),
	cancelScan: vi.fn(async () => {}),
	importFile: vi.fn(),
	importText: vi.fn(),
	bringIn: vi.fn()
}));

import * as api from './api';
import ImportScreen from './ImportScreen.svelte';
import { importer } from './import.svelte';
import { calls } from './store.svelte';

const call = (name: string, method: string, url: string): Call => ({ id: `c_${name}`, name, kind: 'rest', method, url });

function endpoint(id: string, method: string, path: string, status: Endpoint['status'], extra: Partial<Endpoint> = {}): Endpoint {
	return {
		id,
		key: `${method} ${path}`,
		source: 'code:src/OrderController.java',
		group: 'OrderController',
		file: 'src/OrderController.java',
		line: 3,
		method,
		path,
		hint: '',
		call: call(id, method, `{{baseUrl}}${path}`),
		status,
		...extra
	};
}

const FOUND: Found = {
	sources: [
		{ id: 'code:src/OrderController.java', kind: 'spring', label: 'OrderController.java', file: 'src/OrderController.java', count: 3, error: null, inRepository: true },
		{ id: 'file:api/orders.http', kind: 'http', label: 'orders.http', file: 'api/orders.http', count: 1, error: null, inRepository: true }
	],
	endpoints: [
		endpoint('e1', 'GET', '/orders/{id}', { kind: 'new' }, { hint: 'id' }),
		endpoint('e2', 'POST', '/orders', {
			kind: 'changed',
			collection: 'orders-api',
			callId: 'c_x',
			what: 'CreateOrderRequest gained couponCode'
		}),
		endpoint('e3', 'DELETE', '/orders/{id}', { kind: 'in', collection: 'orders-api', callId: 'c_y' }),
		endpoint('e4', 'GET', '/health', { kind: 'new' }, { source: 'file:api/orders.http', group: 'orders.http', file: 'api/orders.http' })
	],
	environments: [],
	scripted: 2,
	baseUrl: 'http://localhost:8085',
	filesRead: 40,
	capped: false
};

function loaded(): Loaded {
	return {
		folder: '.spagitty/calls',
		collections: [{ stem: 'orders-api', file: '.spagitty/calls/orders-api.json', error: null, value: { v: 1, name: 'orders-api', items: [] } }],
		environments: [],
		shared: { v: 1, vars: [] },
		sharedError: null,
		state: { active: null, open: [], current: null, collapsed: [], filter: '' },
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

async function show(found: Found | null = FOUND): Promise<Mounted> {
	vi.mocked(api.load).mockResolvedValue(loaded());
	vi.mocked(api.found).mockResolvedValue(found);
	view = render(ImportScreen, {});
	await settle();
	return view;
}

const rows = () =>
	view!.all('.endpoint').map((row) => ({
		text: row.querySelector('.path')?.textContent,
		tag: row.querySelector('.tag')?.textContent,
		ticked: (row.querySelector('input') as HTMLInputElement).checked
	}));
const button = (text: string) =>
	view!.all('button').find((found) => found.textContent?.replace(/\s+/g, ' ').trim().startsWith(text)) as HTMLButtonElement;

beforeEach(() => {
	vi.clearAllMocks();
	handlers.clear();
	calls.reset();
	importer.reset();
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

describe('Import', () => {
	it('groups sources and endpoints, ticking New and Changed', async () => {
		await show();
		expect(view!.all('.sources .source:not(.all) .label').map((s) => s.textContent)).toEqual(['OrderController.java', 'orders.http']);
		expect(rows()).toEqual([
			{ text: '/orders/{id}', tag: 'New', ticked: true },
			{ text: '/orders', tag: 'Changed', ticked: true },
			{ text: '/orders/{id}', tag: 'In orders-api', ticked: false },
			{ text: '/health', tag: 'New', ticked: true }
		]);
		// A changed one says what changed.
		expect(view!.all('.endpoint')[1].textContent).toContain('CreateOrderRequest gained couponCode');
		expect(view!.text()).toContain("2 calls had Postman script lines that are not run; they are in each call's notes.");
		expect(button('Add 3 to orders-api')).toBeTruthy();
	});

	it('filters by status and by source', async () => {
		await show();
		click(button('Changed'));
		expect(rows().map((row) => row.text)).toEqual(['/orders']);
		click(button('All'));
		click(view!.all('.sources .source').find((s) => s.textContent?.includes('orders.http'))!);
		expect(rows().map((row) => row.text)).toEqual(['/health']);
	});

	it('brings the ticked ones in with the choices, into a new collection', async () => {
		await show();
		vi.mocked(api.bringIn).mockResolvedValue({ stem: 'shop', brought: { added: 2, changed: 1, unchanged: 0 }, environments: 1 });
		click(view!.get('input[aria-label="GET /health"]'));
		click(button('New collection'));
		const name = view!.get('input[aria-label="The new collection\'s name"]') as HTMLInputElement;
		name.value = 'shop';
		fire(name, 'input');
		click(button('By path'));
		click(button('Create collection shop with 2'));
		await settle();
		expect(api.bringIn).toHaveBeenCalledWith({
			ids: ['e1', 'e2'],
			into: null,
			name: 'shop',
			choices: { folders: 'byPath', bodies: true, pathVars: true },
			serverAddress: true
		});
		expect(notice.ok).toHaveBeenCalledWith('Brought into shop: 2 added, 1 updated, 1 environment made');
	});

	it('scans, says how far it has got, and takes what it found', async () => {
		await show(null);
		expect(view!.text()).toContain('Scan the code, or choose a file');
		click(button('Scan the code'));
		await settle();
		const id = vi.mocked(api.scan).mock.calls[0][0];
		handlers.get('calls-scan')!({ payload: { scanId: id, files: 1200 } });
		flushSync();
		expect(view!.get('[role="status"]').textContent).toBe('Reading files… 1,200');
		handlers.get('calls-scan-done')!({ payload: { scanId: id, found: FOUND, error: null, cancelled: false } });
		await settle();
		expect(rows()).toHaveLength(4);
	});

	it('adds a file chosen from outside and a pasted cURL', async () => {
		await show();
		const outside: Found = {
			...FOUND,
			sources: [...FOUND.sources, { id: 'file:C:/Downloads/shop.postman_collection.json', kind: 'postman', label: 'shop.postman_collection.json', file: 'C:/Downloads/shop.postman_collection.json', count: 1, error: null, inRepository: false }],
			endpoints: [
				...FOUND.endpoints,
				endpoint('e5', 'GET', '/ping', { kind: 'new' }, { source: 'file:C:/Downloads/shop.postman_collection.json', group: 'Requests' })
			]
		};
		vi.mocked(api.importFile).mockResolvedValue(outside);
		click(button('Choose a file…'));
		await settle();
		expect(api.importFile).toHaveBeenCalledWith('C:/Downloads/shop.postman_collection.json');
		// Shown on its own, ticked.
		expect(rows()).toEqual([{ text: '/ping', tag: 'New', ticked: true }]);

		vi.mocked(api.importText).mockRejectedValue('this is not a cURL command');
		click(button('Paste cURL'));
		const area = view!.get('textarea[aria-label="A cURL command"]') as HTMLTextAreaElement;
		area.value = 'wget x';
		fire(area, 'input');
		click(button('Read it'));
		await settle();
		expect(notice.failed).toHaveBeenCalledWith('Could not read that cURL command', 'this is not a cURL command');
	});
});
