// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Share (FEAT-164): the three ways, what goes in, the preview, and the
 * button that says what it will do.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render, type Mounted } from '../../testing/mount';
import type { Collection, Environment, Stored } from './types';

const { notice, save } = vi.hoisted(() => ({
	notice: { ok: vi.fn(), failed: vi.fn() },
	save: vi.fn(async () => 'C:/Users/me/orders-api.zip')
}));

vi.mock('$lib/ui/notice.svelte', () => ({ notice, describe: (error: unknown) => String(error) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ save }));
vi.mock('./api', () => ({
	sharePlan: vi.fn(async (ask: { format: string; environments: string[] }) => ({
		files:
			ask.format === 'zip'
				? [
						{ path: 'collection.json', size: 2048 },
						...ask.environments.map((env) => ({ path: `env/${env}.json`, size: 300 })),
						{ path: 'README.md', size: 400 }
					]
				: ask.format === 'commit'
					? [{ path: '.spagitty/calls/orders-api.json', size: 2048 }]
					: [{ path: 'orders-api.postman_collection.json', size: 3000 }],
		total: 1500
	})),
	share: vi.fn(async (ask: { format: string }) => (ask.format === 'commit' ? 'abc1234def' : 'C:/Users/me/orders-api.zip'))
}));

import * as api from './api';
import ShareDialog from './ShareDialog.svelte';

const collections: Stored<Collection>[] = [
	{ stem: 'orders-api', file: '.spagitty/calls/orders-api.json', error: null, value: { v: 1, name: 'orders-api', items: [] } }
];
const env = (name: string): Stored<Environment> => ({
	stem: name,
	file: `.spagitty/calls/env/${name}.json`,
	error: null,
	value: { v: 1, name, color: 'ok', askBeforeSending: false, vars: [] }
});

let view: Mounted | null = null;
const onclose = vi.fn();

async function settle(): Promise<void> {
	for (let i = 0; i < 4; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	flushSync();
}

async function show(shareAs: 'file' | 'zip' | 'commit' = 'zip') {
	view = render(ShareDialog, { collections, environments: [env('local'), env('staging')], stem: 'orders-api', shareAs, onclose });
	await settle();
}

const button = (text: string) =>
	[...document.querySelectorAll('button')].find((b) => b.textContent?.replace(/\s+/g, ' ').trim().startsWith(text)) as HTMLButtonElement;
const files = () => [...document.querySelectorAll('.preview li')].map((li) => li.textContent?.replace(/\s+/g, ' ').trim());

beforeEach(() => vi.clearAllMocks());
afterEach(() => {
	view?.destroy();
	view = null;
	document.body.innerHTML = '';
});

describe('Share', () => {
	it('starts as Settings says, with every environment, and previews the zip', async () => {
		await show();
		expect(button('Zip bundle').getAttribute('aria-checked')).toBe('true');
		expect(files()).toEqual(['collection.json2.0 KB', 'env/local.json300 B', 'env/staging.json300 B', 'README.md400 B']);
		expect(document.body.textContent).toContain('Secrets and kept values go in empty, in every format.');
		expect(button('Save orders-api.zip · 1.5 KB')).toBeTruthy();
	});

	it('leaves an environment out when it is unticked', async () => {
		await show();
		const staging = [...document.querySelectorAll('label.check')].find((l) => l.textContent?.includes('staging'))!;
		click(staging.querySelector('input') as HTMLElement);
		await settle();
		expect(vi.mocked(api.sharePlan).mock.calls.at(-1)?.[0]).toMatchObject({ format: 'zip', environments: ['local'] });
	});

	it('saves one Postman file where the person chooses', async () => {
		await show('file');
		click(button('Postman 2.1'));
		await settle();
		expect(document.body.textContent).toContain('pm.environment.set');
		click(button('Save orders-api as Postman 2.1 JSON'));
		await settle();
		expect(save).toHaveBeenCalledWith({ defaultPath: 'orders-api.postman_collection.json' });
		expect(vi.mocked(api.share).mock.calls[0][0]).toMatchObject({ stem: 'orders-api', format: 'postman', environments: [], path: 'C:/Users/me/orders-api.zip' });
		expect(onclose).toHaveBeenCalled();
	});

	it('commits on this branch without asking where', async () => {
		await show('commit');
		click(button('Commit 1 file to this branch'));
		await settle();
		expect(save).not.toHaveBeenCalled();
		expect(vi.mocked(api.share).mock.calls[0][0]).toMatchObject({ format: 'commit', path: null });
		expect(notice.ok).toHaveBeenCalledWith('Committed abc1234: Calls: update orders-api');
	});
});
