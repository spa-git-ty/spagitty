// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, render } from '../../testing/mount';
import type {
	FileDiff,
	ForgeRepo,
	PullRequest,
	PullRequestComment,
	PullRequestCommit
} from '$lib/types';

vi.mock('$lib/api', () => ({
	inTauri: vi.fn(() => true),
	forgeRepo: vi.fn(),
	forgeAccounts: vi.fn(() => Promise.resolve([{ kind: 'gitHub', host: 'github.com', user: 'ada' }])),
	pullRequests: vi.fn(),
	pullRequestFiles: vi.fn(),
	pullRequestCommits: vi.fn(),
	pullRequestComments: vi.fn(),
	commitFiles: vi.fn(),
	submitReview: vi.fn(),
	replyComment: vi.fn(),
	mergePullRequest: vi.fn(),
	closePullRequest: vi.fn(),
	setPrDraft: vi.fn()
}));

vi.mock('$app/navigation', () => ({ goto: vi.fn(() => Promise.resolve()) }));
vi.mock('$lib/review/store.svelte', () => ({ review: { open: vi.fn(() => Promise.resolve(true)), notHere: null } }));

import * as api from '$lib/api';
import { goto } from '$app/navigation';
import { review } from '$lib/review/store.svelte';
import PRDiffPane from './PRDiffPane.svelte';
import PRMarkdown from './PRMarkdown.svelte';
import PRWorkspace from './PRWorkspace.svelte';
import { requests } from './store.svelte';
import { notice } from '$lib/ui/notice.svelte';

const forgeRepo = vi.mocked(api.forgeRepo);
const pullRequests = vi.mocked(api.pullRequests);
const pullRequestFiles = vi.mocked(api.pullRequestFiles);
const pullRequestCommits = vi.mocked(api.pullRequestCommits);
const pullRequestComments = vi.mocked(api.pullRequestComments);
const commitFiles = vi.mocked(api.commitFiles);
const submitReview = vi.mocked(api.submitReview);
const replyComment = vi.mocked(api.replyComment);
const mergePullRequest = vi.mocked(api.mergePullRequest);
const closePullRequest = vi.mocked(api.closePullRequest);
const setPrDraft = vi.mocked(api.setPrDraft);

const REPO: ForgeRepo = {
	kind: 'gitHub',
	host: 'github.com',
	owner: 'spagitty',
	name: 'spagitty'
};

function request(overrides: Partial<PullRequest> = {}): PullRequest {
	return {
		id: 'PR_1',
		number: 412,
		title: 'Workspace review overhaul',
		body: '## Overview\n\nThis PR overhauls the review workspace.',
		authorName: 'ada',
		updated: 1_787_650_200,
		sourceBranch: 'feature/workspace',
		targetBranch: 'main',
		draft: false,
		review: 'awaitingReview',
		checks: 'passing',
		needsYou: false,
		needsYouBecause: null,
		changedFiles: 2,
		added: 25,
		removed: 5,
		mergeable: true,

		headSha: 'a1b2c3d4e5f6',

		reviewRequested: false,

		openThreads: 0,

		resolvedThreads: 0,

		repliesToYou: 0,

		repository: null,
		...overrides
	};
}

function file(path = 'src/main.rs'): FileDiff {
	return {
		path,
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 10,
		removed: 2,
		hunks: [
			{
				oldStart: 1,
				oldLines: 3,
				newStart: 1,
				newLines: 4,
				header: '@@ -1,3 +1,4 @@',
				lines: [
					{ origin: 'context', old: 1, new: 1, text: 'fn main() {' },
					{ origin: 'removed', old: 2, new: null, text: '    old();' },
					{ origin: 'added', old: null, new: 2, text: '    new_one();' },
					{ origin: 'added', old: null, new: 3, text: '    new_two();' },
					{ origin: 'context', old: 3, new: 4, text: '}' }
				]
			}
		]
	};
}

