// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { extension, listing } from '../../testing/extension-fixtures';

let handlers: Record<string, (event: { payload: unknown }) => void> = {};
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((name: string, handler: (event: { payload: unknown }) => void) => {
		handlers[name] = handler;
		return Promise.resolve(() => delete handlers[name]);
	})
}));

vi.mock('./api', () => ({
	list: vi.fn(),
	runCommand: vi.fn(() => Promise.resolve({ operation: 'op-1', reviewId: null })),
	suggestedBases: vi.fn(() => Promise.resolve(['main', 'develop'])),
	previewReview: vi.fn(),
	startReview: vi.fn(() => Promise.resolve({ operation: 'op-2', reviewId: 'rv-2' })),
	cancel: vi.fn(() => Promise.resolve()),
	reviews: vi.fn(() => Promise.resolve([])),
	confirm: vi.fn(() => Promise.resolve())
}));

import * as api from './api';
import { extensions } from './store.svelte';
import { palette } from '$lib/palette/store.svelte';
import { notice } from '$lib/ui/notice.svelte';

const list = vi.mocked(api.list);

beforeEach(() => {
	vi.clearAllMocks();
	extensions.reset();
	palette.clear();
	notice.dismiss();
	handlers = {};
});

function registeredIds(): string[] {
	palette.setQuery('');
	return palette.matches.map((m) => m.command.id).filter((id) => id.startsWith('extension:'));
}

describe('the palette', () => {
	it('registers commands as a group and removes them as one when the extension stops contributing', async () => {
		list.mockResolvedValueOnce(listing());
		await extensions.setRepository('/repo');
		expect(registeredIds()).toContain('extension:com.example.hello/hello');
		expect(registeredIds()).not.toContain('extension:com.example.hello/quiet');

		list.mockResolvedValueOnce(listing([extension({ state: 'failed', stateReason: 'It crashed.' })]));
		await extensions.refresh();
		expect(registeredIds()).toEqual([]);
	});

	it('greys a command with its reason instead of hiding it', async () => {
		list.mockResolvedValueOnce(listing());
		await extensions.setRepository('/repo');
		palette.setQuery('Review task');
		const task = palette.matches.find((m) => m.command.id === 'extension:com.example.hello/task');
		expect(task?.command.enabled?.()).toBe(false);
		expect(task?.command.unavailable?.()).toBe('Select a task');
	});
});

describe('operations', () => {
	it('follows an operation from start to finish and reports a failure', async () => {
		list.mockResolvedValue(listing());
		await extensions.start();
		await extensions.setRepository('/repo');
		extensions.receive({ kind: 'operationStarted', operation: 'op-1', extension: 'com.example.hello', reviewId: 'rv-1', title: 'Review' });
		extensions.receive({
			kind: 'operationProgress',
			operation: 'op-1',
			extension: 'com.example.hello',
			message: 'Reviewing 3 files',
			elapsedMs: 4000,
			findings: 2
		});
		expect(extensions.running[0]).toMatchObject({ message: 'Reviewing 3 files', findings: 2, elapsedMs: 4000 });

		extensions.receive({
			kind: 'operationFinished',
			operation: 'op-1',
			extension: 'com.example.hello',
			reviewId: null,
			status: 'failed',
			message: 'It stopped unexpectedly.'
		});
		expect(extensions.running).toEqual([]);
		expect(extensions.finishedOf('op-1')?.status).toBe('failed');
		expect(notice.current?.tone).toBe('error');
	});

	it('marks a cancellation at once and asks the host', async () => {
		extensions.receive({ kind: 'operationStarted', operation: 'op-9', extension: 'x', reviewId: null, title: 'T' });
		await extensions.cancel('op-9');
		expect(extensions.running[0].cancelling).toBe(true);
		expect(api.cancel).toHaveBeenCalledWith('op-9');
	});

	it('shows a notice the extension asked for', () => {
		extensions.receive({ kind: 'notice', extension: 'x', level: 'info', message: 'Hi' });
		expect(notice.current?.title).toBe('Extension');
		expect(notice.current?.detail).toBe('Hi');
	});
});

describe('running a command', () => {
	it('runs an ordinary command against the open repository', async () => {
		list.mockResolvedValueOnce(listing());
		await extensions.setRepository('/repo');
		const hello = extension().manifest.contributes!.commands![0];
		await extensions.run('com.example.hello', hello);
		expect(api.runCommand).toHaveBeenCalledWith('com.example.hello', 'hello', {
			kind: 'workingCopy',
			workdir: '/repo',
			taskId: null,
			pullRequest: null
		});
	});

	it('opens the scope of a review instead of sending anything', async () => {
		list.mockResolvedValueOnce(listing());
		vi.mocked(api.previewReview).mockResolvedValueOnce({
			baseRef: 'HEAD',
			baseCommit: 'a'.repeat(40),
			headCommit: 'a'.repeat(40),
			scope: 'uncommitted',
			files: [{ path: 'x.rs', status: 'modified', origin: 'unstaged' }],
			excluded: [],
			truncated: false
		});
		await extensions.setRepository('/repo');
		const review = extension().manifest.contributes!.commands![1];
		await extensions.run('com.example.hello', review);
		expect(api.startReview).not.toHaveBeenCalled();
		expect(extensions.draft?.preview?.files).toHaveLength(1);
		expect(extensions.draft?.bases).toEqual(['main', 'develop']);

		await extensions.confirmDraft();
		expect(api.startReview).toHaveBeenCalledWith(
			'com.example.hello',
			'review',
			expect.objectContaining({ target: 'workingCopy', scope: 'uncommitted' }),
			'/repo'
		);
		expect(extensions.draft).toBeNull();
	});

	it('refuses with the reason when it cannot run', async () => {
		list.mockResolvedValueOnce(listing());
		await extensions.setRepository('/repo');
		const task = extension().manifest.contributes!.commands![2];
		await extensions.run('com.example.hello', task);
		expect(notice.current?.detail).toBe('Select a task');
		expect(api.runCommand).not.toHaveBeenCalled();
	});

	it('uses a farm task\'s worktree and base when one is selected', async () => {
		list.mockResolvedValueOnce(listing());
		vi.mocked(api.previewReview).mockResolvedValueOnce({
			baseRef: 'main',
			baseCommit: 'a'.repeat(40),
			headCommit: 'b'.repeat(40),
			scope: 'committed',
			files: [],
			excluded: [],
			truncated: false
		});
		await extensions.setRepository('/repo');
		extensions.setContext('farmTask', { taskId: 'TASK-0001', taskHasCommit: true, workdir: '/repo/.spagitty/wt', base: 'main' });
		flushSync();
		const task = extension().manifest.contributes!.commands![2];
		await extensions.run('com.example.hello', task);
		expect(api.previewReview).toHaveBeenCalledWith(
			'com.example.hello',
			'review',
			expect.objectContaining({ target: 'farmTask', scope: 'committed', base: 'main', taskId: 'TASK-0001' }),
			'/repo/.spagitty/wt'
		);
	});
});

describe('confirmations from the backend', () => {
	it('queues them and answers each once', async () => {
		await extensions.start();
		handlers['extension-confirm']({
			payload: { id: 'c1', extension: 'x', kind: 'pullRequestComment', title: 'Post?', target: 'o/r#1', body: 'hi' }
		});
		expect(extensions.confirmations).toHaveLength(1);
		await extensions.answer('c1', true);
		expect(api.confirm).toHaveBeenCalledWith('c1', true);
		expect(extensions.confirmations).toEqual([]);
	});
});
