// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The agents store (2.0): reading, listening, and the notifications it
 * raises while the window is not in front.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { aReview, aSnapshot } from '../../testing/agent-fixtures';

const handlers = new Map<string, (event: { payload: unknown }) => void>();
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
		handlers.set(name, handler);
		return () => handlers.delete(name);
	})
}));
vi.mock('$lib/api', () => ({
	inTauri: vi.fn(() => true),
	notifyDesktop: vi.fn(() => Promise.resolve())
}));
vi.mock('./api', () => ({
	snapshot: vi.fn(),
	list: vi.fn(),
	control: vi.fn(),
	forget: vi.fn(),
	start: vi.fn(),
	consent: vi.fn()
}));

import * as appApi from '$lib/api';
import * as api from './api';
import { dialog } from '$lib/ui/dialog.svelte';
import { notice } from '$lib/ui/notice.svelte';
import { agents } from './store.svelte';

beforeEach(() => {
	vi.clearAllMocks();
	agents.reset(null);
});

describe('reading', () => {
	it('reads the machine list and the repository’s assignments, and listens', async () => {
		vi.mocked(api.snapshot).mockResolvedValue(aSnapshot());
		vi.mocked(api.list).mockResolvedValue([aReview()]);
		await agents.load('/work/spagitty');
		expect(agents.snapshot?.local[0].name).toBe('Claude Code');
		expect(agents.assignments).toHaveLength(1);
		expect(agents.repo).toBe('/work/spagitty');
		expect(handlers.has('assignment-event')).toBe(true);

		handlers.get('assignment-line')!({ payload: { id: 'review-1', line: '· read a.rs' } });
		expect(agents.lines('review-1')).toEqual(['· read a.rs']);
		handlers.get('assignment-event')!({ payload: aReview({ id: 'review-2' }) });
		expect(agents.assignments.map((a) => a.id)).toEqual(['review-2', 'review-1']);
		// Another repository's assignment is not this one's.
		handlers.get('assignment-event')!({ payload: aReview({ id: 'other', repo: '/elsewhere' }) });
		expect(agents.assignments).toHaveLength(2);
	});

	it('says why it could not read', async () => {
		vi.mocked(api.snapshot).mockRejectedValue({ kind: 'io', message: 'no configuration directory' });
		vi.mocked(api.list).mockResolvedValue([]);
		await agents.load(null);
		expect(agents.error).toBe('no configuration directory');
	});
	it('keeps the newest repository when an older read answers last', async () => {
		let answerA!: (value: unknown) => void;
		vi.mocked(api.snapshot)
			.mockImplementationOnce(() => new Promise((resolve) => (answerA = resolve)) as never)
			.mockResolvedValueOnce(aSnapshot());
		vi.mocked(api.list).mockResolvedValue([]);
		const first = agents.load('/a');
		await agents.load('/b');
		answerA(aSnapshot());
		await first;
		expect(agents.repo).toBe('/b');
		expect(agents.loading).toBe(false);
	});
});

describe('notifications', () => {
	it('say when an agent waits, finishes or stops, only while the window is not in front', () => {
		agents.reset(aSnapshot(), [aReview({ state: 'working' })]);
		const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(false);
		agents.absorb(aReview({ state: 'waiting' }));
		agents.absorb(aReview({ state: 'done', sentence: 'Done · 2 findings to decide' }));
		agents.absorb(aReview({ state: 'failed', sentence: 'Codex exited with 1.' }));
		expect(vi.mocked(appApi.notifyDesktop).mock.calls.map((c) => c[0])).toEqual([
			'Claude Code is waiting for you',
			'Claude Code finished',
			'Claude Code stopped'
		]);
		expect(appApi.notifyDesktop).toHaveBeenCalledWith(
			'Claude Code finished',
			'#214 Cache avatars on disk instead of in memory · Done · 2 findings to decide'
		);
		focus.mockReturnValue(true);
		agents.absorb(aReview({ state: 'waiting' }));
		expect(appApi.notifyDesktop).toHaveBeenCalledTimes(3);
		focus.mockRestore();
	});

	it('are each switchable', () => {
		agents.reset(aSnapshot({ notify: { waiting: false, finished: true, stopped: true } }), [aReview({ state: 'working' })]);
		const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(false);
		agents.absorb(aReview({ state: 'waiting' }));
		expect(appApi.notifyDesktop).not.toHaveBeenCalled();
		focus.mockRestore();
	});
});

describe('failures', () => {
	it('a control the engine did not take, a refused start and a forget are said', async () => {
		const failed = vi.spyOn(notice, 'failed');
		vi.mocked(api.control).mockRejectedValue({ kind: 'notFound', message: 'That assignment is not running.' });
		await agents.control('x', { kind: 'stop' });
		vi.mocked(api.start).mockRejectedValue({ kind: 'refused', message: 'Claude Code wrote commits in this pull request, so it cannot review it.' });
		const started = await agents.start(
			{ repo: '/w', agent: 'claude', level: 'stepByStep', note: '', target: aReview().target, work: { job: 'review', description: '', threads: [], checks: '', conflictFixes: [] }, lands: false },
			null,
			null
		);
		expect(started).toBeNull();
		agents.reset(aSnapshot(), [aReview({ state: 'done' })]);
		vi.mocked(api.forget).mockRejectedValue({ kind: 'refused', message: 'Stop the agent first.' });
		await agents.forget('review-1');
		expect(failed.mock.calls.map((c) => c[1])).toEqual([
			'That assignment is not running.',
			'Claude Code wrote commits in this pull request, so it cannot review it.',
			'Stop the agent first.'
		]);
	});

	it('a consent the backend did not keep is said, and the assign is let go', async () => {
		const failed = vi.spyOn(notice, 'failed');
		vi.spyOn(dialog, 'confirm').mockResolvedValue(true);
		vi.mocked(api.start).mockRejectedValue({ kind: 'consent', message: 'This repository has not agreed.' });
		vi.mocked(api.consent).mockRejectedValue({ kind: 'io', message: 'agents.json could not be written.' });
		const started = await agents.start(
			{ repo: '/w', agent: 'api-1', level: 'stepByStep', note: '', target: aReview().target, work: { job: 'review', description: '', threads: [], checks: '', conflictFixes: [] }, lands: false },
			'Anthropic',
			'anthropic'
		);
		expect(started).toBeNull();
		expect(failed).toHaveBeenCalledWith('The agent was not assigned', 'agents.json could not be written.');
	});

	it('forgetting a finished one takes it off the list', async () => {
		agents.reset(aSnapshot(), [aReview({ state: 'done' })]);
		vi.mocked(api.forget).mockResolvedValue(undefined);
		await agents.forget('review-1');
		expect(agents.assignments).toEqual([]);
	});
});
