// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merger's state (FEAT-100): the two branches, where the result lands, how,
 * and the dry run's forecast for the pair.
 *
 * The forecast is per pair, not per direction: changing where it lands or how
 * re-derives the plan from the same forecast (`plan.ts`) and asks git nothing.
 * Picking another branch, or the refs moving under the screen, asks again.
 * Superseded answers are dropped rather than painted over a newer one.
 *
 * Landing (FEAT-101) goes through the commit dialog, from the plan when there
 * is nothing to resolve: it names what will be written and takes the message,
 * and only its button writes.
 *
 * Once it has landed, the branch that received it is checked out, and a push
 * of it is offered (FEAT-123). Landing itself still checks nothing out: the
 * merge is made from the dry run, and the checkout is a separate step that
 * git may refuse without the merge being any less made.
 */

import * as api from '../api';
import { activity } from '../activity/store.svelte';
import * as farmApi from '../farm/api';
import { network } from '../network/store.svelte';
import { repo } from '../repo.svelte';
import { requests } from '../requests/store.svelte';
import type {
	BranchRow,
	MergeMethod,
	MergerForecast,
	MergerLandAsk,
	MergerLanded,
	MergerResolution,
	PullRequest,
	Tag
} from '../types';
import {
	defaultNewName,
	effectiveStrategy,
	pushPlan,
	roles,
	strategies,
	type Into,
	type PushPlan,
	type Roles,
	type Strategy,
	type StrategyChoice
} from './plan';

/** One name a branch picker offers. */
export interface Pickable {
	name: string;
	group: 'local' | 'remote' | 'tag';
	time: number;
}

let a = $state<string | null>(null);
let b = $state<string | null>(null);
let into = $state<Into>('a');
let asked = $state<Strategy>('merge');
/** The new branch's name as typed; null until it is, so it follows A and B. */
let typedName = $state<string | null>(null);

let forecast = $state<MergerForecast | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let pickables = $state<Pickable[]>([]);
let current = $state<string | null>(null);
let seq = 0;
/** Each picker name's commit, as the last read of the refs found it. */
let tips = new Map<string, string>();
/**
 * What the forecast in hand — or on its way — was asked for: the repository,
 * the pair and both tips. A refresh that changes none of them has nothing new
 * to ask, and asking again would throw away an answer still on its way
 * (BUG-060).
 */
let requested: string | null = null;

/** The question a forecast answers, as one string. */
function question(path: string | null, x: string | null, y: string | null): string {
	return [path ?? '', x ?? '', y ?? '', tips.get(x ?? '') ?? '', tips.get(y ?? '') ?? ''].join('\u0000');
}

/**
 * What Merger was opened for, when it was opened for something (FEAT-117).
 *
 * Every merge is made here, so the other screens that used to make one open
 * Merger instead and say what they came for. A pull request is merged through
 * its host, with the host's methods, and when it conflicts the conflicts are
 * resolved here first; a Farm task is merged like any two branches, and the
 * Farm is told when it is done.
 */
export type Origin =
	| {
			kind: 'pullRequest';
			pr: PullRequest;
			/**
			 * Set while its conflicts are being resolved here: the branch, what it
			 * takes in, and the remote the pull request's own branch is on. The
			 * remote is null when that branch is not known to be one of this
			 * repository's remote's: a fork's is not, and a push from here would
			 * not reach it (BUG-076).
			 */
			local: { branch: string; target: string; remote: string | null } | null;
	  }
	| { kind: 'farmTask'; id: string; branch: string; target: string; title: string };

let origin = $state<Origin | null>(null);
/** The host merge of a pull request. */
let prMethod = $state<MergeMethod>('merge');
let prTitle = $state('');
let prMessage = $state('');
let prMerging = $state(false);
let prError = $state<string | null>(null);
let prMerged = $state(false);
/** Fetching the pull request's branch to resolve it here. */
let preparing = $state(false);
let prepareError = $state<string | null>(null);
/** Pushing the resolved branch back to the pull request. */
let pushing = $state(false);
let pushed = $state(false);
let pushError = $state<string | null>(null);
/** The Farm could not be told its task was merged. */
let farmError = $state<string | null>(null);

