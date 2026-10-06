// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Review screen's state (FEAT-087): the inbox, the saved records behind
 * its progress, and which pull request is open in the review room.
 *
 * The open repository's pull requests are the `requests` store's — the same
 * list the Pull requests screen reads, read once — and this adds what only
 * the Review screen needs: the list across repositories, and per pull
 * request what the reviewer has done.
 */

import * as api from '../api';
import type { ReviewKey } from '../api';
import { repo } from '../repo.svelte';
import { requests } from '../requests/store.svelte';
import { notice } from '../ui/notice.svelte';
import type { ForgeKind, PullRequest, ReviewSummary } from '../types';
import { groupInbox, inboxOrder, type InboxGroup } from './inbox';
import { keyFor, keyString, normalise, type ReviewRecord } from './record';

export type InboxScope = 'repo' | 'all';

let scope = $state<InboxScope>('repo');
let involved = $state<PullRequest[]>([]);
let involvedLoading = $state(false);
let involvedError = $state<string | null>(null);
let involvedLoaded = $state(false);
/** The host the "All my repos" search ran against, and what kind it is. */
let searchHost = $state<string | null>(null);
let searchKind = $state<ForgeKind | null>(null);
/**
 * What GitLab's list does not carry — checks and thread counts — once asked
 * for (FEAT-088), by repository and number.
 */
let summaries = $state<Record<string, ReviewSummary>>({});
/** Rows already asked about, so a refreshed list does not ask twice. */
const asked = new Set<string>();
let selectedId = $state<string | null>(null);
let records = $state<Record<string, ReviewRecord>>({});
let room = $state<{ pr: PullRequest; key: ReviewKey } | null>(null);
/** A row from another repository that has no clone Spagitty knows. */
let notHere = $state<string | null>(null);
/** A worktree being made, for which pull request (FEAT-089). */
let checkingOut = $state<number | null>(null);

/** The repository generation the list was last read for. */
let primedFor = -1;
let involvedSeq = 0;

const LOCAL_PREFIX = 'spagitty.review.';

/** Outside the desktop shell — a plain browser, a test — records live here. */
function localRead(key: ReviewKey): unknown {
	try {
		const raw = localStorage.getItem(LOCAL_PREFIX + keyString(key));
		return raw ? JSON.parse(raw) : null;
	} catch {
		return null;
	}
}

function localWrite(key: ReviewKey, record: ReviewRecord | null): void {
	try {
		if (record === null) localStorage.removeItem(LOCAL_PREFIX + keyString(key));
		else localStorage.setItem(LOCAL_PREFIX + keyString(key), JSON.stringify(record));
	} catch {
		// Storage unavailable; the record lasts as long as the window.
	}
}

function summaryKey(pr: PullRequest): string {
	return `${pr.repository ?? ''}#${pr.number}`;
}

/** A row with what was learnt after the list laid over it. */
function merged(pr: PullRequest): PullRequest {
	const summary = summaries[summaryKey(pr)];
	if (!summary) return pr;
	return {
		...pr,
		checks: summary.checks ?? pr.checks,
		openThreads: summary.openThreads,
		resolvedThreads: summary.resolvedThreads,
		repliesToYou: summary.repliesToYou
	};
}

