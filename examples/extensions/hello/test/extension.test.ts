// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'bun:test';
import { FakeHost } from '@spagitty/extension-sdk/testing';
import { extension } from '../src/main';

const repository = {
	'repository.describe': () => ({ name: 'demo', branch: 'main', head: null, detached: false, remotes: [], forge: null })
};

describe('Hello', () => {
	it('says hello with the configured greeting', async () => {
		const host = FakeHost.inMemory(extension, { settings: { greeting: 'Hi' }, services: repository });
		await host.start();
		const outcome = await host.command('hello', { kind: 'workingCopy', repository: 'repo:1' });
		expect(outcome.status).toBe('completed');
		expect(outcome.message).toBe('Hi, demo');
		expect(host.notices[0].message).toBe('Hi from main');
		expect(host.violations).toEqual([]);
	});

	it('draws its panel from data', async () => {
		const host = FakeHost.inMemory(extension, { services: repository });
		await host.start();
		const panel = (await host.panel('about', { kind: 'workingCopy', repository: 'repo:1' })) as { rows: { label: string }[] };
		expect(panel.rows.map((r) => r.label)).toEqual(['Greeting', 'Branch']);
	});
});