/** Why a Farm task cannot be squashed or rebased in. */
export const FARM_KEEPS_COMMITS =
	'A Farm task is merged with its commits kept, so the Farm can see its work is in the branch';

/** Where the screen is: the plan, resolving, the commit dialog, or done. */
export type Phase = 'plan' | 'resolve' | 'commit' | 'done';
let phase = $state<Phase>('plan');
/** Where Back from the commit dialog goes. */
let returnTo = $state<'plan' | 'resolve'>('plan');
/** The message as typed; null until it is, so it follows the plan. */
let typedMessage = $state<string | null>(null);
let landing = $state(false);
let landError = $state<string | null>(null);
/** What landed, and what it came from, for the done state. */
let landed = $state<{ landed: MergerLanded; source: string; strategy: Strategy } | null>(null);

/**
 * What became of checking out the branch that received the merge (FEAT-123).
 *
 * - `switching`: git is doing it now.
 * - `done`: it is checked out here now.
 * - `already`: it was the branch checked out, and landing brought it forward.
 * - `elsewhere`: it is checked out in another worktree, which git allows only
 *   once. Landing updated it there; here stays where it was.
 * - `failed`: git refused, in `error`'s words. The merge is made all the same.
 */
export type Checkout =
	| { state: 'switching' }
	| { state: 'done' }
	| { state: 'already' }
	| { state: 'elsewhere'; path: string }
	| { state: 'failed'; error: string };

let checkout = $state<Checkout | null>(null);
/** Where a push of the branch that received it would go; null with no remote. */
let pushTo = $state<PushPlan | null>(null);
/** The remotes have been read, so a null `pushTo` means there are none. */
let pushKnown = $state(false);
/** They are being read: the push is about to be offered, or said to have nowhere to go. */
let pushReading = $state(false);
/**
 * Which landing the work after it belongs to. The reads and the checkout that
 * follow a merge are not waited for, so the dialog can be closed, and another
 * merge made, while they are still out. What comes back for an earlier landing
 * is dropped: it would offer a push of the branch before this one.
 */
let settlement = 0;

const KEY = 'spagitty.merger.pick';

interface Remembered {
	a: string | null;
	b: string | null;
	into: Into;
	strategy: Strategy;
}

/** Per viewer and per repository: a convenience, so it may be lost. */
function remembered(repo: string): Remembered | null {
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, Remembered>;
		return all[repo] ?? null;
	} catch {
		return null;
	}
}

function remember(repo: string | null) {
	if (!repo) return;
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, Remembered>;
		all[repo] = { a, b, into, strategy: asked };
		localStorage.setItem(KEY, JSON.stringify(all));
	} catch {
		// Storage is unavailable: the pick is only for this visit.
	}
}

let repoPath: string | null = null;

/** Two spellings of one directory: separators and a trailing one aside. */
function samePath(x: string | null, y: string | null): boolean {
	if (x === null || y === null) return false;
	const plain = (path: string) => path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase();
	return plain(x) === plain(y);
}

/** A pair asked for from elsewhere — the graph's drag — taken on the next prime. */
let preset: { a: string; b: string; into: Into } | null = null;

/** Local branches newest first, then remote-tracking branches, then tags. */
export function pickablesFrom(rows: BranchRow[], tags: Tag[]): Pickable[] {
	const local = rows
		.filter((row) => row.kind === 'branch')
		.map((row) => ({ name: row.name, group: 'local' as const, time: row.time }));
	const remote = rows
		.filter((row) => row.kind === 'remote' && !row.name.endsWith('/HEAD'))
		.map((row) => ({ name: row.name, group: 'remote' as const, time: row.time }));
	const tagged = tags.map((tag) => ({ name: tag.name, group: 'tag' as const, time: 0 }));
	const byTime = (x: Pickable, y: Pickable) => y.time - x.time;
	return [...local.sort(byTime), ...remote.sort(byTime), ...tagged];
}

/**
 * The pair to start from: what was picked last time if both still exist,
 * otherwise the checked-out branch and the most recently committed other
 * local branch.
 */
