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
	confirm: vi.fn(() => Promise.resolve()),
	sendFindings: vi.fn(() => Promise.resolve({ task: 'TASK-0004', message: 'TASK-0004 is in the farm as a draft.' }))
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

describe('sending findings to an agent', () => {
	it('reports the draft task it made, and the refusal when it made none', async () => {
		list.mockResolvedValueOnce(listing());
		await extensions.setRepository('/repo');
		const record = { result: { reviewId: 'rv-1' } } as never;
		await extensions.send('com.example.hello', record, ['f-1']);
		expect(api.sendFindings).toHaveBeenCalledWith('com.example.hello', '/repo', 'rv-1', ['f-1']);
		expect(notice.current?.detail).toContain('TASK-0004');
		vi.mocked(api.sendFindings).mockRejectedValueOnce({ kind: 'refused', message: 'These findings are about uncommitted changes.' });
		await extensions.send('com.example.hello', record, ['f-1']);
		expect(notice.current?.tone).toBe('error');
		expect(notice.current?.detail).toContain('uncommitted');
	});
});

describe('confirmations from the backend', () => {
	it('removes an expired or cancelled confirmation without posting', async () => {
		await extensions.start();
		handlers['extension-confirm']({ payload: { id: 'ended', extension: 'x', kind: 'pullRequestComment', title: 'Post?', target: 'o/r#1', body: 'hi' } });
		handlers['extension-confirm-ended']({ payload: 'ended' });
		expect(extensions.confirmations).toEqual([]);
		expect(api.confirm).not.toHaveBeenCalled();
	});
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

it('an older scope preview cannot overwrite the newer scope or reopen a closed dialog', async () => {
 list.mockResolvedValue(listing());await extensions.setRepository('/repo');
 let resolveOld!: (p: never)=>void;
 vi.mocked(api.previewReview).mockImplementationOnce(()=>new Promise(resolve=>{resolveOld=resolve as never;}));
 const old=extensions.beginReview('com.example.hello','review','workingCopy');
 await vi.waitFor(()=>expect(api.previewReview).toHaveBeenCalled());
 vi.mocked(api.previewReview).mockResolvedValueOnce({scope:'committed',files:[{path:'new',status:'modified',origin:'committed'}]} as never);
 await extensions.updateDraft({scope:'committed'});
 resolveOld({scope:'uncommitted',files:[{path:'old'}]} as never);await old;
 expect(extensions.draft?.preview?.files[0].path).toBe('new');
 extensions.closeDraft();expect(extensions.draft).toBeNull();
});
it('preview, start, cancellation and history failures stay visible and never imply success', async () => {
 list.mockResolvedValue(listing());await extensions.setRepository('/repo');
 vi.mocked(api.previewReview).mockRejectedValueOnce(new Error('preview changed'));
 await extensions.beginReview('com.example.hello','review','workingCopy');
 expect(extensions.draft?.error).toContain('preview changed');
 await extensions.confirmDraft();expect(api.startReview).not.toHaveBeenCalled();
 vi.mocked(api.previewReview).mockResolvedValueOnce({scope:'committed',files:[]} as never);
 await extensions.updateDraft({scope:'committed'});
 vi.mocked(api.startReview).mockRejectedValueOnce(new Error('consent revoked'));
 await extensions.confirmDraft();expect(extensions.draft?.error).toContain('consent revoked');expect(extensions.draft?.busy).toBe(false);
 vi.mocked(api.cancel).mockRejectedValueOnce(new Error('already stopped'));
 await extensions.cancel('gone');expect(notice.current?.detail).toContain('already stopped');
 vi.mocked(api.reviews).mockRejectedValueOnce(new Error('history unavailable'));
 await extensions.loadReviews('com.example.hello');expect(extensions.error).toContain('history unavailable');
 await extensions.setRepository('/other');expect(extensions.draft).toBeNull();
});

it.each(['success', 'failure'])('a %s from an old repository cannot replace review history or errors', async (outcome) => {
	list.mockResolvedValue(listing());
	await extensions.setRepository('/repo');
	let complete!: (value: never) => void;
	let fail!: (error: Error) => void;
	vi.mocked(api.reviews).mockImplementationOnce(() => new Promise((resolve, reject) => {
		complete = resolve as never;
		fail = reject;
	}));
	const pending = extensions.loadReviews('com.example.hello');
	await extensions.setRepository('/other');
	if (outcome === 'success') complete([{ result: { reviewId: 'old' } }] as never);
	else fail(new Error('old repository unavailable'));
	await pending;
	expect(extensions.reviewsOf('com.example.hello')).toEqual([]);
	expect(extensions.error).toBeNull();
});
it.each(['success', 'failure'])('an old review start %s cannot close or reopen another repository dialog', async (outcome) => {
	list.mockResolvedValue(listing());
	vi.mocked(api.previewReview).mockResolvedValue({ scope: 'uncommitted', files: [] } as never);
	await extensions.setRepository('/repo');
	await extensions.beginReview('com.example.hello', 'review', 'workingCopy');
	let complete!: (value: never) => void;
	let fail!: (error: Error) => void;
	vi.mocked(api.startReview).mockImplementationOnce(() => new Promise((resolve, reject) => {
		complete = resolve as never;
		fail = reject;
	}));
	const pending = extensions.confirmDraft();
	await extensions.setRepository('/other');
	await extensions.beginReview('com.example.hello', 'review', 'workingCopy');
	if (outcome === 'success') complete({ operation: 'old', reviewId: 'old' } as never);
	else fail(new Error('old start failed'));
	await pending;
	expect(extensions.draft?.workdir).toBe('/other');
	expect(extensions.draft?.error).toBeNull();
	expect(extensions.draft?.busy).toBe(false);
});
it('a delayed base lookup cannot reopen a closed review dialog', async () => {
	list.mockResolvedValue(listing());
	await extensions.setRepository('/repo');
	let complete!: (bases: string[]) => void;
	vi.mocked(api.suggestedBases).mockImplementationOnce(() => new Promise(resolve => { complete = resolve; }));
	const pending = extensions.beginReview('com.example.hello', 'review', 'workingCopy');
	extensions.closeDraft();
	complete(['main']);
	await pending;
	expect(extensions.draft).toBeNull();
	expect(api.previewReview).not.toHaveBeenCalled();
});