function commit(sha = 'abc123456789'): PullRequestCommit {
	return {
		sha,
		short: sha.slice(0, 7),
		summary: 'Refactor diff and workspace components',
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		time: 1_787_650_200
	};
}

function comment(id = 101): PullRequestComment {
	return {
		id,
		inReplyTo: null,
		path: 'src/main.rs',
		line: 2,
		side: 'RIGHT',
		body: 'Please rename new_one()',
		author: 'grace',
		createdAt: 1_787_650_200,
		resolved: false
	};
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
	vi.clearAllMocks();
	requests.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
	forgeRepo.mockResolvedValue(REPO);
	pullRequests.mockResolvedValue([request()]);
	pullRequestFiles.mockResolvedValue([file()]);
	pullRequestCommits.mockResolvedValue([commit()]);
	pullRequestComments.mockResolvedValue([comment()]);
	commitFiles.mockResolvedValue([file('src/commit-specific.rs')]);
});

describe('PR workspace store flow', () => {
	it('opens workspace and loads PR data', async () => {
		requests.present([request()]);
		requests.openWorkspace('PR_1');

		expect(requests.viewMode).toBe('workspace');
		expect(requests.openId).toBe('PR_1');

		await requests.loadWorkspaceData();

		expect(pullRequestFiles).toHaveBeenCalledWith(412);
		expect(pullRequestCommits).toHaveBeenCalledWith(412);
		expect(pullRequestComments).toHaveBeenCalledWith(412);
		expect(requests.files).toHaveLength(1);
		expect(requests.commits).toHaveLength(1);
		expect(requests.comments).toHaveLength(1);
	});

	it('closes workspace back to list view', () => {
		requests.present([request()]);
		requests.openWorkspace('PR_1');
		expect(requests.viewMode).toBe('workspace');

		requests.closeWorkspace();
		expect(requests.viewMode).toBe('list');
	});

	it('selects commit and loads commit files into cache', async () => {
		requests.present([request()]);
		requests.presentFiles([file('src/main.rs')], 412);

		await requests.selectCommit('abc123456789');

		expect(requests.selectedCommitSha).toBe('abc123456789');
		expect(commitFiles).toHaveBeenCalledWith('abc123456789');
		expect(requests.currentFiles[0].path).toBe('src/commit-specific.rs');

		await requests.selectCommit(null);
		expect(requests.selectedCommitSha).toBeNull();
		expect(requests.currentFiles[0].path).toBe('src/main.rs');
	});

	it('adds and removes draft comments', () => {
		requests.addDraftComment('src/main.rs', 2, 'RIGHT', 'Nice clean refactor');
		expect(requests.draftComments).toHaveLength(1);
		expect(requests.draftComments[0].body).toBe('Nice clean refactor');

		requests.removeDraftComment('src/main.rs', 2, 'RIGHT');
		expect(requests.draftComments).toHaveLength(0);
	});

	it('preserves draft comments on review failure', async () => {
		requests.present([request()]);
		requests.addDraftComment('src/main.rs', 2, 'RIGHT', 'Important feedback');

		submitReview.mockRejectedValueOnce('network timeout');

		const ok = await requests.review('comment', 'Summary note');
		expect(ok).toBe(false);
		expect(requests.reviewError).toContain('network timeout');
		expect(requests.draftComments).toHaveLength(1);
		expect(requests.draftComments[0].body).toBe('Important feedback');
	});

	it('submits review with draft comments included', async () => {
		requests.present([request()]);
		requests.addDraftComment('src/main.rs', 2, 'RIGHT', 'Inline review note');

		submitReview.mockResolvedValueOnce();

		const ok = await requests.review('comment', 'Summary note');
		expect(ok).toBe(true);
		expect(submitReview).toHaveBeenCalledWith(
			412,
			'comment',
			'Summary note',
			[{ path: 'src/main.rs', line: 2, side: 'RIGHT', body: 'Inline review note' }]
		);
		expect(requests.draftComments).toHaveLength(0);
	});

	it('resolves comment threads', () => {
		requests.presentComments([comment(101)], 412);
		expect(requests.comments[0].resolved).toBe(false);

		requests.resolveComment(101);
		expect(requests.comments[0].resolved).toBe(true);
	});

	it('replies to comment thread', async () => {
		requests.present([request()]);
		requests.presentComments([comment(101)], 412);

		replyComment.mockResolvedValueOnce({
			id: 102,
			inReplyTo: 101,
			path: 'src/main.rs',
			line: 2,
			side: 'RIGHT',
			body: 'Done in recent commit',
			author: 'ada',
			createdAt: 1_787_650_300,
			resolved: false
		});

		const ok = await requests.replyToComment(101, 'Done in recent commit');
		expect(ok).toBe(true);
		expect(replyComment).toHaveBeenCalledWith(412, 101, 'Done in recent commit');
		expect(requests.comments).toHaveLength(2);
	});

	it('determines role from author vs connected user', async () => {
		await requests.load();
		requests.present([request({ authorName: 'ada' })]);
		expect(requests.role).toBe('developer');

		requests.present([request({ authorName: 'grace' })]);
		expect(requests.role).toBe('reviewer');
	});

	it('merges a pull request via store (FEAT-071)', async () => {
		requests.present([request()]);
		mergePullRequest.mockResolvedValueOnce(undefined);
		pullRequests.mockResolvedValue([]);

		const ok = await requests.merge('squash', 'My title', 'My message');
		expect(ok).toBe(true);
		expect(mergePullRequest).toHaveBeenCalledWith(412, 'squash', 'My title', 'My message');
	});

	it('closes a pull request via store (FEAT-071)', async () => {
		requests.present([request()]);
		closePullRequest.mockResolvedValueOnce(undefined);
		pullRequests.mockResolvedValue([]);

		const ok = await requests.close();
		expect(ok).toBe(true);
		expect(closePullRequest).toHaveBeenCalledWith(412);
	});

	it('goes back to the list when the open one is merged, not to another (BUG-062)', async () => {
		pullRequests.mockResolvedValue([request(), request({ id: 'PR_2', number: 2 })]);
		await requests.load();
		requests.openWorkspace('PR_1');
		mergePullRequest.mockResolvedValueOnce(undefined);
		pullRequests.mockResolvedValue([request({ id: 'PR_2', number: 2 })]);

		expect(await requests.merge('merge')).toBe(true);

		expect(requests.viewMode).toBe('list');
		expect(requests.openId).toBe('PR_2');
	});

	it('goes back to the list when the open one is closed (BUG-062)', async () => {
		pullRequests.mockResolvedValue([request(), request({ id: 'PR_2', number: 2 })]);
		await requests.load();
		requests.openWorkspace('PR_1');
		closePullRequest.mockResolvedValueOnce(undefined);
		pullRequests.mockResolvedValue([request({ id: 'PR_2', number: 2 })]);

		expect(await requests.close()).toBe(true);

		expect(requests.viewMode).toBe('list');
	});

	it('reads what another pull request opened in the workspace holds (BUG-062)', async () => {
		requests.present([request()]);
		requests.openWorkspace('PR_1');
		await requests.loadWorkspaceData();
		vi.clearAllMocks();

		// Nothing was open, then the list arrives: the one it lands on is read.
		requests.clear();
		requests.openWorkspace();
		requests.present([request({ id: 'PR_2', number: 2 })]);
		await settle();

		expect(requests.viewMode).toBe('workspace');
		expect(pullRequestFiles).toHaveBeenCalledWith(2);
		expect(pullRequestCommits).toHaveBeenCalledWith(2);
		expect(requests.files).toHaveLength(1);
		expect(requests.commits).toHaveLength(1);
	});

	it('says the merged one was merged (BUG-062)', async () => {
		pullRequests.mockResolvedValue([request(), request({ id: 'PR_2', number: 2 })]);
		await requests.load();
		requests.openWorkspace('PR_1');
		mergePullRequest.mockResolvedValueOnce(undefined);
		pullRequests.mockResolvedValue([request({ id: 'PR_2', number: 2 })]);

		const view = render(PRWorkspace, {});
		click(view.all('button').find((b) => b.textContent?.trim() === 'Merge')!);
		await settle();
		click(view.all('button').find((b) => b.textContent?.trim() === 'Confirm Merge')!);
		await settle();
		await settle();

		expect(notice.current?.title).toBe('#412 merged');
		expect(requests.viewMode).toBe('list');
	});

	it('toggles draft status via store (FEAT-071)', async () => {
		await requests.load();
		requests.present([request({ draft: false })]);
		setPrDraft.mockResolvedValueOnce(undefined);
		pullRequests.mockResolvedValue([request({ draft: true })]);

		const ok = await requests.toggleDraft();
		expect(ok).toBe(true);
		// The node id travels with the number: GitHub converts draft state over
		// GraphQL and cannot address a pull request by its number.
		expect(setPrDraft).toHaveBeenCalledWith(412, 'PR_1', 'Workspace review overhaul', true);
	});

	it('offers no draft toggle on Bitbucket (FEAT-071)', async () => {
		// Bitbucket Cloud has no draft pull request. Writing `Draft:` into the
		// title would leave a state the host does not know it is in.
		forgeRepo.mockResolvedValue({ ...REPO, kind: 'bitbucket', host: 'bitbucket.org' });
		await requests.load();

		expect(requests.canDraft).toBe(false);
		expect(await requests.toggleDraft()).toBe(false);
		expect(setPrDraft).not.toHaveBeenCalled();

		requests.select('PR_1');
		const view = render(PRWorkspace, {});
		const draftBtn = view
			.all('button')
			.find((b) => b.textContent?.trim() === 'Draft' || b.textContent?.trim() === 'Mark Ready');
		expect(draftBtn).toBeUndefined();
	});

	it('offers the draft toggle on GitHub (FEAT-071)', async () => {
		// The other half of the check above: the control disappears because the
		// host has no draft, not because the markup stopped rendering it.
		await requests.load();
		requests.select('PR_1');

		const view = render(PRWorkspace, {});
		const draftBtn = view.all('button').find((b) => b.textContent?.trim() === 'Draft');
		expect(draftBtn).toBeDefined();
	});

	it('reports merge error without crashing (FEAT-071)', async () => {
		requests.present([request()]);
		mergePullRequest.mockRejectedValueOnce(new Error('not mergeable'));

		const ok = await requests.merge('merge');
		expect(ok).toBe(false);
		expect(requests.mergeError).toContain('not mergeable');
	});
});