export function startingPair(
	options: Pickable[],
	checkedOut: string | null,
	last: Remembered | null
): { a: string | null; b: string | null } {
	const names = new Set(options.map((option) => option.name));
	if (last?.a && last.b && names.has(last.a) && names.has(last.b) && last.a !== last.b) {
		return { a: last.a, b: last.b };
	}
	const local = options.filter((option) => option.group === 'local');
	const first = checkedOut && names.has(checkedOut) ? checkedOut : (local[0]?.name ?? null);
	const second = local.find((option) => option.name !== first)?.name ?? null;
	return { a: first, b: second };
}

export const merger = {
	get a(): string | null {
		return a;
	},
	get b(): string | null {
		return b;
	},
	get into(): Into {
		return into;
	},
	/**
	 * The forecast for the pair picked now, or null while it is being worked
	 * out. The one in hand is kept until the next lands, and is not handed out
	 * for another pair: drawn under new names it would describe a merge
	 * nobody asked for, and Resolve or Merge now would act on it (BUG-061).
	 */
	get forecast(): MergerForecast | null {
		return forecast && forecast.a.name === a && forecast.b.name === b ? forecast : null;
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},
	get pickables(): Pickable[] {
		return pickables;
	},
	/** The branch checked out here, for the pickers to mark. */
	get current(): string | null {
		return current;
	},
	get newName(): string {
		return typedName ?? (a && b ? defaultNewName(a, b) : '');
	},
	get roles(): Roles | null {
		const plan = this.forecast;
		return plan ? roles(plan, into, this.newName) : null;
	},
	get choices(): StrategyChoice[] {
		const who = this.roles;
		const plan = this.forecast;
		const all = plan && who ? strategies(plan, who) : [];
		if (!this.forFarm) return all;
		// The Farm finishes a task when the task's own commit is in the branch
		// it was meant for. A squash or a rebase writes other commits, so the
		// Farm would never see the task as merged (BUG-077).
		return all.map((choice) =>
			choice.id === 'squash' || choice.id === 'rebase'
				? { ...choice, reason: choice.reason ?? FARM_KEEPS_COMMITS }
				: choice
		);
	},
	/** Opened for a Farm task: its branch goes into its target, with its commits kept. */
	get forFarm(): boolean {
		return origin?.kind === 'farmTask';
	},
	/** The strategy shown: the one asked for, unless it is not possible here. */
	get strategy(): Strategy {
		return effectiveStrategy(this.choices, asked);
	},
	/** Local branch names, for telling a new branch's name is taken. */
	get phase(): Phase {
		return phase;
	},
	get landing(): boolean {
		return landing;
	},
	/** Where the commit dialog was opened from. */
	get returnTo(): 'plan' | 'resolve' {
		return returnTo;
	},
	get landError(): string | null {
		return landError;
	},
	get landed() {
		return landed;
	},
	/** What became of checking out the branch that received the merge. */
	get checkout(): Checkout | null {
		return checkout;
	},
	/** Where *Push now* would send it, or null. */
	get pushTo(): PushPlan | null {
		return pushTo;
	},
	/** Read, and there is no remote to push to. */
	get nowhereToPush(): boolean {
		return pushKnown && pushTo === null;
	},
	/** Where a push would go is still being read. */
	get pushReading(): boolean {
		return pushReading;
	},

	// --- What Merger was opened for (FEAT-117) ---------------------------
	get origin(): Origin | null {
		return origin;
	},
	get prMethod(): MergeMethod {
		return prMethod;
	},
	get prTitle(): string {
		return prTitle;
	},
	get prMessage(): string {
		return prMessage;
	},
	get prMerging(): boolean {
		return prMerging;
	},
	get prError(): string | null {
		return prError;
	},
	get prMerged(): boolean {
		return prMerged;
	},
	get preparing(): boolean {
		return preparing;
	},
	get prepareError(): string | null {
		return prepareError;
	},
	get pushing(): boolean {
		return pushing;
	},
	get pushed(): boolean {
		return pushed;
	},
	get pushError(): string | null {
		return pushError;
	},
	get farmError(): string | null {
		return farmError;
	},
	setPrMethod(next: MergeMethod) {
		prMethod = next;
	},
	setPrTitle(text: string) {
		prTitle = text;
	},
	setPrMessage(text: string) {
		prMessage = text;
	},

	/** Merge this pull request here: through its host, once its conflicts are none. */
	presentPullRequest(pr: PullRequest) {
		origin = { kind: 'pullRequest', pr, local: null };
		prMethod = 'merge';
		prTitle = '';
		prMessage = '';
		prMerging = false;
		prError = null;
		prMerged = false;
		preparing = false;
		prepareError = null;
		pushing = false;
		pushed = false;
		pushError = null;
		farmError = null;
		preset = null;
		phase = 'plan';
		landed = null;
		typedMessage = null;
		typedName = null;
	},

	/** Merge this Farm task's branch into where it was meant to go. */
	presentFarmTask(task: { id: string; branch: string; target: string; title: string }) {
		this.present({ a: task.target, b: task.branch, into: 'a' });
		origin = { kind: 'farmTask', ...task };
		farmError = null;
	},

	/**
	 * Ask the host to merge the pull request, with the method chosen. The host
	 * refuses a pull request that conflicts, and its own sentence is shown.
	 */
	async mergePullRequest(): Promise<boolean> {
		const o = origin;
		if (!o || o.kind !== 'pullRequest' || prMerging) return false;
		prMerging = true;
		prError = null;
		try {
			await api.mergePullRequest(
				o.pr.number,
				prMethod,
				prMethod === 'rebase' ? undefined : prTitle.trim() || undefined,
				prMethod === 'rebase' ? undefined : prMessage.trim() || undefined
			);
			prMerged = true;
			void requests.load();
			return true;
		} catch (e) {
			prError = String(e);
			return false;
		} finally {
			prMerging = false;
		}
	},

	/**
	 * Resolve a conflicting pull request here: fetch its branch, check it out,
	 * and plan the merge of its target into it like any other two branches.
	 * Landing writes the resolution on the pull request's own branch; pushing
	 * it and merging the pull request come after.
	 */
	async resolvePullRequestLocally(): Promise<boolean> {
		const o = origin;
		if (!o || o.kind !== 'pullRequest' || preparing) return false;
		const pr = o.pr;
		preparing = true;
		prepareError = null;
		try {
			const fetched = await api.reviewCheckout(pr.number, pr.targetBranch, pr.headSha);
			const done = await api.reviewCheckOut(pr.number, fetched.head, pr.sourceBranch, pr.targetBranch);
			const [rows, tags] = await Promise.all([api.branches(), api.tags().catch(() => [] as Tag[])]);
			const fetchedTarget = pickablesFrom(rows, tags).find(
				(option) => option.group === 'remote' && option.name.endsWith('/' + pr.targetBranch)
			)?.name;
			const target = fetchedTarget ?? pr.targetBranch;
			// The target was fetched from the remote the pull request is on. Its
			// own branch is pushed there only when that remote is seen to have it.
			const from = fetchedTarget ? fetchedTarget.slice(0, -(pr.targetBranch.length + 1)) : null;
			const own =
				from !== null &&
				(done.upstream !== null ||
					rows.some((row) => row.kind === 'remote' && row.name === `${from}/${pr.sourceBranch}`));
			origin = { kind: 'pullRequest', pr, local: { branch: done.branch, target, remote: own ? from : null } };
			preset = { a: done.branch, b: target, into: 'a' };
			phase = 'plan';
			landed = null;
			typedMessage = null;
			typedName = null;
			pushed = false;
			pushError = null;
			await this.prime(repoPath, done.branch);
			return true;
		} catch (e) {
			prepareError = String(e);
			return false;
		} finally {
			preparing = false;
		}
	},

	/**
	 * Push the resolved branch to the pull request's own branch, and wait for
	 * git to say how it went (BUG-076).
	 *
	 * Through the network store, like every other push: it names the remote,
	 * which is what makes git take the refspec; it lets go of the worker when
	 * the push ends, so the next fetch or push can start; and *Pushed* is said
	 * only once git has said so.
	 */
	async pushPullRequestHead(): Promise<boolean> {
		const o = origin;
		if (!o || o.kind !== 'pullRequest' || !o.local || pushing) return false;
		const { branch, remote } = o.local;
		if (remote === null) {
			pushError = `Spagitty does not know where ${o.pr.sourceBranch} is: it has not been fetched from this repository's remote, or the pull request comes from a fork. Fetch and try again, or push ${branch} to it yourself.`;
			return false;
		}
		// A pull fetches too, and is not the network store's (FEAT-121).
		if (network.running || activity.blocked('network')) {
			pushError = 'A fetch or a push is running. Push again when it has finished.';
			return false;
		}
		pushing = true;
		pushError = null;
		try {
			const started = await network.push(remote, `${branch}:${o.pr.sourceBranch}`);
			if (!started || !(await network.whenDone())) {
				pushError = network.error ?? 'The push did not go through.';
				return false;
			}
			pushed = true;
			return true;
		} finally {
			pushing = false;
		}
	},

	/** Back to the pull request, to merge it through its host now it is resolved. */
	async backToPullRequest() {
		const o = origin;
		if (!o || o.kind !== 'pullRequest') return;
		origin = { kind: 'pullRequest', pr: o.pr, local: null };
		phase = 'plan';
		landed = null;
		await requests.load();
		const fresh = requests.all.find((request) => request.number === o.pr.number);
		if (fresh) origin = { kind: 'pullRequest', pr: fresh, local: null };
	},

	/** Tell the Farm its task's branch is merged (again, after a failure). */
	async finishFarmTask(): Promise<boolean> {
		const o = origin;
		if (!o || o.kind !== 'farmTask') return false;
		farmError = null;
		try {
			await farmApi.taskMerged(o.id);
			return true;
		} catch (e) {
			farmError = String(e);
			return false;
		}
	},

	/** Leave what Merger was opened for. */
	leaveOrigin() {
		origin = null;
	},
	/** `Merge branch 'feat' into main`, or `Squash feat into main`. */
	get defaultMessage(): string {
		const who = this.roles;
		if (!who) return '';
		return this.strategy === 'squash'
			? `Squash ${who.sourceName} into ${who.targetName}`
			: `Merge branch '${who.sourceName}' into ${who.targetName}`;
	},
	get message(): string {
		return typedMessage ?? this.defaultMessage;
	},
	setMessage(text: string) {
		typedMessage = text;
	},

	/** Open the commit dialog, from the plan or from resolving. */
	openCommit(from: 'plan' | 'resolve' = 'plan') {
		returnTo = from;
		landError = null;
		phase = 'commit';
	},
	/** Back out of the commit dialog, writing nothing. */
	back() {
		if (landing) return;
		phase = returnTo;
		landError = null;
	},
	setPhase(next: Phase) {
		phase = next;
	},

	/** What the backend needs to land or replay the merge planned now. */
	ask(resolutions: MergerResolution[] = []): MergerLandAsk | null {
		// Never a plan for another pair than the one named (BUG-061).
		const plan = this.forecast;
		if (!plan || !a || !b) return null;
		const strategy = this.strategy;
		return {
			a,
			b,
			aTip: plan.a.tip,
			bTip: plan.b.tip,
			target: into,
			newName: into === 'new' ? this.newName : undefined,
			strategy,
			message: strategy === 'rebase' || strategy === 'ff' ? undefined : this.message,
			resolutions
		};
	},

	/**
	 * Write it: commit the result and move the receiving branch. Refused by the
	 * backend if either branch moved since the forecast was read.
	 */
	async land(resolutions: MergerResolution[] = []): Promise<boolean> {
		const ask = this.ask(resolutions);
		const who = this.roles;
		if (!ask || !who || landing) return false;
		// A rebase lands from its own worktree once every commit is replayed
		// (FEAT-103), not in one call.
		const rebase = ask.strategy === 'rebase';
		// Where the receiving branch is checked out, as the forecast read it:
		// asked now, because the forecast is re-read once this has landed.
		const side = ask.target === 'new' ? null : (this.forecast?.[ask.target] ?? null);
		const elsewhere = side?.checkedOut ?? null;
		landing = true;
		landError = null;
		settlement += 1;
		checkout = null;
		pushTo = null;
		pushKnown = false;
		pushReading = false;
		try {
			// Named while it runs, and not started over a pull or a checkout
			// that is still writing (FEAT-121).
			const result = await activity.run('merge', 'Merging', () =>
				rebase ? api.mergerRebaseFinish(ask) : api.mergerLand(ask)
			);
			landed = { landed: result, source: who.sourceName, strategy: ask.strategy };
			phase = 'done';
			if (origin?.kind === 'farmTask') await this.finishFarmTask();
			// A merge somebody chose here. A pull request's branch is already
			// checked out and has its own push; a Farm task is the Farm's.
			if (origin === null) {
				pushing = false;
				pushed = false;
				pushError = null;
				void this.settle(result.target, elsewhere);
			}
			return true;
		} catch (e) {
			landError = String(e);
			return false;
		} finally {
			landing = false;
		}
	},

	/**
	 * After a merge has landed: check out the branch that received it, and
	 * work out where a push of it would go (FEAT-123).
	 *
	 * Not part of landing, and never its failure. The merge is made by then;
	 * a checkout git refuses — uncommitted work in the way is the usual
	 * reason — is said in git's words beside it, and can be asked for again.
	 */
	async settle(target: string, elsewhere: string | null = null): Promise<void> {
		// Where a push would go is read first and beside the checkout: it is
		// quick, and the dialog's buttons are then the same from the moment it
		// opens rather than changing under the pointer when a checkout ends.
		const mine = settlement;
		pushReading = true;
		const reading = Promise.all([api.branches(), api.remotes()])
			.then(([rows, known]) => {
				if (mine !== settlement) return;
				pushTo = pushPlan(
					target,
					rows,
					known.map((remote) => remote.name)
				);
				pushKnown = true;
			})
			.catch(() => {
				if (mine !== settlement) return;
				// Unread: no push is offered, and none is said to be impossible.
				pushTo = null;
				pushKnown = false;
			})
			.finally(() => {
				if (mine === settlement) pushReading = false;
			});

		if (current === target || samePath(elsewhere, repoPath)) {
			checkout = { state: 'already' };
		} else if (elsewhere !== null) {
			checkout = { state: 'elsewhere', path: elsewhere };
		} else {
			await this.checkOutLanded();
		}
		await reading;
	},

	/** Check out the branch that received the merge; again, after a refusal. */
	async checkOutLanded(): Promise<boolean> {
		const target = landed?.landed.target;
		if (!target || checkout?.state === 'switching') return false;
		const mine = settlement;
		checkout = { state: 'switching' };
		try {
			await activity.run('checkout', `Switching to ${target}`, () => api.checkout(target));
			// True whoever is still asking: git has moved HEAD.
			current = target;
			if (mine === settlement) checkout = { state: 'done' };
			return true;
		} catch (e) {
			if (mine === settlement) checkout = { state: 'failed', error: String(e) };
			return false;
		} finally {
			// HEAD moved, or did not: the chrome reads which from here.
			void repo.refresh();
		}
	},

	/**
	 * Push the branch that received the merge, and wait for git to say how it
	 * went, as a pull request's branch is pushed (BUG-076). The remote and the
	 * branch are named, so this does not depend on the checkout having worked.
	 */
	async pushLanded(): Promise<boolean> {
		const plan = pushTo;
		if (!landed || !plan || pushing || pushed) return false;
		const waiting = activity.blocked('network');
		if (waiting) {
			pushError = `${waiting}. Push again when it has finished.`;
			return false;
		}
		pushing = true;
		pushError = null;
		try {
			const started = await network.push(plan.remote, plan.refspec);
			if (!started || !(await network.whenDone())) {
				pushError = network.error ?? 'The push did not go through.';
				return false;
			}
			pushed = true;
			return true;
		} finally {
			pushing = false;
		}
	},

	/** Merge another: back to the plan, read afresh. */
	again() {
		phase = 'plan';
		landed = null;
		settlement += 1;
		checkout = null;
		pushTo = null;
		pushKnown = false;
		pushReading = false;
		pushed = false;
		pushError = null;
		typedMessage = null;
		landError = null;
		void this.load();
	},

	get localNames(): string[] {
		return pickables.filter((option) => option.group === 'local').map((option) => option.name);
	},

	/** Read the names the pickers offer, and settle on a pair. */
	async prime(path: string | null, checkedOut: string | null): Promise<void> {
		const changed = path !== repoPath;
		repoPath = path;
		current = checkedOut;
		if (changed) {
			forecast = null;
			error = null;
			typedName = null;
		}
		const [rows, tags] = await Promise.all([api.branches(), api.tags().catch(() => [] as Tag[])]);
		pickables = pickablesFrom(rows, tags);
		tips = new Map([...rows.map((row) => [row.name, row.id] as const), ...tags.map((tag) => [tag.name, tag.target] as const)]);
		const wanted = preset;
		preset = null;
		if (wanted) {
			a = wanted.a;
			b = wanted.b;
			into = wanted.into;
			remember(path);
		} else if (changed || a === null || b === null) {
			const last = path ? remembered(path) : null;
			const pair = startingPair(pickables, checkedOut, last);
			a = pair.a;
			b = pair.b;
			if (last) {
				into = last.into;
				asked = last.strategy;
			}
		}
		// Refreshed with neither branch moved: the forecast in hand, or the one
		// still on its way, is the answer. Asking again would drop it — and on
		// a repository that refreshes faster than a forecast takes, it would
		// never land (BUG-060). A failed ask is not an answer: the refresh
		// tries again, so a passing failure does not outlive its cause.
		if (!changed && question(path, a, b) === requested && (loading || forecast !== null)) return;
		await this.load();
	},

	/** Ask for the forecast of the pair picked now. */
	async load(): Promise<void> {
		const mine = ++seq;
		requested = question(repoPath, a, b);
		if (!a || !b) {
			forecast = null;
			loading = false;
			return;
		}
		if (a === b) {
			forecast = null;
			error = 'Pick two different branches';
			loading = false;
			return;
		}
		loading = true;
		try {
			const next = await api.mergerForecast(a, b);
			if (mine !== seq) return;
			forecast = next;
			error = null;
		} catch (e) {
			if (mine !== seq) return;
			forecast = null;
			error = String(e);
		} finally {
			if (mine === seq) loading = false;
		}
	},

	pickA(name: string) {
		a = name;
		remember(repoPath);
		void this.load();
	},
	pickB(name: string) {
		b = name;
		remember(repoPath);
		void this.load();
	},
	setInto(next: Into) {
		// A Farm task goes where it was meant to (BUG-077).
		if (this.forFarm && next !== 'a') return;
		into = next;
		remember(repoPath);
	},
	/** Turn the direction round: into A becomes into B and back. */
	swap() {
		if (this.forFarm) return;
		into = into === 'b' ? 'a' : 'b';
		remember(repoPath);
	},
	setStrategy(next: Strategy) {
		asked = next;
		remember(repoPath);
	},
	setNewName(name: string) {
		typedName = name;
	},

	/**
	 * Open on this pair next time the screen primes (TASK-056): what the
	 * graph's drag of one branch onto another asks for, B coming into A.
	 */
	present(next: { a: string; b: string; into: Into }) {
		origin = null;
		preset = next;
		phase = 'plan';
		landed = null;
		typedMessage = null;
		typedName = null;
	},

	/** For tests and previews: a pair and its forecast, as if read. */
	seed(next: { a: string; b: string; forecast: MergerForecast; pickables?: Pickable[] }) {
		a = next.a;
		b = next.b;
		forecast = next.forecast;
		pickables = next.pickables ?? [];
		error = null;
		loading = false;
	},

	/** For tests: back to nothing picked. */
	reset() {
		a = null;
		b = null;
		into = 'a';
		asked = 'merge';
		typedName = null;
		forecast = null;
		loading = false;
		error = null;
		pickables = [];
		tips = new Map();
		requested = null;
		current = null;
		repoPath = null;
		preset = null;
		phase = 'plan';
		returnTo = 'plan';
		typedMessage = null;
		landing = false;
		landError = null;
		landed = null;
		settlement += 1;
		checkout = null;
		pushTo = null;
		pushKnown = false;
		pushReading = false;
		origin = null;
		prMethod = 'merge';
		prTitle = '';
		prMessage = '';
		prMerging = false;
		prError = null;
		prMerged = false;
		preparing = false;
		prepareError = null;
		pushing = false;
		pushed = false;
		pushError = null;
		farmError = null;
		seq += 1;
	}
};
