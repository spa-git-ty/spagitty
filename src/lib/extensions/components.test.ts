// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { click, render } from '../../testing/mount';
import { extension, listing, reviewRecord } from '../../testing/extension-fixtures';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(() => Promise.resolve(null)) }));
vi.mock('./api', () => ({
	list: vi.fn(),
	enable: vi.fn(() => Promise.resolve()),
	disable: vi.fn(() => Promise.resolve()),
	setGrant: vi.fn(() => Promise.resolve()),
	setSetting: vi.fn(() => Promise.resolve()),
	reviews: vi.fn(() => Promise.resolve([])),
	setDisposition: vi.fn(() => Promise.resolve()),
	deleteReviews: vi.fn(() => Promise.resolve()),
	panel: vi.fn(() => Promise.resolve({ title: 'About', rows: [{ label: 'Branch', value: 'main' }] })),
	suggestedBases: vi.fn(() => Promise.resolve([])),
	previewReview: vi.fn(),
	runCommand: vi.fn(),
	cancel: vi.fn(() => Promise.resolve()),
	uninstall: vi.fn(() => Promise.resolve()),
	restart: vi.fn(() => Promise.resolve()),
	install: vi.fn(),
	inspect: vi.fn()
}));

import * as api from './api';
import ContributedActions from './ContributedActions.svelte';
import ExtensionCard from './ExtensionCard.svelte';
import ExtensionPanels from './ExtensionPanels.svelte';
import FindingsPanel from './FindingsPanel.svelte';
import InstallDialog from './InstallDialog.svelte';
import ReviewStatusPanel from './ReviewStatusPanel.svelte';
import { extensions } from './store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import type { InstallPreview } from './types';

async function settle() {
	for (let i = 0; i < 5; i++) await Promise.resolve();
	flushSync();
}

beforeEach(async () => {
	vi.clearAllMocks();
	extensions.reset();
	vi.mocked(api.list).mockResolvedValue(listing());
	await extensions.setRepository('/repo');
});

describe('contributed actions', () => {
	it('draws one button per action, greyed with its reason', () => {
		const view = render(ContributedActions, { context: 'workingCopy' as const });
		const buttons = view.all('button');
		expect(buttons.map((b) => b.textContent?.trim())).toEqual(['Menu only', 'Review changes', 'Say hello']);
		const quiet = buttons[0];
		expect(quiet.hasAttribute('disabled')).toBe(true);
		expect(quiet.getAttribute('title')).toBe('No changes');
		view.destroy();
	});

	it('draws nothing for an extension that failed', async () => {
		vi.mocked(api.list).mockResolvedValue(listing([extension({ state: 'failed' })]));
		await extensions.refresh();
		const view = render(ContributedActions, { context: 'workingCopy' as const });
		expect(view.all('button')).toEqual([]);
		view.destroy();
	});
});

describe('the findings panel', () => {
	it('tells a result from approval and invents no location', async () => {
		vi.mocked(api.reviews).mockResolvedValue([reviewRecord()]);
		const view = render(FindingsPanel, { extension: extension(), provider: 'review', title: 'Findings' });
		await settle();
		expect(view.text()).toContain('Reviewed — 2 findings');
		expect(view.text()).toContain('Blocked');
		expect(view.text()).toContain('Whole review');
		expect(view.text()).not.toMatch(/approved/i);
		const severities = view.all('.finding .sev').map((e) => e.textContent?.trim());
		expect(severities).toEqual(['High', 'Unknown']);
		view.destroy();
	});

	it('records a dismissal and offers to send only what is selected', async () => {
		vi.mocked(api.reviews).mockResolvedValue([reviewRecord()]);
		const onsend = vi.fn();
		const view = render(FindingsPanel, { extension: extension(), provider: 'review', title: 'Findings', onsend });
		await settle();
		const dismiss = view.all('button').find((b) => b.textContent?.trim() === 'Dismiss')!;
		click(dismiss);
		expect(api.setDisposition).toHaveBeenCalledWith('com.example.hello', '/repo', 'rv-1791000000000-1', 'f-1', 'dismissed');

		const send = view.all('button').find((b) => b.textContent?.includes('to an agent'))!;
		expect(send.hasAttribute('disabled')).toBe(true);
		const box = view.get('input[type="checkbox"]') as HTMLInputElement;
		box.checked = true;
		box.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();
		click(send);
		await settle();
		expect(onsend).toHaveBeenCalledWith(expect.objectContaining({ provider: 'review' }), ['f-1']);
		view.destroy();
	});

	it('shows a running review with its elapsed time and a way to cancel', async () => {
		extensions.receive({ kind: 'operationStarted', operation: 'op-3', extension: 'com.example.hello', reviewId: 'rv-3', title: 'Review' });
		const view = render(FindingsPanel, { extension: extension(), provider: 'review', title: 'Findings' });
		await settle();
		expect(view.find('[role="status"]')?.textContent).toContain('Reviewing');
		click(view.all('button').find((b) => b.textContent?.trim() === 'Cancel')!);
		expect(api.cancel).toHaveBeenCalledWith('op-3');
		view.destroy();
	});

	it('keeps very long text inside the card', async () => {
		const record = reviewRecord();
		record.result.findings[0].title = 'x'.repeat(5000);
		vi.mocked(api.reviews).mockResolvedValue([record]);
		const view = render(FindingsPanel, { extension: extension(), provider: 'review', title: 'Findings' });
		await settle();
		expect(view.text()).toContain('x'.repeat(100));
		view.destroy();
	});
});

