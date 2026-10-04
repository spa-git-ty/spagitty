// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A pull request's files, and the review left on it (FEAT-058).
 *
 * The store's job here is everything that is not the network: not showing one
 * pull request's diff under another's title, not spending a rate limit on a
 * re-render, and not letting a slow read land after the reader has moved on.
 * Each of those is a test rather than a comment.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import type { FileDiff, ForgeRepo, PullRequest } from '$lib/types';

// The confirmation is a gate on an action that writes to somebody else's
// server, so both sides of it are driven here rather than left to the sweep.
vi.mock('$lib/ui/dialog.svelte', () => ({
	dialog: { confirm: vi.fn() }
}));

vi.mock('$lib/api', () => ({
	inTauri: vi.fn(() => true),
	forgeRepo: vi.fn(),
	forgeAccounts: vi.fn(() => Promise.resolve([])),
	pullRequests: vi.fn(),
	pullRequestFiles: vi.fn(),
	pullRequestCommits: vi.fn(),
	pullRequestComments: vi.fn(),
	commitFiles: vi.fn(),
	submitReview: vi.fn(),
	replyComment: vi.fn()
}));

import * as api from '$lib/api';
import { dialog } from '$lib/ui/dialog.svelte';
import RequestDetail from './RequestDetail.svelte';
import { requests } from './store.svelte';

const forgeRepo = vi.mocked(api.forgeRepo);
const pullRequests = vi.mocked(api.pullRequests);
const pullRequestFiles = vi.mocked(api.pullRequestFiles);
const submitReview = vi.mocked(api.submitReview);
const confirmDialog = vi.mocked(dialog.confirm);

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
		title: 'Give the graph a footer',
		body: 'PR body description',
		authorName: 'grace',
		updated: 1_787_650_200,
		sourceBranch: 'feature/footer',
		targetBranch: 'main',
		draft: false,
		review: 'awaitingReview',
		checks: 'passing',
		needsYou: false,
		needsYouBecause: null,
		changedFiles: 2,
		added: 120,
		removed: 34,
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

function file(path: string, overrides: Partial<FileDiff> = {}): FileDiff {
	return {
		path,
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 3,
		removed: 1,
		hunks: [],
		...overrides
	};
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
	vi.clearAllMocks();
	requests.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
	forgeRepo.mockResolvedValue(REPO);
	pullRequests.mockResolvedValue([request()]);
	pullRequestFiles.mockResolvedValue([file('src/a.rs'), file('src/b.rs')]);
	submitReview.mockResolvedValue(undefined);
	confirmDialog.mockResolvedValue(true);
});

describe('presenting files', () => {
	it('opens the first file, so the pane is not empty behind a list', () => {
		requests.present([request()]);
		requests.presentFiles([file('src/a.rs'), file('src/b.rs')], 412);

		expect(requests.openPath).toBe('src/a.rs');
		expect(requests.openFile?.path).toBe('src/a.rs');
	});

	it('keeps the open file when it is still in the list', () => {
		requests.present([request()]);
		requests.presentFiles([file('src/a.rs'), file('src/b.rs')], 412);
		requests.selectPath('src/b.rs');

		requests.presentFiles([file('src/a.rs'), file('src/b.rs')], 412);

		expect(requests.openPath).toBe('src/b.rs');
	});

	it('moves to the first file when the open one is gone', () => {
		requests.present([request()]);
		requests.presentFiles([file('src/a.rs'), file('src/b.rs')], 412);
		requests.selectPath('src/b.rs');

		requests.presentFiles([file('src/a.rs')], 412);

		expect(requests.openPath).toBe('src/a.rs');
	});

	it('has no open file when the pull request changes nothing', () => {
		requests.present([request()]);
		requests.presentFiles([], 412);

		expect(requests.openPath).toBeNull();
		expect(requests.openFile).toBeNull();
	});
});

