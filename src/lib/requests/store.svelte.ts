// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Pull requests, read from whichever service hosts the open repository.
 *
 * FEAT-010 built this store's shape with nothing behind it; FEAT-017 filled it
 * in without changing the shape, which is what the earlier item was for.
 *
 * The read is a single call that either succeeds completely or fails with a
 * reason — offline, rate limited, no account, or a repository the host will not
 * show. Those are four different sentences and the store keeps them apart,
 * because "could not load" is a useless thing to tell somebody who is about to
 * decide whether to wait or to go and fix something.
 */

import * as api from '../api';
import type {
	FileDiff,
	ForgeRepo,
	PullRequest,
	PullRequestComment,
	PullRequestCommit
} from '../types';

export type WorkspaceViewMode = 'list' | 'workspace';
let list = $state<PullRequest[]>([]);
let connected = $state(false);
let error = $state<string | null>(null);
let openId = $state<string | null>(null);
let repo = $state<ForgeRepo | null>(null);
let loading = $state(false);
let viewMode = $state<WorkspaceViewMode>('list');
let currentUser = $state<string | null>(null);
let isCreateModalOpen = $state(false);
let creating = $state(false);
let createError = $state<string | null>(null);

/** Guards against a slow read landing after a newer one. */
let seq = 0;

/**
 * The files of the pull request currently open, and nothing else's.
 */
let files = $state<FileDiff[]>([]);
let filesFor = $state<number | null>(null);
let filesLoading = $state(false);
let filesError = $state<string | null>(null);
let openPath = $state<string | null>(null);

/** Commits belonging to open PR. */
let commits = $state<PullRequestCommit[]>([]);
let commitsFor = $state<number | null>(null);
let commitsLoading = $state(false);
let commitsError = $state<string | null>(null);
let selectedCommitSha = $state<string | null>(null);
let commitFilesCache = $state<Record<string, FileDiff[]>>({});
let commitFilesLoading = $state(false);

/** The comments on the open pull request, read here and answered in Review (FEAT-116). */
let comments = $state<PullRequestComment[]>([]);
let commentsFor = $state<number | null>(null);
let commentsLoading = $state(false);
let commentsError = $state<string | null>(null);

/** Guards a slow file read landing after the reader has moved on. */
let fileSeq = 0;

/** In flight state for close/draft (FEAT-071). Merging is Merger's (FEAT-117). */
let closing = $state(false);
let closeError = $state<string | null>(null);
let togglingDraft = $state(false);
let draftError = $state<string | null>(null);

/** How long to wait before asking again after a refusal. Tests set it to 0. */
export const retryRefusal = { afterMs: 700 };

/** The host's refusal of the sign-in, or the keychain not giving it up. */
function refusal(error: unknown): boolean {
	return /refused|keychain/i.test(String(error));
}

/**
 * Ask, and ask once more after a refusal (BUG-067): a sign-in refused on the
 * very first read and accepted on the next, with nothing changed, is not a bad
 * sign-in, and the reader is not sent to fix their account for it. `still`
 * says whether the answer is wanted after the wait; when it is not, nothing is
 * asked again and the answer is undefined.
 */
export async function askAgainAfterRefusal<T>(ask: () => Promise<T>, still: () => boolean): Promise<T | undefined> {
	try {
		return await ask();
	} catch (first) {
		if (!refusal(first)) throw first;
		await new Promise((resolve) => setTimeout(resolve, retryRefusal.afterMs));
		if (!still()) return undefined;
		return ask();
	}
}