describe('the extension card', () => {
	it('asks for consent before turning on an extension that sends code', async () => {
		const view = render(ExtensionCard, {
			extension: extension({ enabled: false, consented: false, state: 'disabled' }),
			workdir: '/repo',
			onupdate: vi.fn()
		});
		const toggle = view.get('input[type="checkbox"]');
		click(toggle);
		await settle();
		expect(dialog.question?.body).toContain('Example Cloud');
		dialog.accept();
		await settle();
		expect(api.enable).toHaveBeenCalledWith('com.example.hello', '/repo', [], expect.stringContaining('Example Cloud'));
		view.destroy();
	});

	it('marks only bundled extensions official, whatever the publisher says', () => {
		const local = render(ExtensionCard, {
			extension: extension({ publisher: 'Spagitty' }),
			workdir: '/repo',
			onupdate: vi.fn()
		});
		expect(local.get('.badge').textContent?.trim()).toBe('Installed from a file');
		local.destroy();
		const bundled = render(ExtensionCard, {
			extension: extension({ provenance: 'bundled', official: true }),
			workdir: '/repo',
			onupdate: vi.fn()
		});
		expect(bundled.get('.badge').textContent?.trim()).toBe('Official');
		expect(bundled.text()).not.toContain('Remove');
		bundled.destroy();
	});

	it('shows its own setup panel only when asked, so opening Settings starts nothing', async () => {
		const ext = extension({
			manifest: {
				...extension().manifest,
				contributes: { panels: [{ id: 'connection', title: 'Setup', renderer: 'summary', location: 'global' }] }
			}
		});
		vi.mocked(api.list).mockResolvedValue(listing([ext]));
		await extensions.refresh();
		const view = render(ExtensionCard, { extension: ext, workdir: '/repo', onupdate: vi.fn() });
		await settle();
		expect(api.panel).not.toHaveBeenCalled();
		click(view.all('button').find((b) => b.textContent?.trim() === 'Show')!);
		await settle();
		expect(api.panel).toHaveBeenCalledWith('com.example.hello', 'connection', expect.anything());
		view.destroy();
	});

	it('keeps diagnostics behind an explicit action', () => {
		const view = render(ExtensionCard, {
			extension: extension({ diagnostics: { stderr: 'boom', logs: [], toolRuns: [], error: null, program: null } }),
			workdir: '/repo',
			onupdate: vi.fn()
		});
		expect((view.get('details.diagnostics') as HTMLDetailsElement).open).toBe(false);
		view.destroy();
	});
});

describe('the install dialog', () => {
	it('states the trust model and what is new before anything is installed', () => {
		const oninstall = vi.fn();
		const view = render(InstallDialog, {
			preview: {
				token: 't',
				id: 'com.example.hello',
				name: 'Hello',
				version: '2.0.0',
				publisher: 'Example',
				description: null,
				license: 'MIT',
				targets: ['x86_64-pc-windows-msvc'],
				capabilities: [{ capability: 'tools.execute', required: true, granted: false, description: 'Run tools' }],
				files: [{ path: 'bin/hello.exe', size: 2048, sha256: 'ab', executable: true }],
				digest: 'ab'.repeat(32),
				warnings: [],
				compatibility: { compatible: true, reasons: [] },
				replaces: '1.0.0',
				addedCapabilities: ['tools.execute'],
				trust: 'Extensions are programs that run on this computer with your permissions.'
			} as InstallPreview,
			oninstall,
			oncancel: vi.fn()
		});
		expect(view.text()).toContain('with your permissions');
		expect(view.text()).toContain('new in this version');
		expect(view.text()).toContain('not verified');
		click(view.all('button').find((b) => b.textContent?.trim() === 'Update to 2.0.0')!);
		expect(oninstall).toHaveBeenCalled();
		view.destroy();
	});
});

describe('panels drawn from data', () => {
	it('draws a summary the extension supplied', async () => {
		const view = render(ExtensionPanels, { location: 'workingCopy' as const });
		await settle();
		expect(api.panel).toHaveBeenCalledWith('com.example.hello', 'about', expect.objectContaining({ kind: 'workingCopy' }));
		expect(view.text()).toContain('Branch');
		expect(view.text()).toContain('main');
		view.destroy();
	});

	it('never draws "not observed" as a pass', () => {
		const view = render(ReviewStatusPanel, { data: { state: 'notObserved' as const } });
		expect(view.text()).toContain('No review seen');
		expect(view.text()).not.toMatch(/passed|approved/i);
		view.destroy();
		const unknown = render(ReviewStatusPanel, { data: { state: 'bogus' as never } });
		expect(unknown.text()).toContain('Unavailable');
		unknown.destroy();
	});
});
