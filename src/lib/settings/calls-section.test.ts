// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Settings › Calls (FEAT-165): the groups, a change written with the rest
 * of the settings, what the open repository overrides, and Reset.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render, type Mounted } from '../../testing/mount';
import type { CallsSettings } from '$lib/calls/types';
import type { Settings } from '$lib/types';

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true),
	setSettings: vi.fn(() => Promise.resolve())
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(async () => null) }));
vi.mock('$lib/calls/api', () => ({
	defaultSettings: vi.fn(),
	overridden: vi.fn(async () => ['timeoutSeconds'])
}));

import * as api from '$lib/api';
import * as callsApi from '$lib/calls/api';
import CallsSection from './CallsSection.svelte';
import { SECTIONS, settings } from './store.svelte';

const DEFAULTS: CallsSettings = {
	folder: '.spagitty/calls',
	openFound: true,
	lookFor: ['postman', 'insomnia', 'bruno', 'openapi', 'http', 'wsdl'],
	scanOnOpen: 'ask',
	frameworks: ['springMvc', 'springWebflux', 'jaxRs', 'jaxWs', 'springWs'],
	skipPaths: '**/test/**, **/generated/**',
	clickToKeep: true,
	keepEvery: true,
	suggestNames: true,
	timeoutSeconds: 30,
	followRedirects: true,
	checkTls: true,
	proxy: { kind: 'system' },
	clientCerts: [],
	beautify: true,
	everyHeader: null,
	soapVersion: '1.1',
	envelopeFromWsdl: true,
	wsSecurity: 'username',
	faultAsError: true,
	history: 500,
	shareAs: 'zip'
};

let view: Mounted | null = null;

async function settle(): Promise<void> {
	for (let i = 0; i < 4; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	flushSync();
}

async function show(calls: Partial<CallsSettings> = {}): Promise<Mounted> {
	await settings.write({ ...settings.settings, calls: { ...DEFAULTS, ...calls } } as Settings);
	vi.mocked(api.setSettings).mockClear();
	view = render(CallsSection, {});
	await settle();
	return view;
}

const written = () => (vi.mocked(api.setSettings).mock.calls.at(-1)?.[0] as Settings).calls!;
const button = (text: string) =>
	view!.all('button').find((found) => found.textContent?.replace(/\s+/g, ' ').trim() === text) as HTMLButtonElement;

beforeEach(() => {
	view?.destroy();
	view = null;
	vi.clearAllMocks();
});

describe('Settings › Calls', () => {
	it('is a chip after Behaviour', () => {
		const ids = SECTIONS.map((section) => section.id);
		expect(ids[ids.indexOf('behaviour') + 1]).toBe('calls');
	});

	it('shows the six groups, without the frameworks no reader exists for', async () => {
		await show();
		expect(view!.all('h3').map((heading) => heading.textContent)).toEqual([
			'Where calls live',
			'Sending',
			'Scanning code',
			'SOAP',
			'Kept values',
			'Safety and sharing'
		]);
		for (const missing of ['Micronaut', 'Quarkus', 'Express', 'FastAPI']) expect(view!.text()).not.toContain(missing);
		expect(view!.text()).toContain('.spagitty/calls/settings.json');
		// Not a setting: always, and locked.
		expect(view!.text()).toContain('Leave secrets out when sharing Always');
		expect(button('Signed').disabled).toBe(true);
	});

	it('writes a change with the rest of the settings', async () => {
		await show();
		const redirects = view!.get('input[aria-label="Follow redirects"]') as HTMLInputElement;
		redirects.checked = false;
		fire(redirects, 'change');
		await settle();
		expect(written().followRedirects).toBe(false);

		click(button('Custom'));
		await settle();
		expect(written().proxy).toEqual({ kind: 'custom', url: 'http://' });

		click(button('HAR'));
		await settle();
		expect(written().lookFor).toContain('har');

		const timeout = view!.get('input[aria-label="Time out after, in seconds"]') as HTMLInputElement;
		timeout.value = '0';
		fire(timeout, 'change');
		await settle();
		expect(view!.get('[role="alert"]').textContent).toBe('Between 1 and 3600.');
	});

	it('says which settings the open repository overrides', async () => {
		await show();
		expect(callsApi.overridden).toHaveBeenCalled();
		const rows = view!.all('.row').filter((row) => row.textContent?.includes('Overridden in this repository'));
		expect(rows.map((row) => row.querySelector('.label')?.textContent?.replace('Overridden in this repository', '').trim())).toEqual([
			'Time out after'
		]);
	});

	it('resets the section to what a new install has', async () => {
		await show({ beautify: false, history: 20 });
		vi.mocked(callsApi.defaultSettings).mockResolvedValue(DEFAULTS);
		click(button('Reset'));
		await settle();
		expect(written()).toEqual(DEFAULTS);
	});
});
