// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The review room's state (FEAT-091): one pull request, read file by file.
 *
 * The pull request's head is fetched and diffed on disk (FEAT-089), so every
 * file can be read whole and folds open without asking the host again. When
 * the fetch cannot happen — no remote, no network, a host that refuses — the
 * room falls back to the patch the host sends, which has the changed parts
 * and nothing between them, and says so.
 *
 * What the reviewer does here that should outlast the window — which files
 * are viewed — is kept in the pull request's record (`store.svelte.ts`). The
 * rest is how the room is being looked at, and lasts as long as the room.
 */

import * as api from '../api';
import type { ReviewKey } from '../api';
import { detectLanguage, tokenizeDiff, type Token } from '../diff/highlight';
import { pairWords, type Segment } from '../diff/words';
import { reading } from '../reading.svelte';
import { notice } from '../ui/notice.svelte';
import type {
	ConflictFix,
	ConflictFixes,
	DiffLine,
	FileDiff,
	FileStatus,
	PullHead,
	PullRequest,
	PullRequestComment,
	ReviewVerdict
} from '../types';
import { pendingOn, toDraft } from './drafts';
import type { PendingComment } from './record';
import { blocksOf, blocksOfHunks, lineKey, type Block, type Laid, type Scope, type SideName, type Composing } from './rows';
import { review } from './store.svelte';
import { threadsOf, type Thread } from './threads';

export type Layout = 'one' | 'all';
export type Phase = 'idle' | 'reading' | 'ready' | 'failed';
export type Panel = 'open' | 'resolved';
/** Every file; the ones with the author's own changes; the ones with conflict fixes. */
export type Filter = 'all' | 'author' | 'conflict';

/** A file the pull request touches, as the files list shows it. */
export interface RoomFile {
	path: string;
	oldPath: string | null;
	status: FileStatus;
	binary: boolean;
	tooLarge: boolean;
	added: number;
	removed: number;
	/**
	 * What a viewed tick is kept against: the new blob, so a file the author
	 * changes afterwards comes back unticked. A deleted file has no new blob
	 * and is kept against its old one; a file read from the host's patch has
	 * neither, and is kept against the head it was read at.
	 */
	mark: string;
}

/** A file's lines once read, and what the screen needs cut from them. */
export interface Content {
	lines: DiffLine[];
	/** The host's hunks, when the file came as a patch. Null when read from disk. */
	hunks: Block[] | null;
	/** Changed words, by line index (FEAT-090). */
	words: Map<number, Segment[]>;
	/** Each line's colours, by line index, each side read on its own (FEAT-094). */
	syntax: Token[][];
	/** Instead of lines: binary, too large, could not be read. */
	note?: string;
}

/** How many files are read at once when the room shows them all. */
const READERS = 4;

let current = $state<{ pr: PullRequest; key: ReviewKey } | null>(null);
let phase = $state<Phase>('idle');
let error = $state<string | null>(null);
/** Why the head could not be fetched, when the room fell back to the host's patch. */
let fallback = $state<string | null>(null);
let head = $state.raw<PullHead | null>(null);
let files = $state.raw<RoomFile[]>([]);
let contents = $state.raw<Record<string, Content>>({});
let selected = $state<string | null>(null);
let scope = $state<Scope>('changes');
let layout = $state<Layout>('one');
let expanded = $state.raw<Record<string, number[]>>({});
let focus = $state<string | null>(null);
/** A row to bring into view once it is drawn: a jump from the Conversation card. */
let target = $state<string | null>(null);
let ruler = $state(true);
let readingSet = $state(true);
let comments = $state.raw<PullRequestComment[]>([]);
let commentsError = $state<string | null>(null);
let panel = $state<Panel>('open');
let conflicts = $state.raw<ConflictFixes | null>(null);
let conflictsError = $state<string | null>(null);
let filter = $state<Filter>('all');
/** Which side of a conflict is open under a card's header, by card. */
let sides = $state.raw<Map<string, SideName>>(new Map());
/** Where a comment is being written (FEAT-093). */
let composer = $state<Composing | null>(null);
/** The text for the pull request as a whole, as typed. */
let body = $state('');
let bodyTimer: ReturnType<typeof setTimeout> | null = null;
let sending = $state(false);