describe('PRWorkspace component UI', () => {
	it('mounts and renders PR header, accordion panes, and controls', () => {
		vi.mocked(api.inTauri).mockReturnValue(false);
		requests.present([request()]);
		requests.select('PR_1');
		requests.presentFiles([file('src/main.rs')], 412);
		requests.presentCommits([commit()], 412);
		requests.presentComments([comment()], 412);

		const view = render(PRWorkspace, {});

		expect(view.text()).toContain('#412');
		expect(view.text()).toContain('Workspace review overhaul');
		expect(view.text()).toContain('ada');
		expect(view.text()).toContain('CHANGELOG');
		expect(view.text()).toContain('All Changed Files');
		expect(view.text()).toContain('List Of Commits');
		expect(view.text()).toContain('src/main.rs');
		expect(view.text()).toContain('Refactor diff and workspace components');

		// Toggle role preview
		expect(view.text()).toContain('Reviewer View');
		const roleBtn = view.get('.role-toggle-btn');
		click(roleBtn);
		expect(view.text()).toContain('Developer View');

		// Click changelog tab to view PR markdown description
		const changelogBtn = view.get('.changelog-btn');
		click(changelogBtn);
		expect(requests.openPath).toBe('__changelog__');
		expect(view.text()).toContain('Pull Request Description');

		// Toggle accordion
		const headers = view.all('.accordion-header');
		click(headers[0]); // collapse all files
		click(headers[1]); // collapse commits

		// Toggle commit expansion via full commit row button
		click(headers[1]); // expand commits again
		const commitRowBtn = view.get('.commit-row-btn');
		click(commitRowBtn);

		view.destroy();
	});

	it('opens and closes review modal in reviewer mode', () => {
		vi.mocked(api.inTauri).mockReturnValue(false);
		requests.present([request({ authorName: 'grace' })]);
		requests.select('PR_1');
		requests.presentFiles([file('src/main.rs')], 412);

		const view = render(PRWorkspace, {});

		const publishBtn = view.all('button').find((b) => b.textContent?.includes('Publish Review'));
		expect(publishBtn).toBeDefined();
		if (publishBtn) click(publishBtn);

		expect(view.text()).toContain('Publish Pull Request Review');
		expect(view.text()).toContain('Approve');
		expect(view.text()).toContain('Request Changes');

		const cancelBtn = view.all('button').find((b) => b.textContent?.includes('Cancel'));
		expect(cancelBtn).toBeDefined();
		if (cancelBtn) click(cancelBtn);

		expect(view.text()).not.toContain('Publish Pull Request Review');
		view.destroy();
	});
});

