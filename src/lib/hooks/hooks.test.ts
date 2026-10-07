// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import type { Hooks } from '$lib/types';

const emitted: Array<(event: { payload: unknown }) => void> = [];
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (_name: string, handler: (event: { payload: unknown }) => void) => {
		emitted.push(handler);
		return () => {};
	})
}));

vi.mock('$lib/api', () => ({
	inTauri: vi.fn(() => true),
	hooks: vi.fn(),
	setHooksEnabled: vi.fn(() => Promise.resolve()),
	commit: vi.fn(() => Promise.resolve('abc')),
	workingCopy: vi.fn(() =>
		Promise.resolve({ staged: [{ path: 'a.txt', status: 'modified' }], unstaged: [], conflicted: [] })
	),
	signing: vi.fn(() => Promise.resolve(null)),
	workingDiff: vi.fn(() =>
		Promise.resolve({ path: 'a.txt', status: 'modified', binary: false, tooLarge: false, added: 0, removed: 0, hunks: [] })
	)
}));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/delight/watch', () => ({ commitLanded: vi.fn() }));

import * as api from '$lib/api';
import { changes } from '$lib/changes/store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import DialogHost from '$lib/ui/DialogHost.svelte';
import HookRunWindow from './HookRunWindow.svelte';
import HooksView from './HooksView.svelte';
import { hooks, onCommit } from './store.svelte';

const info = (overrides: Partial<Hooks> = {}): Hooks => ({
	dir: '.husky/_',
	manager: 'husky',
	config: null,
	enabled: true,
	hooks: [
		{ name: 'commit-msg', path: '.husky/commit-msg', script: 'npx commitlint --edit "$1"\n', truncated: false, onCommit: true },
		{ name: 'pre-commit', path: '.husky/pre-commit', script: 'npx lint-staged\n', truncated: false, onCommit: true },
		{ name: 'pre-push', path: '.husky/pre-push', script: 'npm test\n', truncated: false, onCommit: false }
	],
	...overrides
});

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

async function ready(subject = 'Fix it') {
	await changes.load();
	await settle();
	changes.setSubject(subject);
}

beforeEach(() => {
	vi.clearAllMocks();
	emitted.length = 0;
	changes.clear();
	hooks.clear();
	vi.mocked(api.hooks).mockResolvedValue(info());
});

describe('which hooks a commit runs (FEAT-107)', () => {
	it('names them in the order git runs them, and none when switched off', () => {
		expect(onCommit(info())).toEqual(['pre-commit', 'commit-msg']);
		expect(onCommit(info({ enabled: false }))).toEqual([]);
		expect(onCommit(null)).toEqual([]);
	});
});