describe('loading files', () => {
	it('reads the open pull request by its number', async () => {
		requests.present([request({ number: 77 })]);
		await requests.loadFiles();

		expect(pullRequestFiles).toHaveBeenCalledWith(77);
		expect(requests.files).toHaveLength(2);
	});

	it('does not read again for a pull request it has already read', async () => {
		requests.present([request()]);
		await requests.loadFiles();
		await requests.loadFiles();

		// The screen calls this from an effect that re-runs on every render.
		// A request per render spends somebody's rate limit on nothing.
		expect(pullRequestFiles).toHaveBeenCalledTimes(1);
	});

	it('reads again after a failure, since the reader asked it to', async () => {
		requests.present([request()]);
		pullRequestFiles.mockRejectedValueOnce(new Error('rate limited'));
		await requests.loadFiles();
		expect(requests.filesError).toContain('rate limited');

		await requests.loadFiles();

		expect(pullRequestFiles).toHaveBeenCalledTimes(2);
		expect(requests.filesError).toBeNull();
		expect(requests.files).toHaveLength(2);
	});

	it('reports a failure in the host words and shows no stale files', async () => {
		requests.present([request()]);
		await requests.loadFiles();
		expect(requests.files).toHaveLength(2);

		// A second pull request, whose read fails.
		requests.present([request({ id: 'PR_2', number: 500 })]);
		pullRequestFiles.mockRejectedValueOnce(new Error('this repository is private'));
		await requests.loadFiles();

		expect(requests.filesError).toContain('private');
		expect(requests.files).toHaveLength(0);
		expect(requests.openPath).toBeNull();
	});

	it('does nothing outside Tauri', async () => {
		vi.mocked(api.inTauri).mockReturnValue(false);
		requests.present([request()]);

		await requests.loadFiles();

		expect(pullRequestFiles).not.toHaveBeenCalled();
	});

	it('does nothing when no pull request is open', async () => {
		await requests.loadFiles();

		expect(pullRequestFiles).not.toHaveBeenCalled();
	});

	it('drops a read that lands after the reader has moved on', async () => {
		requests.present([request({ id: 'PR_1', number: 1 })]);

		let land: (files: FileDiff[]) => void = () => {};
		pullRequestFiles.mockImplementationOnce(
			() => new Promise<FileDiff[]>((resolve) => (land = resolve))
		);
		const slow = requests.loadFiles();

		// The reader opens a different pull request while the first read is in
		// flight, and its files arrive afterwards.
		requests.present([request({ id: 'PR_2', number: 2 })]);
		requests.select('PR_2');
		land([file('stale.rs')]);
		await slow;
		await settle();

		expect(requests.files.map((f) => f.path)).not.toContain('stale.rs');
	});
});

describe('selecting a pull request', () => {
	it('drops the previous one files rather than showing them under a new title', async () => {
		requests.present([request({ id: 'PR_1' }), request({ id: 'PR_2', number: 500 })]);
		await requests.loadFiles();
		expect(requests.files).toHaveLength(2);

		requests.select('PR_2');

		expect(requests.files).toHaveLength(0);
		expect(requests.openPath).toBeNull();
	});

	it('keeps them when the selection has not actually changed', async () => {
		requests.present([request({ id: 'PR_1' })]);
		await requests.loadFiles();

		requests.select('PR_1');

		expect(requests.files).toHaveLength(2);
	});

	it('drops them when the list arrives with a different request open', async () => {
		requests.present([request({ id: 'PR_1' })]);
		await requests.loadFiles();
		expect(requests.files).toHaveLength(2);

		// PR_1 was merged and is no longer in the list.
		requests.present([request({ id: 'PR_2', number: 500 })]);

		expect(requests.files).toHaveLength(0);
	});

	it('drops them when the read fails', async () => {
		requests.present([request()]);
		await requests.loadFiles();

		requests.fail('offline');

		expect(requests.files).toHaveLength(0);
	});
});

describe('reviewing', () => {
	it('sends the verdict and the comment for the open pull request', async () => {
		requests.present([request({ number: 77 })]);

		const landed = await requests.review('approve', 'looks right');

		expect(landed).toBe(true);
		expect(submitReview).toHaveBeenCalledWith(77, 'approve', 'looks right');
	});

	it('re-reads the list, because the review decision is the host to compute', async () => {
		requests.present([request()]);

		await requests.review('approve', '');

		// Not patched locally: a guess at the new review state here would be a
		// second source of truth for the one fact this screen exists to show.
		expect(pullRequests).toHaveBeenCalledTimes(1);
	});

	it('keeps the host refusal and says it did not land', async () => {
		requests.present([request()]);
		submitReview.mockRejectedValueOnce(new Error('you cannot review your own pull request'));

		const landed = await requests.review('approve', '');

		expect(landed).toBe(false);
		expect(requests.reviewError).toContain('your own pull request');
		// The refusal must not be mistaken for the list having failed.
		expect(requests.error).toBeNull();
	});

	it('is not left marked in flight after a failure', async () => {
		requests.present([request()]);
		submitReview.mockRejectedValueOnce(new Error('nope'));

		await requests.review('comment', 'something');

		expect(requests.reviewing).toBe(false);
	});

	it('does nothing when no pull request is open', async () => {
		const landed = await requests.review('approve', '');

		expect(landed).toBe(false);
		expect(submitReview).not.toHaveBeenCalled();
	});

	it('does nothing outside Tauri', async () => {
		vi.mocked(api.inTauri).mockReturnValue(false);
		requests.present([request()]);

		expect(await requests.review('approve', '')).toBe(false);
		expect(submitReview).not.toHaveBeenCalled();
	});
});

