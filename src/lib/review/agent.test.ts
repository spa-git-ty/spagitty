// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Review with an agent (2.0): findings as pending comments, the last act at
 * Unattended, a moved head, and assigning.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { aFinding, aReview, aSnapshot } from '../../testing/agent-fixtures';
import { request } from '../../testing/git-fixtures';

vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/agents/api', () => ({
	control: vi.fn(() => Promise.resolve()),
	start: vi.fn((r) => Promise.resolve({ ...r })),
	consent: vi.fn(() => Promise.resolve())
}));

import * as api from '$lib/api';
import * as agentsApi from '$lib/agents/api';
import { agents } from '$lib/agents/store.svelte';
import { control as repoControl } from '../../testing/repo-store.svelte';
import { draftOf, marked, reviewBody, sendable, syncRecord } from './agent-drafts';
import * as work from './agent.svelte';
import { emptyRecord, type PendingComment } from './record';
import { review } from './store.svelte';

const KEY = { host: 'github.com', owner: 'spa-git-ty', name: 'spagitty', number: 214 };

function comment(p = aFinding()) {
	return p as Parameters<typeof draftOf>[1];
}

beforeEach(() => {
	vi.clearAllMocks();
	work.forgetActed();
	review.clear();
	agents.reset(aSnapshot());
	repoControl.reset();
	repoControl.setInfo({ path: '/work/spagitty', name: 'spagitty', bare: false, head: { branch: 'main', detached: false, id: 'a', short: 'a' }, lastFetched: null } as never);
	vi.mocked(api.inTauri).mockReturnValue(false);
});

describe('a finding as a pending comment', () => {
	it('is placed by side and number, against the head it was made on', () => {
		const draft = draftOf(aReview(), comment(aFinding({}, { line: 7 })));
		expect(draft).toMatchObject({ id: 'agent:review-1:p2-0', line: 7, side: 'RIGHT', headSha: 'bbb' });
		expect(draft.agent).toMatchObject({ name: 'Claude Code', severity: 'high', state: 'proposed' });
		const finding = aFinding().body as Extract<ReturnType<typeof aFinding>['body'], { kind: 'comment' }>;
		const old = draftOf(aReview(), comment({ ...aFinding(), body: { ...finding, side: 'old', startLine: 3 } }));
		expect(old.side).toBe('LEFT');
		expect(old.startSide).toBe('LEFT');
	});

	it('is added once, applied when the agent applies it, and taken out when dismissed', () => {
		const record = emptyRecord();
		expect(syncRecord(record, aReview())).toBe(true);
		expect(record.drafts).toHaveLength(2);
		expect(syncRecord(record, aReview())).toBe(false);

		const applied = aReview({ proposals: [aFinding({ state: 'applied' }), aFinding({ id: 'p2-1', state: 'dismissed' })] });
		expect(syncRecord(record, applied)).toBe(true);
		expect(record.drafts.map((d) => d.agent?.state)).toEqual(['applied']);
	});

	it('is not brought back once the person decided it and it left', () => {
		const record = emptyRecord();
		expect(syncRecord(record, aReview({ proposals: [aFinding({ state: 'accepted', decidedBy: 'person' })] }))).toBe(false);
		expect(record.drafts).toEqual([]);
	});

	it('only decided ones are sent, marked as drafted unless the repository says not to', () => {
		const proposed = draftOf(aReview(), comment());
		const accepted: PendingComment = { ...proposed, id: 'x', agent: { ...proposed.agent!, state: 'accepted' } };
		const edited: PendingComment = { ...proposed, id: 'y', agent: { ...proposed.agent!, state: 'edited' } };
		expect(sendable([proposed, accepted, edited]).map((d) => d.id)).toEqual(['x', 'y']);
		expect(marked('Rename it.', accepted.agent, true)).toBe('Rename it.\n\n_Drafted with Claude Code_');
		expect(marked('Rename it.', edited.agent, true)).toBe('Rename it.');
		expect(marked('Rename it.', accepted.agent, false)).toBe('Rename it.');
		expect(reviewBody('', [accepted], true)).toBe('1 of these comments was drafted with Claude Code.');
		expect(reviewBody('Looks right.', [accepted, { ...accepted, id: 'z' }], true)).toBe(
			'Looks right.\n\n2 of these comments were drafted with Claude Code.'
		);
		expect(reviewBody('Looks right.', [accepted], false)).toBe('Looks right.');
		expect(reviewBody('Looks right.', [], true)).toBe('Looks right.');
	});
});