const threads = $derived(threadsOf(comments));
const reading_ = new Set<string>();
let seq = 0;

function noteFor(file: RoomFile): string | null {
	if (file.binary) return 'Binary file. There are no lines to show.';
	if (file.tooLarge) return 'This file is too large to show.';
	return null;
}

function put(path: string, content: Content): void {
	contents = { ...contents, [path]: content };
}

function fileAt(path: string | null): RoomFile | null {
	return path === null ? null : (files.find((file) => file.path === path) ?? null);
}

async function readComments(number: number, mine: number): Promise<void> {
	try {
		const found = await api.reviewComments(number);
		if (mine !== seq) return;
		comments = found;
		commentsError = null;
	} catch (e) {
		if (mine !== seq) return;
		comments = [];
		commentsError = String(e);
	}
}

/**
 * What the pull request's merges wrote while fixing conflicts (FEAT-092), and
 * keep which files and merges in the record, for the inbox.
 */
async function readConflicts(fetched: PullHead, key: ReviewKey, mine: number): Promise<void> {
	try {
		const found = await api.reviewConflicts(fetched.mergeBase, fetched.head, fetched.base);
		if (mine !== seq) return;
		conflicts = found;
		conflictsError = null;
		await review.saveRecord(key, (record) => {
			record.conflictFiles = found.files.map((file) => file.path);
			record.conflictMerges = [...new Set(found.fixes.map((fix) => fix.short))];
		});
	} catch (e) {
		if (mine !== seq) return;
		conflicts = null;
		conflictsError = String(e);
	}
}

/** The files as the host's patch has them, when the head could not be fetched. */
function fromPatch(patch: FileDiff[], headSha: string): RoomFile[] {
	const out: RoomFile[] = [];
	const read: Record<string, Content> = {};
	for (const file of patch) {
		out.push({
			path: file.path,
			oldPath: null,
			status: file.status,
			binary: file.binary,
			tooLarge: file.tooLarge,
			added: file.added,
			removed: file.removed,
			mark: `head:${headSha}`
		});
		const laid = blocksOfHunks(file.hunks);
		read[file.path] = {
			lines: laid.lines,
			hunks: laid.blocks,
			words: pairWords(laid.lines),
			syntax: tokenizeDiff(laid.lines, detectLanguage(file.path)),
			note: noteFor(out[out.length - 1]) ?? undefined
		};
	}
	contents = read;
	return out;
}