describe('the panel', () => {
	/** The panel with a request open and its files already read. */
	async function panel() {
		requests.present([request()]);
		await requests.loadFiles();
		const view = render(RequestDetail, {});
		flushSync();
		return view;
	}

	it('lists every changed file with its counts', async () => {
		const view = await panel();

		expect(view.text()).toContain('src/a.rs');
		expect(view.text()).toContain('src/b.rs');
		expect(view.text()).toContain('+3');
		expect(view.text()).toContain('−1');
		view.destroy();
	});

	it('marks the open file, and opens another when it is clicked', async () => {
		const view = await panel();
		expect(view.all('.file.at')).toHaveLength(1);

		const rows = view.all('.file');
		click(rows[1]);
		flushSync();

		expect(requests.openPath).toBe('src/b.rs');
		view.destroy();
	});

	it('says binary rather than showing a file that changed nothing', async () => {
		requests.present([request()]);
		pullRequestFiles.mockResolvedValue([
			file('icon.png', { binary: true, added: 0, removed: 0 })
		]);
		await requests.loadFiles();
		const view = render(RequestDetail, {});
		flushSync();

		expect(view.text()).toContain('binary');
		view.destroy();
	});

	it('shows the host own words when the files cannot be read', async () => {
		requests.present([request()]);
		// Not `...Once`: the panel's effect retries after a failure, which is the
		// behaviour the store is meant to have. A one-shot rejection would let
		// the retry succeed and assert nothing about the failed state.
		pullRequestFiles.mockRejectedValue(new Error('rate limited until 09:20'));
		await requests.loadFiles();
		const view = render(RequestDetail, {});
		// The panel's effect retries on mount, so the failed state on screen is
		// the retry's, not the first attempt's. Waiting for it is the difference
		// between asserting the error and asserting the spinner in front of it.
		await settle();
		flushSync();

		expect(view.text()).toContain('rate limited until 09:20');
		expect(view.text()).toContain('Try again');
		view.destroy();
	});

	it('says so rather than showing an empty list for a request that changes nothing', async () => {
		requests.present([request()]);
		pullRequestFiles.mockResolvedValue([]);
		await requests.loadFiles();
		const view = render(RequestDetail, {});
		flushSync();

		expect(view.text()).toContain('changes no files');
		view.destroy();
	});

	it('opens the review form, and will not submit a comment with nothing in it', async () => {
		const view = await panel();

		click(view.get('.actions button'));
		flushSync();

		// Comment is the default verdict, and a comment with no comment is what
		// the host rejects with a 422 that does not name the field.
		const submit = view
			.all('button')
			.find((button) => button.textContent?.includes('Submit review')) as HTMLButtonElement;
		expect(submit.disabled).toBe(true);
		expect(view.text()).toContain('needs something written with it');
		view.destroy();
	});

	it('lets approve go without a comment, because the host takes it', async () => {
		const view = await panel();
		click(view.get('.actions button'));
		flushSync();

		const approve = view
			.all('.verdict')
			.find((button) => button.textContent?.includes('Approve')) as HTMLElement;
		click(approve);
		flushSync();

		const submit = view
			.all('button')
			.find((button) => button.textContent?.includes('Submit review')) as HTMLButtonElement;
		expect(submit.disabled).toBe(false);
		view.destroy();
	});

	/** Open the review form and press Submit. */
	async function submitFrom(view: ReturnType<typeof render>) {
		click(view.get('.actions button'));
		flushSync();

		const box = view.get('.comment') as HTMLTextAreaElement;
		box.value = 'one note';
		box.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();

		const submit = view
			.all('button')
			.find((button) => button.textContent?.includes('Submit review')) as HTMLElement;
		click(submit);
		await settle();
		flushSync();
	}

	it('asks before it posts, naming the verdict rather than asking if you are sure', async () => {
		const view = await panel();

		await submitFrom(view);

		expect(confirmDialog).toHaveBeenCalledTimes(1);
		const asked = confirmDialog.mock.calls[0][0];
		expect(asked.title).toContain('Comment');
		expect(asked.title).toContain('#412');
		expect(asked.title).not.toMatch(/are you sure/i);
		// It says who sees it and that Spagitty cannot take it back.
		expect(asked.body).toContain('cannot take it back');
		view.destroy();
	});

	it('sends nothing when the confirmation is declined', async () => {
		confirmDialog.mockResolvedValue(false);
		const view = await panel();

		await submitFrom(view);

		expect(submitReview).not.toHaveBeenCalled();
		view.destroy();
	});

	it('posts the verdict and the comment once confirmed', async () => {
		const view = await panel();

		await submitFrom(view);

		expect(submitReview).toHaveBeenCalledWith(412, 'comment', 'one note');
		view.destroy();
	});

	it('still says plainly that merging is not built', async () => {
		const view = await panel();

		const merge = view
			.all('button')
			.find((button) => button.textContent?.includes('Merge')) as HTMLButtonElement;
		expect(merge.disabled).toBe(true);
		expect(merge.title).not.toBe('');
		view.destroy();
	});
});