describe('the last act at Unattended', () => {
	it('sends the review as the person, with the agent’s comments, and says it went', async () => {
		vi.mocked(api.submitReview).mockResolvedValue(undefined);
		// What the agent applied is already in the record by its last act.
		await review.saveRecord(KEY, (record) => {
			record.drafts = [{ ...draftOf(aReview(), comment()), agent: { ...draftOf(aReview(), comment()).agent!, state: 'applied' } }];
		});
		work.follow();
		const sending = aReview({
			repo: '/work/spagitty',
			state: 'working',
			proposals: [aFinding({ state: 'applied' })],
			lastAct: { kind: 'send', verdict: 'comment', body: 'Caches avatars on disk.' }
		});
		agents.absorb(sending);
		await vi.waitFor(() => expect(api.submitReview).toHaveBeenCalled());
		const [number, verdict, body, drafts] = vi.mocked(api.submitReview).mock.calls[0];
		expect([number, verdict]).toEqual([214, 'comment']);
		expect(body).toContain('Caches avatars on disk.');
		expect(drafts).toHaveLength(1);
		await vi.waitFor(() => expect(agentsApi.control).toHaveBeenCalledWith('review-1', { kind: 'acted', ok: true }));
		// Never twice.
		agents.absorb(sending);
		expect(api.submitReview).toHaveBeenCalledTimes(1);
	});

	it('refuses, with the reason, when the repository is not the one open', async () => {
		work.follow();
		agents.absorb(aReview({ repo: '/somewhere/else', lastAct: { kind: 'send', verdict: 'comment', body: '' } }));
		await vi.waitFor(() =>
			expect(agentsApi.control).toHaveBeenCalledWith('review-1', {
				kind: 'acted',
				ok: false,
				message: 'The repository was closed before the review could be sent.'
			})
		);
		expect(api.submitReview).not.toHaveBeenCalled();
	});

	it('reports a send that failed', async () => {
		vi.mocked(api.submitReview).mockRejectedValue('host said no');
		work.follow();
		agents.absorb(aReview({ repo: '/work/spagitty', lastAct: { kind: 'send', verdict: 'approve', body: '' } }));
		await vi.waitFor(() =>
			expect(agentsApi.control).toHaveBeenCalledWith('review-1', expect.objectContaining({ kind: 'acted', ok: false }))
		);
	});
});

describe('a moved head', () => {
	it('is told to a live assignment once', () => {
		agents.reset(aSnapshot(), [aReview({ state: 'working' })]);
		const pr = request({ number: 214, headSha: 'ccc' });
		work.watchHeads([pr]);
		work.watchHeads([pr]);
		expect(agentsApi.control).toHaveBeenCalledTimes(1);
		expect(agentsApi.control).toHaveBeenCalledWith('review-1', { kind: 'moved' });
	});

	it('is not, when the head is the one it works on', () => {
		agents.reset(aSnapshot(), [aReview({ state: 'working' })]);
		work.watchHeads([request({ number: 214, headSha: 'bbb' })]);
		expect(agentsApi.control).not.toHaveBeenCalled();
	});
});

describe('assigning', () => {
	it('fetches the head as the room does and hands over the description, threads and checks', async () => {
		vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h9', base: 'b9', mergeBase: 'm9' });
		vi.mocked(api.pullRequestComments).mockResolvedValue([
			{ id: 1, inReplyTo: null, path: 'a.rs', line: 3, side: 'RIGHT', body: 'Why a Map?', author: 'nour.h', createdAt: 1, resolved: false },
			{ id: 2, inReplyTo: 1, path: 'a.rs', line: 3, side: 'RIGHT', body: 'reply', author: 'y', createdAt: 2, resolved: false }
		]);
		const pr = request({ number: 214, title: 'Cache avatars', body: 'Avatars live on disk now.', checks: 'failing', targetBranch: 'main' });
		await work.assign(pr, KEY, { agent: 'claude', level: 'stepByStep', note: 'mind the cache', lands: false });
		const sent = vi.mocked(agentsApi.start).mock.calls[0][0];
		expect(sent.target).toMatchObject({ kind: 'review', number: 214, base: 'm9', head: 'h9', target: 'main' });
		expect(sent.work).toMatchObject({ job: 'review', description: 'Avatars live on disk now.', checks: 'checks failing' });
		expect(sent.work.job === 'review' && sent.work.threads).toEqual([
			{ path: 'a.rs', line: 3, author: 'nour.h', body: 'Why a Map?', resolved: false }
		]);
		expect(sent).toMatchObject({ repo: '/work/spagitty', agent: 'claude', note: 'mind the cache', resume: null });
	});

	it('resumes with the same agent and level, from the record', async () => {
		vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h9', base: 'b9', mergeBase: 'm9' });
		vi.mocked(api.pullRequestComments).mockResolvedValue([]);
		await work.resume(aReview({ state: 'stopped', level: 'signOff' }), request({ number: 214 }), KEY);
		expect(vi.mocked(agentsApi.start).mock.calls[0][0]).toMatchObject({ agent: 'claude', level: 'signOff', resume: 'review-1' });
	});
});

describe('deciding in the room', () => {
	it('accepts, edits, dismisses and asks why, telling the agent each time', async () => {
		agents.reset(aSnapshot(), [aReview()]);
		await review.saveRecord(KEY, (record) => {
			record.drafts = [draftOf(aReview(), comment()), draftOf(aReview(), comment(aFinding({ id: 'p2-1' })))];
		});
		const [first, second] = review.recordAt(KEY)!.drafts;

		await work.accept(first);
		expect(review.recordAt(KEY)!.drafts[0].agent?.state).toBe('accepted');
		await work.edit(second, 'In my words.');
		expect(review.recordAt(KEY)!.drafts[1]).toMatchObject({ body: 'In my words.', agent: { state: 'edited' } });
		work.askWhy(second);
		await work.dismiss(review.recordAt(KEY)!.drafts[1]);
		expect(review.recordAt(KEY)!.drafts).toHaveLength(1);
		expect(vi.mocked(agentsApi.control).mock.calls.map((c) => c[1])).toEqual([
			{ kind: 'decide', proposal: 'p2-0', state: 'accepted' },
			{ kind: 'decide', proposal: 'p2-1', state: 'edited' },
			{ kind: 'askWhy', proposal: 'p2-1' },
			{ kind: 'decide', proposal: 'p2-1', state: 'dismissed' }
		]);
	});

	it('reads the agent’s answer to Why', () => {
		const a = aReview({ proposals: [aFinding({ why: 'The write is not atomic.' })] });
		agents.reset(aSnapshot(), [a]);
		expect(work.whyOf(draftOf(a, comment()))).toBe('The write is not atomic.');
	});
});