export const room = {
	get pr(): PullRequest | null {
		return current?.pr ?? null;
	},

	get phase(): Phase {
		return phase;
	},

	get error(): string | null {
		return error;
	},

	/** Why the room is reading the host's patch rather than the fetched head. */
	get fallback(): string | null {
		return fallback;
	},

	get head(): PullHead | null {
		return head;
	},

	get files(): RoomFile[] {
		return files;
	},

	/**
	 * Open a pull request: fetch its head, list what it changes, and land on
	 * the first file not yet viewed.
	 */
	async load(pr: PullRequest, key: ReviewKey): Promise<void> {
		const mine = ++seq;
		current = { pr, key };
		phase = 'reading';
		error = null;
		fallback = null;
		head = null;
		files = [];
		contents = {};
		selected = null;
		scope = 'changes';
		layout = 'one';
		expanded = {};
		focus = null;
		target = null;
		ruler = reading.current.ruler !== 'off';
		readingSet = true;
		comments = [];
		commentsError = null;
		panel = 'open';
		conflicts = null;
		conflictsError = null;
		filter = 'all';
		sides = new Map();
		composer = null;
		body = review.recordAt(key)?.body ?? '';
		if (bodyTimer) clearTimeout(bodyTimer);
		bodyTimer = null;
		sending = false;
		reading_.clear();

		void readComments(pr.number, mine);

		let headSha = pr.headSha;
		try {
			const fetched = await api.reviewCheckout(pr.number, pr.targetBranch, pr.headSha);
			const listed = await api.reviewFiles(fetched.mergeBase, fetched.head);
			if (mine !== seq) return;
			head = fetched;
			headSha = fetched.head;
			files = listed.map((file) => ({
				path: file.path,
				oldPath: file.oldPath ?? null,
				status: file.status,
				binary: file.binary,
				tooLarge: file.tooLarge,
				added: file.added,
				removed: file.removed,
				mark: file.newBlob ?? (file.oldBlob ? `gone:${file.oldBlob}` : `head:${fetched.head}`)
			}));
		} catch (fetchError) {
			if (mine !== seq) return;
			try {
				const patch = await api.pullRequestFiles(pr.number);
				if (mine !== seq) return;
				files = fromPatch(patch, headSha);
				fallback = String(fetchError);
			} catch {
				if (mine !== seq) return;
				error = String(fetchError);
				phase = 'failed';
				return;
			}
		}

		// Ticks kept against a file that has changed since are dropped: the
		// file reads as unviewed, and the count of viewed files stays true.
		const marks = new Map(files.map((file) => [file.path, file.mark]));
		const record = await review.saveRecord(key, (kept) => {
			for (const [path, mark] of Object.entries(kept.viewed)) {
				if (marks.get(path) !== mark) delete kept.viewed[path];
			}
			kept.headSha = headSha;
			kept.files = files.length;
		});
		if (mine !== seq) return;

		const first = files.find((file) => record.viewed[file.path] !== file.mark) ?? files[0] ?? null;
		selected = first?.path ?? null;
		phase = 'ready';
		if (selected) void this.ensure(selected);
		if (head) void readConflicts(head, key, mine);
	},

	/** Leave the room; anything still on its way is dropped. */
	leave(): void {
		seq += 1;
		current = null;
		phase = 'idle';
		head = null;
		files = [];
		contents = {};
		comments = [];
		conflicts = null;
		reading_.clear();
	},

	/** A file's content, or undefined while it has not been read. */
	contentOf(path: string): Content | undefined {
		return contents[path];
	},

	/** Read a file, unless it has been or is being read. */
	async ensure(path: string): Promise<void> {
		const file = fileAt(path);
		if (!file || contents[path] || reading_.has(path) || !head) return;
		const note = noteFor(file);
		if (note) {
			put(path, { lines: [], hunks: null, words: new Map(), syntax: [], note });
			return;
		}
		const mine = seq;
		const from = head.mergeBase;
		const to = head.head;
		reading_.add(path);
		try {
			const whole = await api.reviewFile(from, to, path, file.oldPath);
			if (mine !== seq) return;
			put(path, {
				lines: whole.lines,
				hunks: null,
				words: pairWords(whole.lines),
				syntax: tokenizeDiff(whole.lines, detectLanguage(path))
			});
		} catch (e) {
			if (mine !== seq) return;
			put(path, { lines: [], hunks: null, words: new Map(), syntax: [], note: `This file could not be read: ${e}` });
		} finally {
			reading_.delete(path);
		}
	},

	/** Read every file, a few at a time, in order. */
	async ensureAll(): Promise<void> {
		const queue = files.map((file) => file.path).filter((path) => !contents[path]);
		const work = async () => {
			for (let path = queue.shift(); path !== undefined; path = queue.shift()) {
				await this.ensure(path);
			}
		};
		await Promise.all(Array.from({ length: READERS }, work));
	},

	/** The files the diff column shows, laid out. */
	get laid(): Laid[] {
		const shown = layout === 'one' ? files.filter((file) => file.path === selected) : this.visible;
		return shown.map((file) => {
			const content = contents[file.path];
			if (!content) return { path: file.path, lines: null, blocks: [] };
			if (content.note) return { path: file.path, lines: [], blocks: [], note: content.note };
			const blocks =
				content.hunks ?? cut(file.path, content.lines);
			return { path: file.path, lines: content.lines, blocks, fixes: this.fixesIn(file.path) };
		});
	},

	// --- Conflict fixes (FEAT-092) -----------------------------------------

	get conflicts(): ConflictFixes | null {
		return conflicts;
	},

	get conflictsError(): string | null {
		return conflictsError;
	},

	fixesIn(path: string): ConflictFix[] {
		return conflicts?.fixes.filter((fix) => fix.path === path) ?? [];
	},

	hasFix(path: string): boolean {
		return conflicts?.files.some((file) => file.path === path) ?? false;
	},

	/** The pull request's author changed it, beyond any conflict fix. */
	isAuthors(path: string): boolean {
		return conflicts?.files.find((file) => file.path === path)?.author ?? true;
	},

	get filter(): Filter {
		return filter;
	},

	setFilter(next: Filter): void {
		filter = next;
	},

	/** The files the filter lets through: the list, and the column in All. */
	get visible(): RoomFile[] {
		if (filter === 'author') return files.filter((file) => this.isAuthors(file.path));
		if (filter === 'conflict') return files.filter((file) => this.hasFix(file.path));
		return files;
	},

	/** Which side of a card's conflict is open, by card. */
	get sides(): ReadonlyMap<string, SideName> {
		return sides;
	},

	/** Show one side of a card's conflict, or close it when it is the one open. */
	toggleSide(chunk: string, side: SideName): void {
		const next = new Map(sides);
		if (next.get(chunk) === side) next.delete(chunk);
		else next.set(chunk, side);
		sides = next;
	},

	get selected(): string | null {
		return selected;
	},

	get selectedFile(): RoomFile | null {
		return fileAt(selected);
	},

	/** Open one file on its own. */
	select(path: string): void {
		if (!fileAt(path)) return;
		selected = path;
		layout = 'one';
		void this.ensure(path);
	},

	/** The selected file's place in the list, from 0. */
	get position(): number {
		return files.findIndex((file) => file.path === selected);
	},

	/** The file before or after the selected one, round the end. */
	step(by: 1 | -1): void {
		if (files.length === 0) return;
		const at = Math.max(0, this.position);
		this.select(files[(at + by + files.length) % files.length].path);
	},

	get scope(): Scope {
		return scope;
	},

	/** Whole file needs the files from disk; the host's patch has only the changes. */
	get canShowWhole(): boolean {
		return head !== null;
	},

	setScope(next: Scope): void {
		if (next === 'whole' && !this.canShowWhole) return;
		scope = next;
		// Back to the changes means back to them folded.
		if (next === 'changes') expanded = {};
	},

	get layout(): Layout {
		return layout;
	},

	setLayout(next: Layout): void {
		layout = next;
		if (next === 'all') void this.ensureAll();
	},

	/** Show the unchanged lines a fold hides. */
	expand(path: string, id: number): void {
		const open = expanded[path] ?? [];
		if (open.includes(id)) return;
		expanded = { ...expanded, [path]: [...open, id] };
	},

	// --- Viewed ----------------------------------------------------------

	isViewed(path: string): boolean {
		const file = fileAt(path);
		const record = current ? review.recordAt(current.key) : null;
		return !!file && !!record && record.viewed[path] === file.mark;
	},

	get viewedCount(): number {
		return files.filter((file) => this.isViewed(file.path)).length;
	},

	async setViewed(path: string, viewed: boolean): Promise<void> {
		const file = fileAt(path);
		if (!file || !current) return;
		await review.saveRecord(current.key, (record) => {
			if (viewed) record.viewed[path] = file.mark;
			else delete record.viewed[path];
		});
	},

	toggleViewed(path: string): Promise<void> {
		return this.setViewed(path, !this.isViewed(path));
	},

	/** Tick the selected file and open the next one not viewed, round the end. */
	async viewedNext(): Promise<void> {
		const at = this.position;
		if (at < 0) return;
		await this.setViewed(files[at].path, true);
		for (let i = 1; i < files.length; i++) {
			const next = files[(at + i) % files.length];
			if (!this.isViewed(next.path)) {
				this.select(next.path);
				return;
			}
		}
		layout = 'one';
	},

	// --- Reading aids ----------------------------------------------------

	/** The line the reader is on, as a row key. */
	get focus(): string | null {
		return focus;
	},

	setFocus(key: string | null): void {
		focus = key;
	},

	/** A row the diff column should bring into view, once. */
	get target(): string | null {
		return target;
	},

	reached(): void {
		target = null;
	},

	get ruler(): boolean {
		return ruler;
	},

	/** One line, or the whole chunk: as Settings › Reading says, one line when it says off. */
	get rulerMode(): 'line' | 'chunk' {
		return reading.current.ruler === 'chunk' ? 'chunk' : 'line';
	},

	toggleRuler(): void {
		ruler = !ruler;
	},

	/** Code and comments in the reading set, or as they were before it. */
	get readingSet(): boolean {
		return readingSet;
	},

	toggleReading(): void {
		readingSet = !readingSet;
	},

	// --- Threads -----------------------------------------------------------

	get threads(): Thread[] {
		return threads;
	},

	get commentsError(): string | null {
		return commentsError;
	},

	get panel(): Panel {
		return panel;
	},

	setPanel(next: Panel): void {
		panel = next;
	},

	/** Open threads on a file, for its row in the list. */
	openThreadsOn(path: string): number {
		return threads.filter((thread) => thread.path === path && !thread.resolved).length;
	},

	/**
	 * Go to a thread: its file, opened alone, with the fold over its line
	 * opened, and the line in focus.
	 */
	async jumpTo(thread: Thread): Promise<void> {
		if (thread.line === null) return;
		await goTo(thread.path, thread.side, thread.line, `${thread.path}\nthread\n${thread.id}`);
	},

	/**
	 * Resolve a thread on the host, or open it again (FEAT-093). Shown at
	 * once, and put back with the reason if the host refuses.
	 */
	async setResolved(thread: Thread, resolved: boolean): Promise<void> {
		if (!current || !thread.threadId) return;
		const ids = new Set(thread.comments.map((comment) => comment.id));
		const mark = (to: boolean) => {
			comments = comments.map((comment) => (ids.has(comment.id) ? { ...comment, resolved: to } : comment));
		};
		mark(resolved);
		try {
			await api.resolveThread(current.pr.number, thread.threadId, resolved);
		} catch (e) {
			mark(!resolved);
			notice.failed(resolved ? 'The thread was not resolved' : 'The thread was not reopened', e);
		}
	},

	/** Answer a thread on the host, now: a reply is not held for the review. */
	async reply(thread: Thread, body: string): Promise<boolean> {
		if (!current || body.trim() === '') return false;
		try {
			const posted = await api.replyComment(current.pr.number, thread.id, body.trim());
			comments = [...comments, posted];
			return true;
		} catch (e) {
			notice.failed('The reply was not sent', e);
			return false;
		}
	},

	// --- Pending comments (FEAT-093) -----------------------------------------

	/** The lines a comment is being written on, or null. */
	get composer(): Composing | null {
		return composer;
	},

	/** Start a comment on one line. */
	compose(path: string, index: number): void {
		if (!head || !contents[path]) return;
		composer = { path, from: index, to: index };
	},

	/**
	 * Cover a range: from where the comment being written starts — or the
	 * line in focus — to `index`, in the same file.
	 */
	extend(path: string, index: number): void {
		if (!head || !contents[path]) return;
		if (composer && composer.path === path) {
			composer = { path, from: composer.from, to: index };
			return;
		}
		const at = focus?.startsWith(`${path}\n`) ? Number(focus.slice(path.length + 1)) : NaN;
		composer = { path, from: Number.isInteger(at) ? at : index, to: index };
	},

	closeComposer(): void {
		composer = null;
	},

	/** Every comment waiting for Finish review, against any head. */
	get drafts(): PendingComment[] {
		return (current ? review.recordAt(current.key)?.drafts : null) ?? [];
	},

	/** Written against the head being read: placed on their lines, and sent. */
	get currentDrafts(): PendingComment[] {
		const sha = head?.head ?? current?.pr.headSha ?? '';
		return this.drafts.filter((draft) => draft.headSha === sha);
	},

	/** Written before the author last pushed: their lines may have moved. */
	get olderDrafts(): PendingComment[] {
		const sha = head?.head ?? current?.pr.headSha ?? '';
		return this.drafts.filter((draft) => draft.headSha !== sha);
	},

	/** Keep what is in the composer as a pending comment. */
	async addDraft(body: string): Promise<boolean> {
		const writing = composer;
		const content = writing ? contents[writing.path] : undefined;
		const file = writing ? fileAt(writing.path) : null;
		if (!current || !head || !writing || !content || !file || body.trim() === '') return false;
		const draft = pendingOn(writing.path, file.oldPath, content.lines, writing.from, writing.to, body, head.head);
		await review.saveRecord(current.key, (record) => {
			record.drafts.push(draft);
		});
		composer = null;
		return true;
	},

	async removeDraft(id: string): Promise<void> {
		if (!current) return;
		await review.saveRecord(current.key, (record) => {
			record.drafts = record.drafts.filter((draft) => draft.id !== id);
		});
	},

	async jumpToDraft(draft: PendingComment): Promise<void> {
		await goTo(draft.path, draft.side, draft.line, `${draft.path}\ndraft\n${draft.id}`);
	},

	/** What is written for the pull request as a whole, waiting with them. */
	get body(): string {
		return body;
	},

	/** Keep it a moment after the typing stops: a file written per key is a lot. */
	setBody(text: string): void {
		body = text;
		if (bodyTimer) clearTimeout(bodyTimer);
		const key = current?.key;
		bodyTimer = setTimeout(() => {
			bodyTimer = null;
			if (key) void review.saveRecord(key, (record) => (record.body = text));
		}, 400);
	},

	get sending(): boolean {
		return sending;
	},

	/**
	 * Finish review: every pending comment written against this head, and
	 * the text for the whole pull request, sent as one review with its
	 * verdict. Sent comments leave the record; older ones stay to be looked
	 * at again.
	 */
	async finish(verdict: ReviewVerdict): Promise<boolean> {
		if (!current || sending) return false;
		const opened = current;
		const sendNow = this.currentDrafts;
		const text = body;
		sending = true;
		try {
			await api.submitReview(opened.pr.number, verdict, text, sendNow.map(toDraft));
			if (bodyTimer) clearTimeout(bodyTimer);
			bodyTimer = null;
			const sent = new Set(sendNow.map((draft) => draft.id));
			await review.saveRecord(opened.key, (record) => {
				record.drafts = record.drafts.filter((draft) => !sent.has(draft.id));
				record.body = '';
			});
			body = '';
			notice.ok('Review sent', VERDICTS[verdict]);
			void readComments(opened.pr.number, seq);
			return true;
		} catch (e) {
			notice.failed('The review was not sent', e);
			return false;
		} finally {
			sending = false;
		}
	}
};