export const review = {
	get scope(): InboxScope {
		return scope;
	},

	setScope(next: InboxScope): void {
		scope = next;
		selectedId = null;
		notHere = null;
		if (next === 'all' && !involvedLoaded) void this.loadInvolved();
	},

	/** Who the connected account is. */
	get me(): string | null {
		return requests.currentUser;
	},

	/** The rows of the current scope, before grouping. */
	get list(): PullRequest[] {
		return (scope === 'repo' ? requests.all : involved).map(merged);
	},

	/**
	 * Ask for the checks and threads GitLab's list leaves out (FEAT-088), for
	 * the rows not asked about yet. Nothing is asked of a host whose list
	 * already says.
	 */
	async loadSummaries(): Promise<void> {
		if (!api.inTauri()) return;
		const kind = scope === 'repo' ? (requests.repo?.kind ?? null) : searchKind;
		if (kind !== 'gitLab') return;
		const rows = (scope === 'repo' ? requests.all : involved).filter(
			(pr) => !asked.has(summaryKey(pr))
		);
		if (rows.length === 0) return;
		for (const pr of rows) asked.add(summaryKey(pr));
		try {
			const found = await api.reviewSummaries(
				rows.map((pr) => ({ repository: pr.repository, number: pr.number }))
			);
			const next = { ...summaries };
			for (const [repository, summary] of found) {
				next[`${repository ?? ''}#${summary.number}`] = summary;
			}
			summaries = next;
		} catch {
			// The inbox shows what the list said; asking again is a Refresh.
			for (const pr of rows) asked.delete(summaryKey(pr));
		}
	},

	get groups(): InboxGroup[] {
		return groupInbox(this.list, this.me, scope);
	},

	get loading(): boolean {
		return scope === 'repo' ? requests.loading : involvedLoading;
	},

	get error(): string | null {
		return scope === 'repo' ? requests.error : involvedError;
	},

	/** The pull request in the preview card: the one chosen, or the first. */
	get selected(): PullRequest | null {
		const order = inboxOrder(this.groups);
		return order.find((pr) => pr.id === selectedId) ?? order[0] ?? null;
	},

	select(id: string): void {
		selectedId = id;
		notHere = null;
	},

	/** A row from another repository whose clone could not be found. */
	get notHere(): string | null {
		return notHere;
	},

	/**
	 * True when a pull request in this repository is waiting on your review:
	 * the rail's dot. From the last read, which happens when a repository
	 * opens and when either pull request screen refreshes.
	 */
	get waiting(): boolean {
		return requests.all.some((pr) => pr.reviewRequested);
	},

	/**
	 * Where a pull request's record is kept.
	 *
	 * A row of this repository is on its host. GitLab names the project on
	 * every row, its own list's too (BUG-044), so naming one does not make a
	 * row another repository's.
	 */
	keyOf(pr: PullRequest): ReviewKey | null {
		return keyFor(requests.repo, pr, this.isHere(pr) ? (requests.repo?.host ?? null) : searchHost);
	},

	/** The saved record for a pull request, once it has been read. */
	recordOf(pr: PullRequest): ReviewRecord | null {
		const key = this.keyOf(pr);
		return key ? (records[keyString(key)] ?? null) : null;
	},

	/** The saved record kept at `key`, once it has been read. */
	recordAt(key: ReviewKey): ReviewRecord | null {
		return records[keyString(key)] ?? null;
	},

	/** Read the saved records for every row not read yet. */
	async loadRecords(list: PullRequest[]): Promise<void> {
		await Promise.all(
			list.map(async (pr) => {
				const key = this.keyOf(pr);
				if (!key || records[keyString(key)]) return;
				let raw: unknown = null;
				try {
					raw = api.inTauri() ? await api.reviewState(key) : localRead(key);
				} catch {
					raw = null;
				}
				records[keyString(key)] = normalise(raw);
			})
		);
	},

	/** The record for `key`, read if it has not been. */
	async recordFor(key: ReviewKey): Promise<ReviewRecord> {
		const id = keyString(key);
		if (!records[id]) {
			let raw: unknown = null;
			try {
				raw = api.inTauri() ? await api.reviewState(key) : localRead(key);
			} catch {
				raw = null;
			}
			records[id] = normalise(raw);
		}
		return records[id];
	},

	/** Change a record and keep it. */
	async saveRecord(key: ReviewKey, change: (record: ReviewRecord) => void): Promise<ReviewRecord> {
		const record = $state.snapshot(await this.recordFor(key)) as ReviewRecord;
		change(record);
		records[keyString(key)] = record;
		if (api.inTauri()) await api.setReviewState(key, record);
		else localWrite(key, record);
		return record;
	},

	/**
	 * Read this repository's pull requests once per repository.
	 *
	 * Called when a repository opens, so the rail's dot is right before the
	 * screen is visited, and when the screen mounts.
	 */
	prime(generation: number): void {
		if (generation === primedFor) return;
		primedFor = generation;
		// A pull request open in the room belongs to the repository that was
		// open; another one has its own.
		room = null;
		selectedId = null;
		summaries = {};
		asked.clear();
		involvedLoaded = false;
		involved = [];
		if (repo.info !== null) void requests.load();
	},

	async loadInvolved(): Promise<void> {
		if (!api.inTauri()) return;
		const mine = ++involvedSeq;
		involvedLoading = true;
		involvedError = null;
		try {
			let host = requests.repo?.host ?? null;
			let kind = requests.repo?.kind ?? null;
			if (!host) {
				const accounts = await api.forgeAccounts();
				host = accounts[0]?.host ?? null;
				kind = accounts[0]?.kind ?? null;
			}
			const found = await api.involvedPullRequests();
			if (mine !== involvedSeq) return;
			searchHost = host;
			searchKind = kind;
			involved = found;
			involvedLoaded = true;
		} catch (e) {
			if (mine === involvedSeq) {
				involvedError = String(e);
				involved = [];
			}
		} finally {
			if (mine === involvedSeq) involvedLoading = false;
		}
	},

	async refresh(): Promise<void> {
		records = {};
		summaries = {};
		asked.clear();
		if (scope === 'all') await this.loadInvolved();
		else await requests.load();
	},

	/** The pull request open in the review room, or null for the inbox. */
	get room(): { pr: PullRequest; key: ReviewKey } | null {
		return room;
	},

	/**
	 * Open a pull request in the review room.
	 *
	 * A row from another repository opens that repository first, when
	 * Spagitty knows a clone of it; the room reads files from disk, so a pull
	 * request with no clone here cannot be reviewed here.
	 */
	async open(pr: PullRequest): Promise<boolean> {
		notHere = null;
		if (pr.repository && !this.isHere(pr)) {
			const host = searchHost ?? requests.repo?.host ?? null;
			const path = host && api.inTauri() ? await api.localCloneOf(host, pr.repository) : null;
			if (!path) {
				notHere = pr.repository;
				return false;
			}
			if (!(await repo.open(path))) return false;
			primedFor = repo.generation;
			await requests.load();
			const local = requests.all.find((candidate) => candidate.number === pr.number);
			if (!local) {
				notHere = pr.repository;
				return false;
			}
			scope = 'repo';
			pr = local;
		}

		const key = this.keyOf(pr);
		if (!key) return false;
		await this.recordFor(key);
		room = { pr, key };
		return true;
	},

	/** True when a row belongs to the open repository. */
	isHere(pr: PullRequest): boolean {
		const here = requests.repo;
		if (!pr.repository) return true;
		return !!here && `${here.owner}/${here.name}`.toLowerCase() === pr.repository.toLowerCase();
	},

	close(): void {
		room = null;
	},

	/** The pull request being checked out, or null. */
	get checkingOut(): number | null {
		return checkingOut;
	},

	/**
	 * Check a pull request's branch out here, to build and run it (FEAT-095).
	 * The head is fetched first when it is not here yet. A branch of yours is
	 * never moved: when the pull request's name is taken here, it is `pr-N`.
	 */
	async checkOut(pr: PullRequest): Promise<string | null> {
		if (!api.inTauri() || checkingOut !== null) return null;
		checkingOut = pr.number;
		try {
			const fetched = await api.reviewCheckout(pr.number, pr.targetBranch, pr.headSha);
			const done = await api.reviewCheckOut(pr.number, fetched.head, pr.sourceBranch, pr.targetBranch);
			const detail = done.renamed
				? `${pr.sourceBranch} here is another branch`
				: done.upstream
					? `following ${done.upstream}`
					: null;
			notice.ok(`On ${done.branch}`, detail);
			return done.branch;
		} catch (e) {
			notice.failed('The branch could not be checked out', e);
			return null;
		} finally {
			checkingOut = null;
			await repo.refresh();
		}
	},

	clear(): void {
		involvedSeq += 1;
		scope = 'repo';
		involved = [];
		involvedLoading = false;
		involvedError = null;
		involvedLoaded = false;
		searchHost = null;
		searchKind = null;
		summaries = {};
		asked.clear();
		selectedId = null;
		records = {};
		room = null;
		notHere = null;
		checkingOut = null;
		primedFor = -1;
	}
};