describe('committing with hooks (FEAT-107)', () => {
	it('asks first, then runs them with a log window fed as they print', async () => {
		let land!: (id: string) => void;
		vi.mocked(api.commit).mockImplementationOnce(() => new Promise((resolve) => (land = resolve)));
		await ready();
		const committing = changes.commit();
		await settle();

		expect(dialog.question?.kind).toBe('choice');
		expect(dialog.question?.title).toBe('Run 2 hooks?');
		expect(dialog.question?.body).toContain('pre-commit, commit-msg');
		dialog.accept();
		await settle();

		const token = hooks.run?.token;
		expect(hooks.run?.status).toBe('running');
		emitted.forEach((handler) => handler({ payload: { token, line: 'lint-staged: 3 files' } }));
		emitted.forEach((handler) => handler({ payload: { token: 999, line: 'another run' } }));
		land('abc');

		expect(await committing).toBe(true);
		expect(api.commit).toHaveBeenCalledWith('Fix it', '', false, false, token);
		expect(hooks.run?.lines).toEqual(['lint-staged: 3 files']);
		expect(hooks.run?.status).toBe('passed');
	});

	it('skips them for this commit from the question', async () => {
		await ready();
		const committing = changes.commit();
		await settle();
		dialog.alternative();

		expect(await committing).toBe(true);
		expect(api.commit).toHaveBeenCalledWith('Fix it', '', false, true, null);
		expect(hooks.run).toBeNull();
	});

	it('commits nothing when the question is cancelled', async () => {
		await ready();
		const committing = changes.commit();
		await settle();
		dialog.dismiss();

		expect(await committing).toBe(false);
		expect(api.commit).not.toHaveBeenCalled();
		expect(changes.subject).toBe('Fix it');
	});

	it('does not ask when this commit already skips them, and forgets that after', async () => {
		await ready();
		changes.setSkipHooks(true);

		expect(await changes.commit()).toBe(true);
		expect(dialog.question).toBeNull();
		expect(api.commit).toHaveBeenCalledWith('Fix it', '', false, true, null);
		expect(changes.skipHooks).toBe(false);
	});

	it('does not ask when there are no commit hooks, or they are off', async () => {
		vi.mocked(api.hooks).mockResolvedValue(info({ enabled: false }));
		await ready();

		expect(await changes.commit()).toBe(true);
		expect(dialog.question).toBeNull();
		expect(api.commit).toHaveBeenCalledWith('Fix it', '', false, false, null);
	});

	it('keeps the log open on a failing hook and says what stopped it', async () => {
		vi.mocked(api.commit).mockRejectedValueOnce('pre-commit: 2 lint errors');
		await ready();
		const committing = changes.commit();
		await settle();
		dialog.accept();

		expect(await committing).toBe(false);
		expect(hooks.run?.status).toBe('failed');
		expect(hooks.run?.error).toContain('2 lint errors');
		expect(changes.subject).toBe('Fix it');
	});
});

describe('the three-way question', () => {
	it('draws Cancel, the middle way and the affirmative', async () => {
		const view = render(DialogHost, {});
		const answer = dialog.choose({ title: 'Run 1 hook?', body: 'x', confirmLabel: 'Run hooks', alternativeLabel: 'Skip hooks' });
		flushSync();

		const labels = view.all('.actions button').map((b) => b.textContent?.trim());
		expect(labels).toEqual(['Cancel', 'Skip hooks', 'Run hooks']);
		click(view.all('.actions button')[1]);
		expect(await answer).toBe('alternative');
		view.destroy();
	});
});

describe('HookRunWindow', () => {
	it('shows the hooks, their output and cannot be closed while running', async () => {
		const token = await hooks.begin(['pre-commit']);
		hooks.print('checking 4 files');
		const view = render(HookRunWindow, {});
		flushSync();

		expect(view.text()).toContain('pre-commit');
		expect(view.text()).toContain('checking 4 files');
		expect(view.text()).toContain('Running');
		const button = view.get('.actions button') as HTMLButtonElement;
		expect(button.disabled).toBe(true);

		hooks.finish(null);
		flushSync();
		expect(view.text()).toContain('Passed · committed');
		click(view.get('.actions button'));
		expect(hooks.run).toBeNull();
		expect(token).toBeGreaterThan(0);
		view.destroy();
	});
});

describe('HooksView', () => {
	it('lists commit hooks apart from the rest, opens a script, and switches them off', async () => {
		const view = render(HooksView, {});
		await settle();
		flushSync();

		expect(view.text()).toContain('Husky');
		expect(view.text()).toContain('.husky/pre-commit');
		expect(view.text()).toContain('Other moments');
		expect(view.text()).not.toContain('npx lint-staged');

		const head = view.all('.hook-head').find((b) => b.textContent?.includes('pre-commit'));
		click(head as HTMLElement);
		expect(view.text()).toContain('npx lint-staged');

		click(view.get('.switch'));
		await settle();
		expect(api.setHooksEnabled).toHaveBeenCalledWith(false);
		view.destroy();
	});

	it('brings lefthook\'s configuration, where its steps are', async () => {
		vi.mocked(api.hooks).mockResolvedValue(
			info({ manager: 'lefthook', config: { path: 'lefthook.yml', text: 'pre-commit:\n  commands: {}' } })
		);
		const view = render(HooksView, {});
		await settle();
		flushSync();

		expect(view.text()).toContain('lefthook.yml');
		expect(view.text()).toContain('commands');
		view.destroy();
	});
});