/** A file read from disk, cut into blocks as the room shows it. */
function cut(path: string, lines: DiffLine[]): Block[] {
	const fixed = new Set(room.fixesIn(path).flatMap((fix) => fix.lines));
	return blocksOf(lines, scope, new Set(expanded[path] ?? []), fixed);
}

/** What the toast says each verdict was. */
const VERDICTS: Record<ReviewVerdict, string> = {
	approve: 'Approved',
	requestChanges: 'Changes requested',
	comment: 'Commented'
};

/**
 * Go to a line: its file, opened alone, with the fold over it opened, the
 * line in focus, and `row` brought into view once it is drawn.
 */
async function goTo(path: string, side: 'LEFT' | 'RIGHT', line: number, row: string): Promise<void> {
	if (!fileAt(path)) return;
	room.select(path);
	await room.ensure(path);
	const content = contents[path];
	if (!content || content.note) return;
	const index = content.lines.findIndex((candidate) =>
		side === 'LEFT' ? candidate.old === line && candidate.origin !== 'added' : candidate.new === line
	);
	if (index < 0) return;
	if (!content.hunks) {
		const fold = cut(path, content.lines).find(
			(block) => block.kind === 'fold' && block.from <= index && index < block.to
		);
		if (fold) room.expand(path, fold.from);
	}
	focus = lineKey(path, index);
	target = row;
}