describe('PRDiffPane component UI', () => {
	it('renders diff lines, comments, and draft composers', () => {
		vi.mocked(api.inTauri).mockReturnValue(false);
		requests.presentComments([comment(101)], 412);
		requests.addDraftComment('src/main.rs', 2, 'RIGHT', 'Draft inline note');

		const view = render(PRDiffPane, {
			file: file('src/main.rs'),
			path: 'src/main.rs',
			error: null,
			loading: false,
			view: 'unified' as const
		});

		expect(view.text()).toContain('fn main()');
		expect(view.text()).toContain('new_one()');
		expect(view.text()).toContain('Please rename new_one()');
		expect(view.text()).toContain('Draft inline note');
		expect(view.text()).toContain('Pending Review Draft');

		// Open inline composer
		const triggers = view.all('.comment-trigger');
		expect(triggers.length).toBeGreaterThan(0);
		click(triggers[0]);

		expect(view.text()).toContain('Add inline review comment');

		view.destroy();
	});
});

describe('PRMarkdown component', () => {
	it('renders empty message when markdown is empty', () => {
		const view = render(PRMarkdown, { markdown: '' });
		expect(view.text()).toContain('No description or changelog provided');
		view.destroy();
	});

	it('renders headings, code blocks, lists, and formatted text', () => {
		const sample = `# Title\n\n## Subheading\n\nSome **bold** and \`inline_code\` and a [link](https://example.com).\n\n\`\`\`rust\nfn hello() {}\n\`\`\`\n\n- [x] Done task\n- [ ] Todo task\n\n> Important quote`;
		const view = render(PRMarkdown, { markdown: sample });

		expect(view.text()).toContain('Title');
		expect(view.text()).toContain('Subheading');
		expect(view.text()).toContain('bold');
		expect(view.text()).toContain('inline_code');
		expect(view.text()).toContain('fn hello() {}');
		expect(view.text()).toContain('Done task');
		expect(view.text()).toContain('Todo task');
		expect(view.text()).toContain('Important quote');

		view.destroy();
	});
});

/** FEAT-105: the pull request screen hands a review to the review room. */
describe('Review from the pull request screen', () => {
	it('opens this pull request in the review room', async () => {
		vi.mocked(api.inTauri).mockReturnValue(false);
		requests.present([request()]);
		requests.select('PR_1');
		const view = render(PRWorkspace, {});

		const button = view.all('button').find((b) => b.textContent?.trim() === 'Review');
		expect(button).toBeDefined();
		click(button as HTMLElement);
		await Promise.resolve();
		await Promise.resolve();

		expect(review.open).toHaveBeenCalledWith(expect.objectContaining({ number: 412 }));
		expect(goto).toHaveBeenCalledWith('/review');
		view.destroy();
	});
});
