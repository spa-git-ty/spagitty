// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The cURL window (FEAT-166): a copy without secrets unless asked, quoted
 * for a shell, and a paste that says what it read and where it goes.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render, type Mounted } from '../../testing/mount';
import type { Call, Collection, Stored } from './types';

const { notice } = vi.hoisted(() => ({ notice: { ok: vi.fn(), failed: vi.fn() } }));

vi.mock('$lib/ui/notice.svelte', () => ({ notice, describe: (error: unknown) => String(error) }));
vi.mock('./api', () => ({
	curl: vi.fn(async (ask: { quoting: string; secrets: boolean }) =>
		ask.secrets ? `curl -u 'mira:hunter2' (${ask.quoting})` : `curl -u 'mira:{{password}}' (${ask.quoting})`
	),
	curlParse: vi.fn(async (text: string) => {
		if (!text.startsWith('curl')) throw 'this is not a cURL command: it should start with curl';
		return {
			call: { id: 'c_new', name: 'Create orders', kind: 'rest', method: 'POST', url: 'https://h/orders' },
			read: ['-X', '-H'],
			ignored: ['-s']
		};
	})
}));

import * as api from './api';
import CurlWindow from './CurlWindow.svelte';

const CALL: Call = { id: 'c_1', name: 'Log in', kind: 'rest', method: 'POST', url: '{{baseUrl}}/login' };
const collections: Stored<Collection>[] = [
	{ stem: 'orders-api', file: 'x', error: null, value: { v: 1, name: 'orders-api', items: [CALL] } },
	{ stem: 'billing', file: 'y', error: null, value: { v: 1, name: 'billing', items: [] } }
];

let view: Mounted | null = null;
const onreplace = vi.fn();
const oncreate = vi.fn();
const onclose = vi.fn();

async function settle(ms = 0): Promise<void> {
	for (let i = 0; i < 4; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, ms));
	}
	flushSync();
}

async function show() {
	view = render(CurlWindow, {
		opened: { stem: 'orders-api', id: 'c_1' },
		call: CALL,
		environment: 'staging',
		collections,
		onreplace,
		oncreate,
		onclose
	});
	await settle();
}

const button = (text: string) =>
	[...document.querySelectorAll('button')].find((b) => b.textContent?.replace(/\s+/g, ' ').trim().startsWith(text)) as HTMLButtonElement;
const command = () => (document.querySelector('textarea[aria-label="The command"]') as HTMLTextAreaElement).value;

beforeEach(() => vi.clearAllMocks());
afterEach(() => {
	view?.destroy();
	view = null;
	document.body.innerHTML = '';
});

describe('the cURL window', () => {
	it('copies without secrets until they are asked for', async () => {
		await show();
		expect(command()).toContain('{{password}}');
		expect(vi.mocked(api.curl).mock.calls[0][0]).toMatchObject({ stem: 'orders-api', environment: 'staging', secrets: false });
		click(document.querySelector('label.check input') as HTMLElement);
		await settle();
		expect(command()).toContain('hunter2');
		click(button('Windows cmd'));
		await settle();
		expect(command()).toContain('(cmd)');
	});

	it('reads a paste, says what it read, and replaces the request', async () => {
		await show();
		click(button('Paste'));
		const area = document.querySelector('textarea[aria-label="A curl command"]') as HTMLTextAreaElement;
		area.value = 'curl -s -X POST https://h/orders -H "A: b"';
		fire(area, 'input');
		await settle(60);
		await new Promise((done) => setTimeout(done, 250));
		await settle();
		expect(document.body.textContent).toContain('Read -X -H.');
		expect(document.body.textContent).toContain('Not read: -s.');
		click(button('Replace the request'));
		expect(onreplace).toHaveBeenCalledWith(expect.objectContaining({ method: 'POST', url: 'https://h/orders' }));
		expect(onclose).toHaveBeenCalled();
	});

	it('adds a pasted call to another collection and folder', async () => {
		await show();
		click(button('Paste'));
		const area = document.querySelector('textarea[aria-label="A curl command"]') as HTMLTextAreaElement;
		area.value = 'curl https://h/orders';
		fire(area, 'input');
		await new Promise((done) => setTimeout(done, 250));
		await settle();
		click(button('A new call'));
		const select = document.querySelector('select[aria-label="Collection"]') as HTMLSelectElement;
		select.value = 'billing';
		fire(select, 'change');
		const folder = document.querySelector('input[aria-label="Folder"]') as HTMLInputElement;
		folder.value = 'Imported';
		fire(folder, 'input');
		click(button('Add the call'));
		expect(oncreate).toHaveBeenCalledWith(expect.objectContaining({ id: 'c_new' }), 'billing', 'Imported');
	});

	it('says why a paste is not a command', async () => {
		await show();
		click(button('Paste'));
		const area = document.querySelector('textarea[aria-label="A curl command"]') as HTMLTextAreaElement;
		area.value = 'wget x';
		fire(area, 'input');
		await new Promise((done) => setTimeout(done, 250));
		await settle();
		expect(document.querySelector('[role="alert"]')?.textContent).toContain('it should start with curl');
		expect(button('Replace the request').disabled).toBe(true);
	});
});