export const requests = {
	get all(): PullRequest[] {
		return list;
	},
	/** True when the last read reached a host and came back with a list. */
	get connected(): boolean {
		return connected;
	},
	/** Which repository on a host this is, or null when it is not on one. */
	get repo(): ForgeRepo | null {
		return repo;
	},

	/**
	 * Whether this host has draft pull requests at all (FEAT-071).
	 *
	 * One host carries a draft flag, another a `Draft:` title marker. A third
	 * has neither, so the workspace offers no control rather than writing a
	 * state that host does not know it is in.
	 */
	get canDraft(): boolean {
		return repo !== null && repo.kind !== 'bitbucket';
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},
	get openId(): string | null {
		return openId;
	},
	get viewMode(): WorkspaceViewMode {
		return viewMode;
	},
	get currentUser(): string | null {
		return currentUser;
	},
	get isCreateModalOpen(): boolean {
		return isCreateModalOpen;
	},
	get creating(): boolean {
		return creating;
	},
	get createError(): string | null {
		return createError;
	},

	openCreateModal(): void {
		isCreateModalOpen = true;
		createError = null;
	},
	closeCreateModal(): void {
		isCreateModalOpen = false;
		createError = null;
	},

	async create(
		title: string,
		body: string,
		head: string,
		base: string,
		draft = false
	): Promise<PullRequest> {
		creating = true;
		createError = null;
		try {
			const newPr = await api.createPullRequest(title, body, head, base, draft);
			isCreateModalOpen = false;
			await this.load();
			this.select(newPr.id);
			return newPr;
		} catch (err) {
			createError = err instanceof Error ? err.message : String(err);
			throw err;
		} finally {
			creating = false;
		}
	},

	/** What is waiting on the person using Spagitty. The screen leads with these. */
	get needingYou(): PullRequest[] {
		return list.filter((request) => request.needsYou);
	},

	/** Everything else: open, but somebody else's move. */
	get waitingOnOthers(): PullRequest[] {
		return list.filter((request) => !request.needsYou);
	},

	/** The open request, or null. */
	get open(): PullRequest | null {
		return list.find((request) => request.id === openId) ?? null;
	},

	/** The files of the open pull request. Empty until they are read. */
	get files(): FileDiff[] {
		return files;
	},
	get filesLoading(): boolean {
		return filesLoading;
	},
	get filesError(): string | null {
		return filesError;
	},

	/** Commits list. */
	get commits(): PullRequestCommit[] {
		return commits;
	},
	get commitsLoading(): boolean {
		return commitsLoading;
	},
	get commitsError(): string | null {
		return commitsError;
	},
	get selectedCommitSha(): string | null {
		return selectedCommitSha;
	},
	get commitFilesCache(): Record<string, FileDiff[]> {
		return commitFilesCache;
	},
	get commitFilesLoading(): boolean {
		return commitFilesLoading;
	},

	/** Comments & drafts. */
	get comments(): PullRequestComment[] {
		return comments;
	},
	get commentsLoading(): boolean {
		return commentsLoading;
	},
	get commentsError(): string | null {
		return commentsError;
	},

	/** Currently displayed file diff list based on active scope (all files vs commit). */
	get currentFiles(): FileDiff[] {
		if (selectedCommitSha && commitFilesCache[selectedCommitSha]) {
			return commitFilesCache[selectedCommitSha];
		}
		return files;
	},

	/** Which file is open in the diff pane, or null. */
	get openPath(): string | null {
		return openPath;
	},
	/** The open file's diff, or null when none is selected. */
	get openFile(): FileDiff | null {
		return this.currentFiles.find((file) => file.path === openPath) ?? null;
	},
	get closing(): boolean {
		return closing;
	},
	get closeError(): string | null {
		return closeError;
	},
	get togglingDraft(): boolean {
		return togglingDraft;
	},
	get draftError(): string | null {
		return draftError;
	},

	select(id: string | null): void {
		if (id === openId) return;
		openId = id;
		this.clearFiles();
	},

	/** Open dedicated PR workspace. */
	openWorkspace(id?: string): void {
		if (id) {
			this.select(id);
		}
		viewMode = 'workspace';
		this.loadWorkspaceData();
	},

	/** Return to requests list view. */
	closeWorkspace(): void {
		viewMode = 'list';
	},

	/** Open one file in the diff pane. */
	selectPath(path: string | null): void {
		openPath = path;
	},

	/** Select a specific commit or null for all PR files. */
	async selectCommit(sha: string | null): Promise<void> {
		selectedCommitSha = sha;
		if (sha === null) {
			if (files.length > 0 && (!openPath || !files.some((f) => f.path === openPath))) {
				openPath = files[0].path;
			}
			return;
		}

		if (commitFilesCache[sha]) {
			const cFiles = commitFilesCache[sha];
			if (cFiles.length > 0 && (!openPath || !cFiles.some((f) => f.path === openPath))) {
				openPath = cFiles[0].path;
			}
			return;
		}

		if (!api.inTauri()) return;
		commitFilesLoading = true;
		try {
			const fetched = await api.commitFiles(sha);
			commitFilesCache[sha] = fetched;
			if (selectedCommitSha === sha) {
				if (fetched.length > 0 && (!openPath || !fetched.some((f) => f.path === openPath))) {
					openPath = fetched[0].path;
				}
			}
		} catch {
			commitFilesCache[sha] = [];
		} finally {
			commitFilesLoading = false;
		}
	},

	/** Put a file list on the screen. */
	presentFiles(next: FileDiff[], forNumber: number): void {
		files = next;
		filesFor = forNumber;
		filesError = null;
		if (openPath === null || !next.some((file) => file.path === openPath)) {
			openPath = next[0]?.path ?? null;
		}
	},

	/** Put commits on the screen (testing / loading). */
	presentCommits(next: PullRequestCommit[], forNumber: number): void {
		commits = next;
		commitsFor = forNumber;
		commitsError = null;
	},

	/** Put comments on the screen. */
	presentComments(next: PullRequestComment[], forNumber: number): void {
		comments = next;
		commentsFor = forNumber;
		commentsError = null;
	},

	/** Read files, commits, and comments for open PR. */
	async loadWorkspaceData(): Promise<void> {
		await Promise.all([this.loadFiles(), this.loadCommits(), this.loadComments()]);
	},

	/** Read commits for the open PR. */
	async loadCommits(force = false): Promise<void> {
		if (!api.inTauri()) return;
		if (typeof api.pullRequestCommits !== 'function') return;
		const request = this.open;
		if (request === null) return;
		if (!force && commitsFor === request.number && commitsError === null) return;

		commitsLoading = true;
		commitsError = null;
		try {
			const found = await api.pullRequestCommits(request.number);
			this.presentCommits(found, request.number);
		} catch (e) {
			commitsError = String(e);
			commits = [];
			commitsFor = null;
		} finally {
			commitsLoading = false;
		}
	},

	/** Read review comments for the open PR. */
	async loadComments(force = false): Promise<void> {
		if (!api.inTauri()) return;
		if (typeof api.pullRequestComments !== 'function') return;
		const request = this.open;
		if (request === null) return;
		if (!force && commentsFor === request.number && commentsError === null) return;

		commentsLoading = true;
		commentsError = null;
		try {
			const found = await api.pullRequestComments(request.number);
			this.presentComments(found, request.number);
		} catch (e) {
			commentsError = String(e);
			comments = [];
			commentsFor = null;
		} finally {
			commentsLoading = false;
		}
	},

	/** Read the open pull request's files. */
	async loadFiles(force = false): Promise<void> {
		if (!api.inTauri()) return;

		const request = this.open;
		if (request === null) return;
		if (!force && filesFor === request.number && filesError === null) return;

		filesLoading = true;
		filesError = null;
		const mine = ++fileSeq;
		try {
			const found = await api.pullRequestFiles(request.number);
			if (mine !== fileSeq) return;
			this.presentFiles(found, request.number);
		} catch (e) {
			if (mine === fileSeq) {
				filesError = String(e);
				files = [];
				filesFor = null;
				openPath = null;
			}
		} finally {
			if (mine === fileSeq) filesLoading = false;
		}
	},

	clearFiles(): void {
		fileSeq += 1;
		files = [];
		filesFor = null;
		filesError = null;
		filesLoading = false;
		commits = [];
		commitsFor = null;
		commitsError = null;
		commitsLoading = false;
		comments = [];
		commentsFor = null;
		commentsError = null;
		commentsLoading = false;
		selectedCommitSha = null;
		commitFilesCache = {};
		openPath = null;
	},

	/** Close / reject the open pull request without merging (FEAT-071). */
	async close(): Promise<boolean> {
		if (!api.inTauri()) return false;
		const request = this.open;
		if (request === null) return false;

		closing = true;
		closeError = null;
		try {
			await api.closePullRequest(request.number);
			await this.load();
			return true;
		} catch (e) {
			closeError = String(e);
			return false;
		} finally {
			closing = false;
		}
	},

	/** Toggle draft / ready-for-review status (FEAT-071). */
	async toggleDraft(): Promise<boolean> {
		if (!api.inTauri()) return false;
		const request = this.open;
		if (request === null) return false;

		if (!this.canDraft) return false;

		togglingDraft = true;
		draftError = null;
		try {
			await api.setPrDraft(request.number, request.id, request.title, !request.draft);
			await this.load();
			return true;
		} catch (e) {
			draftError = String(e);
			return false;
		} finally {
			togglingDraft = false;
		}
	},

	present(next: PullRequest[], from: { connected: boolean } = { connected: true }): void {
		list = next;
		connected = from.connected;
		error = null;
		const was = openId;
		if (openId === null || !next.some((request) => request.id === openId)) {
			// The one on screen is gone — merged, closed — so the workspace goes
			// back to the list rather than turning into another (BUG-062).
			if (was !== null) viewMode = 'list';
			openId = next[0]?.id ?? null;
		}
		if (openId === was) return;
		this.clearFiles();
		if (viewMode === 'workspace') void this.loadWorkspaceData();
	},

	fail(reason: string): void {
		connected = false;
		error = reason;
		list = [];
		openId = null;
		this.clearFiles();
	},

	async load(): Promise<void> {
		if (!api.inTauri()) return;

		loading = true;
		const mine = ++seq;
		try {
			const where = await api.forgeRepo();
			if (mine !== seq) return;
			repo = where;

			if (where === null) {
				list = [];
				connected = false;
				error = null;
				openId = null;
				currentUser = null;
				return;
			}

			if (typeof api.forgeAccounts === 'function') {
				try {
					// The account for this repository's host, not the first one
					// connected: with two accounts "you" would be someone else on
					// the other host (BUG-067).
					const accounts = await api.forgeAccounts();
					currentUser =
						accounts?.find((account) => account.host === where.host)?.user ?? accounts?.[0]?.user ?? null;
				} catch {
					// accounts not loaded
				}
			}

			const found = await askAgainAfterRefusal(
				() => api.pullRequests(),
				() => mine === seq
			);
			if (found === undefined || mine !== seq) return;
			this.present(found);
		} catch (e) {
			if (mine === seq) this.fail(String(e));
		} finally {
			if (mine === seq) loading = false;
		}
	},

	clear(): void {
		seq += 1;
		list = [];
		connected = false;
		error = null;
		openId = null;
		repo = null;
		currentUser = null;
		loading = false;
		viewMode = 'list';
		this.clearFiles();
	}
};

/** What each review state is called on screen. Host-agnostic, deliberately. */
export const REVIEW_LABELS: Record<PullRequest['review'], string> = {
	awaitingReview: 'awaiting review',
	changesRequested: 'changes requested',
	approved: 'approved',
	noReviewers: 'no reviewers'
};

/** What each check state is called. `null` means the host runs no checks. */
export const CHECK_LABELS: Record<NonNullable<PullRequest['checks']>, string> = {
	passing: 'checks passing',
	failing: 'checks failing',
	running: 'checks running'
};
