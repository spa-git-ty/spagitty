// SPDX-License-Identifier: GPL-3.0-or-later
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render } from '../../testing/mount';
import type { Hosted, Issue, Metadata, Page, Thread } from './types';

vi.mock('./api', () => ({ remotes: vi.fn(), cached: vi.fn(), page: vi.fn(), metadata: vi.fn(), thread: vi.fn(), write: vi.fn(), branch: vi.fn() }));
vi.mock('$lib/notifications/watcher.svelte', () => ({ watcher: { subscribe: vi.fn(() => vi.fn()) } }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation', () => ({ goto: vi.fn(async () => {}) }));
import * as api from './api';
import { issues } from './store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import IssuesScreen from './IssuesScreen.svelte';
import TaskEditor from '$lib/farm/components/TaskEditor.svelte';
import { issueHandoff } from './handoff.svelte';

const host: Hosted = { remote: 'origin', repo: { kind: 'gitHub', host: 'github.com', owner: 'team', name: 'app' }, user: 'ada' };
const issue = (number = 1): Issue => ({ id: String(number), number, title: `Refund button ${number}`, body: '**Broken** for partial refunds.', state: 'open', author: 'lin', created: '2026-10-14', updated: '2026-10-14', url: `https://github.com/team/app/issues/${number}`, replies: 1, labels: [{ name: 'bug', color: 'cc0000' }], assignees: ['ada'], milestone: null, locked: false, branchSuggestion: `fix/${number}-refund-button` });
const page = (values = [issue()], more = false): Page => ({ issues: values, page: 1, more, etag: 'tag', canWrite: true });
const thread = (number = 1): Thread => ({ issue: issue(number), comments: [{ id: 'c', author: 'ada', body: 'Reproduced', at: '2026-10-14', url: '' }], events: [], commentsMore: false, eventsMore: false });
const meta: Metadata = { defaultBranch: 'main', open: 42, closed: 12, labels: [{ name: 'bug', color: 'cc0000' }], milestones: [] };
const settle = async () => { await new Promise((resolve) => setTimeout(resolve, 0)); flushSync(); };
beforeEach(() => {
	vi.clearAllMocks(); issues.clear(); dialog.dismiss(); issueHandoff.clear();
	vi.mocked(api.remotes).mockResolvedValue([host]); vi.mocked(api.cached).mockResolvedValue(null);
	vi.mocked(api.page).mockResolvedValue({ data: page(), cacheWarning: null });
	vi.mocked(api.metadata).mockResolvedValue({ data: meta, cacheWarning: null });
	vi.mocked(api.thread).mockImplementation(async (_, number) => thread(number));
	vi.mocked(api.write).mockResolvedValue({}); vi.mocked(api.branch).mockResolvedValue();
});

describe('issue state', () => {
	it('paints the cached page while the network is pending and retains it offline', async () => {
		vi.mocked(api.cached).mockResolvedValue(page());
		let refuse!: (reason: string) => void; vi.mocked(api.page).mockImplementationOnce(() => new Promise((_, reject) => { refuse = reject; }));
		const loading = issues.load('/repo'); await settle();
		expect(issues.rows[0].title).toContain('Refund'); expect(issues.provisional).toBe(true);
		refuse('Offline'); await loading; expect(issues.rows).toHaveLength(1); expect(issues.errors.list).toContain('Offline');
	});
	it('drops old repository and old issue replies', async () => {
		await issues.load('/repo');
		let answer!: (value: Thread) => void; vi.mocked(api.thread).mockImplementationOnce(() => new Promise((resolve) => { answer = resolve; }));
		const old = issues.open(1); await issues.open(2); answer(thread(1)); await old; expect(issues.current?.issue.number).toBe(2);
		let pages!: (value: { data: Page; cacheWarning: null }) => void; vi.mocked(api.page).mockImplementationOnce(() => new Promise((resolve) => { pages = resolve; }));
		const before = issues.filter('is:closed'); await settle(); await issues.load('/other'); pages({ data: page([issue(99)]), cacheWarning: null }); await before;
		expect(issues.rows[0].number).toBe(1);
	});
	it('requires each confirmation and keeps a failed write visible after refresh', async () => {
		await issues.load('/repo'); await issues.open(1);
		const cancelled = issues.send({ kind: 'close', number: 1 }); dialog.dismiss(); await cancelled; expect(api.write).not.toHaveBeenCalled();
		vi.mocked(api.write).mockRejectedValueOnce('Delivery uncertain. Refresh before sending again.');
		const sending = issues.send({ kind: 'comment', number: 1, body: 'Please fix' }); dialog.accept(); expect(await sending).toBe(false);
		await issues.refresh(); expect(issues.errors.write).toContain('Delivery uncertain'); expect(api.write).toHaveBeenCalledTimes(1);
	});
	it('hides writes for a read-only token and never sends after changing repository at confirmation', async () => {
		vi.mocked(api.page).mockResolvedValueOnce({ data: { ...page(), canWrite: false }, cacheWarning: null });
		await issues.load('/repo'); expect(issues.canWrite).toBe(false);
		await issues.load('/other'); const writing = issues.send({ kind: 'close', number: 1 });
		await issues.load('/third'); dialog.accept(); await writing; expect(api.write).not.toHaveBeenCalled();
	});
	it('close with comment confirms the comment and the close separately', async () => {
		await issues.load('/repo'); await issues.open(1);
		const sending = issues.closeWithComment('Fixed'); dialog.accept(); await settle();
		expect(api.write).toHaveBeenCalledTimes(1); expect(dialog.question?.title).toContain('Close');
		dialog.accept(); expect(await sending).toBe(true); expect(api.write).toHaveBeenLastCalledWith('origin', { kind: 'close', number: 1 });
	});
	it('appends another page without duplicating an overlapping row', async () => {
		vi.mocked(api.page).mockResolvedValueOnce({ data: page([issue(1)], true), cacheWarning: null }).mockResolvedValueOnce({ data: { ...page([issue(1), issue(2)]), page: 2 }, cacheWarning: null });
		await issues.load('/repo'); await issues.read(); expect(issues.rows.map((row) => row.number)).toEqual([1, 2]);
		expect(api.page).toHaveBeenLastCalledWith('origin', 'is:open', 2);
	});
});

describe('mounted Issues', () => {
	it('selects a virtual row, draws safe Markdown, and posts a reviewed comment', async () => {
		await issues.load('/repo'); const view = render(IssuesScreen, {});
		click(view.get('.issue-row')); await settle(); expect(view.text()).toContain('Reproduced'); expect(view.get('.md strong').textContent).toBe('Broken');
		const field = view.get('[aria-label="Issue comment"]') as HTMLTextAreaElement; field.value = 'I can fix it'; fire(field, 'input');
		click(view.all('button').find((button) => button.textContent?.trim() === 'Comment')!); expect(api.write).not.toHaveBeenCalled(); dialog.accept(); await settle();
		expect(api.write).toHaveBeenCalledWith('origin', { kind: 'comment', number: 1, body: 'I can fix it' }); view.destroy();
	});
	it('creates an issue through the actual fields and keeps its draft when confirmation is cancelled', async () => {
		await issues.load('/repo'); const view = render(IssuesScreen, {});
		click(view.all('button').find((button) => button.textContent?.trim() === 'New issue')!);
		const title = view.get('[aria-label="New issue title"]') as HTMLInputElement; title.value = 'Broken refunds'; fire(title, 'input');
		const body = view.get('[aria-label="New issue description"]') as HTMLTextAreaElement; body.value = 'Steps to reproduce'; fire(body, 'input');
		click(view.all('button').find((button) => button.textContent?.trim() === 'Create issue')!); dialog.dismiss(); await settle(); expect(title.value).toBe('Broken refunds');
		click(view.all('button').find((button) => button.textContent?.trim() === 'Create issue')!); dialog.accept(); await settle();
		expect(api.write).toHaveBeenCalledWith('origin', { kind: 'create', title: 'Broken refunds', body: 'Steps to reproduce' }); view.destroy();
	});
	it('applies real filter controls and keeps mutation controls absent without write access', async () => {
		vi.mocked(api.page).mockResolvedValue({ data: { ...page(), canWrite: false }, cacheWarning: null });
		await issues.load('/repo'); const view = render(IssuesScreen, {});
		expect(view.text()).not.toContain('New issue');
		click(view.all('.filters button').find((button) => button.textContent?.includes('Closed'))!); await settle();
		expect(api.page).toHaveBeenLastCalledWith('origin', 'is:closed', 1); view.destroy();
	});
	it('hands a complete brief to a fresh task editor without starting or saving work', () => {
		issueHandoff.prepare('/repo', thread()); expect(issueHandoff.take('/other')).toBeNull();
		const initial = issueHandoff.take('/repo')!; expect(initial.description).toContain('Reproduced'); expect(initial.ready).toBe(false); expect(issueHandoff.take('/repo')).toBeNull();
		const onsave = vi.fn(); const view = render(TaskEditor, { task: null, initial, candidates: [], busy: false, onsave, oncancel: vi.fn() });
		expect((view.get('input') as HTMLInputElement).value).toBe('#1 Refund button 1'); expect((view.get('textarea') as HTMLTextAreaElement).value).toContain('Reproduced'); expect(onsave).not.toHaveBeenCalled(); view.destroy();
	});
});
