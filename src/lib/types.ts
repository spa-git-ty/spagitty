// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Wire types shared with the Rust core.
 *
 * These mirror the `serde` shapes in `crates/spagitty-core` and
 * `src-tauri/src/commands.rs`, which all carry
 * `#[serde(rename_all = "camelCase")]`. Keep the two in step.
 */

// --- Refs -----------------------------------------------------------------

export type RefKind = 'branch' | 'remote' | 'tag';

/** Which forge a remote points at, decided from its URL. Picks a glyph, nothing more. */
export type Host = 'gitHub' | 'gitLab' | 'bitbucket' | 'azureDevOps' | 'generic';

/** A remote carrying a branch at a commit. */
export interface RemoteMark {
	/** `origin`, `upstream`. Not drawn on the chip; carried for the tooltip. */
	name: string;
	host: Host;
}

export interface RefChip {
	/**
	 * The branch's own short name — `master`, never `origin/master` — or the
	 * tag's name. Where a branch lives is `local` and `remotes` (FEAT-036).
	 */
	name: string;
	/**
	 * `branch` when a local ref exists, `remote` when it lives only on a remote,
	 * `tag` for tags. What the gutter sorts and styles by.
	 */
	kind: RefKind;
	/** True for the branch HEAD points at. Renders with accent border and a check. */
	current: boolean;
	/** A local `refs/heads/` ref of this name is at this commit. */
	local: boolean;
	/** The remotes carrying this branch **at this commit**, in name order. */
	remotes: RemoteMark[];
	/**
	 * How far the local branch has drifted from its upstream (FEAT-033).
	 *
	 * Null for a tag, for a branch with no upstream, and for a chip with no
	 * local ref — none of them has anything to have drifted from. It comes from
	 * the same read the Branches screen uses, so the two cannot disagree.
	 */
	divergence: BranchDivergence | null;
}

// --- Graph ----------------------------------------------------------------

/**
 * A lane segment in the band *above* a row, running from the previous row's
 * center to this row's. `from === to` is a straight vertical; otherwise it is a
 * cubic elbow spanning exactly one row.
 */
export interface LaneEdge {
	from: number;
	to: number;
	/** Index into the lane color cycle. Stable for a lane's whole lifetime. */
	color: number;
}

/** One commit row. Everything needed to paint it, with no global state. */
export interface GraphRow {
	/** Absolute index in the walk. Row `i` is at `y = i * ROW_PITCH`. */
	index: number;
	id: string;
	short: string;
	/** First line of the commit message. */
	summary: string;
	authorName: string;
	/**
	 * The author's email, lower-cased, or empty where the signature has none.
	 *
	 * What the node's portrait is generated from: one person commits under
	 * several spellings of their name over a career, and the address is what
	 * makes those one face.
	 */
	authorEmail: string;
	/** Up to two uppercase letters. The fallback where no portrait can be drawn. */
	initials: string;
	/** Author time, unix seconds. */
	time: number;
	/** Lane this commit's node sits in. */
	lane: number;
	color: number;
	/**
	 * The commit carries a signature (FEAT-019).
	 *
	 * Read off the object's `gpgsig` header as the walk passes it. It says the
	 * commit **was signed**, not that the signature is valid — verifying means a
	 * subprocess and a keyring per row, and the screens say "signed" rather than
	 * "verified" for exactly that reason.
	 */
	signed: boolean;
	parents: string[];
	refs: RefChip[];
	edges: LaneEdge[];
}

// --- Commit detail --------------------------------------------------------

export type FileStatus = 'added' | 'modified' | 'deleted' | 'renamed' | 'untracked';

export interface ChangedFile {
	path: string;
	status: FileStatus;
}

export interface CommitDetail {
	id: string;
	short: string;
	summary: string;
	/** Everything after the first line, trimmed. May be empty. */
	body: string;
	authorName: string;
	authorEmail: string;
	authorTime: number;
	committerName: string;
	committerEmail: string;
	commitTime: number;
	/** The commit carries a signature. Present, not verified — see `GraphRow`. */
	signed: boolean;
	parents: string[];
	files: ChangedFile[];
}

// --- Diff -----------------------------------------------------------------

export type LineOrigin = 'context' | 'added' | 'removed';

export interface DiffLine {
	origin: LineOrigin;
	/** 1-based line number in the old version. Null on an added line. */
	old: number | null;
	/** 1-based line number in the new version. Null on a removed line. */
	new: number | null;
	/** The line's text, without its terminator. */
	text: string;
}

/** One run of changes plus its context — a `@@` block. */
export interface Hunk {
	oldStart: number;
	oldLines: number;
	newStart: number;
	newLines: number;
	header: string;
	lines: DiffLine[];
}

/** A row in the Diff screen's file list. */
export interface FileChange {
	path: string;
	status: FileStatus;
	/**
	 * No line diff exists: the file is binary, or one side was too large. In
	 * both cases `added` and `removed` are 0 rather than a guess.
	 */
	binary: boolean;
	tooLarge: boolean;
	added: number;
	removed: number;
}

export interface CommitDiff {
	id: string;
	short: string;
	summary: string;
	files: FileChange[];
	/** Totals across every file, for the header. */
	added: number;
	removed: number;
}

export interface FileDiff {
	path: string;
	status: FileStatus;
	binary: boolean;
	tooLarge: boolean;
	added: number;
	removed: number;
	hunks: Hunk[];
}

/** Detailed binary / image diff inspection payload (FEAT-065). */
export interface BinaryDiff {
	path: string;
	isImage: boolean;
	mime: string;
	oldSize: number | null;
	newSize: number | null;
	oldBase64: string | null;
	newBase64: string | null;
}

// --- Working copy ---------------------------------------------------------

/** Which two sides of the working copy a diff compares. */
export type DiffSide = 'staged' | 'unstaged';

/** One path in the working copy, on the side it was found. */
export interface StatusEntry {
	path: string;
	status: FileStatus;
}

/**
 * The working copy, split the way the Commit screen shows it.
 *
 * A path can be in `staged` and `unstaged` at once — staged in part, or
 * changed again afterwards — so these are three lists rather than one list of
 * rows carrying flags.
 */
export interface WorkingCopy {
	/** HEAD against the index: what a commit right now would contain. */
	staged: StatusEntry[];
	/** The index against the working tree, plus untracked files. */
	unstaged: StatusEntry[];
	/** Paths the index holds at stages 1 to 3. Nothing can be committed yet. */
	conflicted: StatusEntry[];
}

// --- Branches -------------------------------------------------------------

/** One row of the Branches screen. */
export interface BranchRow {
	/** Short name for display: `main`, `origin/main`. */
	name: string;
	/** The full ref, for anything that has to be unambiguous. */
	fullName: string;
	kind: RefKind;
	current: boolean;
	id: string;
	short: string;
	/** First line of the tip's message. */
	summary: string;
	authorName: string;
	/** Author time of the tip, unix seconds. */
	time: number;
	/** Short name of the configured upstream, when there is one. */
	upstream: string | null;
	/**
	 * Commits this branch has that its upstream does not, and the other way
	 * round. Null when there is no upstream to compare against. As old as the
	 * last fetch: nothing here talks to a network.
	 */
	ahead: number | null;
	behind: number | null;
	/** Fully contained in HEAD — the branch that is safe to forget. */
	merged: boolean;
}

// --- Stash ----------------------------------------------------------------

/**
 * One `stash@{n}`.
 *
 * A stash is a commit whose first parent is the commit the work was made on, so
 * `commitDiff(entry.id)` shows what is in it — there is no separate stash-diff
 * call, and there does not need to be.
 */
export interface StashEntry {
	/** `n` in `stash@{n}`. 0 is the most recent. */
	index: number;
	/** `stash@{n}` — the name git itself uses. */
	name: string;
	id: string;
	short: string;
	/** What the entry says about itself: `On main: wip on notes`. */
	message: string;
	time: number;
	authorName: string;
	/** The commit the work was made on — the row it hangs off. */
	parent: string;
	parentShort: string;
	parentSummary: string;
}

// --- Pull requests --------------------------------------------------------

/**
 * The shape a pull request takes on the screen.
 *
 * **This is FEAT-017's contract.** Nothing populates it today — Spagitty talks
 * to no hosting service — and it is defined now so that connecting a host is a
 * matter of filling this in rather than redesigning the screen around whatever
 * one host's API happens to return.
 *
 * The vocabulary is host-agnostic on purpose: "pull request", never a brand.
 * The hosting service is a detail, not the language.
 */
export interface PullRequest {
	/** The host's own identifier, whatever form it takes. */
	id: string;
	/** The number people say out loud: "#412". */
	number: number;
	title: string;
	/** Description / markdown changelog text. */
	body: string;
	/** Display name of whoever opened it. */
	authorName: string;
	/** Seconds since the unix epoch. */
	updated: number;
	sourceBranch: string;
	targetBranch: string;
	draft: boolean;
	/** Where it sits with reviewers. */
	review: ReviewState;
	/** Whether the host's checks passed. Null when the host runs none. */
	checks: CheckState | null;
	/** True when this one is waiting on the person using Spagitty. */
	needsYou: boolean;
	/** Why it needs you, in one line, when it does. */
	needsYouBecause: string | null;
	changedFiles: number;
	added: number;
	removed: number;
	/** Whether the host says it can merge. Null when the host has not said. */
	mergeable: boolean | null;
}

export type ReviewState = 'awaitingReview' | 'changesRequested' | 'approved' | 'noReviewers';

/**
 * What a reviewer is saying, in Spagitty's own words (FEAT-058).
 *
 * Mapped onto the host's vocabulary at the edge, the way `ReviewState` already
 * is: GitHub calls these `APPROVE`, `REQUEST_CHANGES` and `COMMENT`.
 */
export type ReviewVerdict = 'approve' | 'requestChanges' | 'comment';

/** Merge strategies supported across forges (FEAT-071). */
export type MergeMethod = 'merge' | 'squash' | 'rebase';

/** A commit belonging to a pull request (FEAT-059). */
export interface PullRequestCommit {
	sha: string;
	short: string;
	summary: string;
	authorName: string;
	authorEmail: string;
	time: number;
}

/** An inline review comment or thread reply (FEAT-059). */
export interface PullRequestComment {
	id: number;
	inReplyTo: number | null;
	path: string;
	line: number | null;
	side: string;
	body: string;
	author: string;
	createdAt: number;
	resolved: boolean;
}

/** A local draft inline comment waiting to be published with a review (FEAT-059). */
export interface DraftComment {
	path: string;
	line: number;
	side: string;
	body: string;
}

/** Which hosting service a remote points at (FEAT-017, FEAT-070). */
export type ForgeKind = 'gitHub' | 'gitLab' | 'bitbucket';
/** A repository on a hosting service, identified from a git remote. */
export interface ForgeRepo {
	kind: ForgeKind;
	/** The hostname, so an enterprise installation is not handed the wrong token. */
	host: string;
	owner: string;
	name: string;
}

/**
 * A connected account.
 *
 * **No token.** It lives in the OS keychain and never crosses this boundary —
 * a token in a type the webview can hold is a token in a devtools inspector.
 */
export interface ForgeAccount {
	kind: ForgeKind;
	host: string;
	/** The login the token authenticates as, read back from the host. */
	user: string;
}

export type CheckState = 'passing' | 'failing' | 'running';

// --- Interactive rebase ---------------------------------------------------

/** What to do with one commit, in git's own vocabulary. */
export type RebaseAction = 'pick' | 'squash' | 'reword' | 'drop';

/** One row of the todo list, as `git rebase -i` would open it. */
export interface TodoRow {
	id: string;
	short: string;
	summary: string;
	authorName: string;
	time: number;
	/** Paths this commit changed. What the conflict heuristic compares. */
	paths: string[];
}

export interface RebaseTodo {
	/** The commit the rebase would replay onto. */
	upstream: string;
	upstreamShort: string;
	/** Oldest first — a rebase replays forwards. */
	rows: TodoRow[];
	/** True when the range was longer than the cap and was cut. */
	truncated: boolean;
}

/** One entry of the plan. The plan is the complete list, and its order is the order. */
export interface RebaseEdit {
	id: string;
	action: RebaseAction;
	/**
	 * The new message, for a `reword`.
	 *
	 * Collected on screen rather than at execution time: "at execution time"
	 * means git opening an editor on a terminal Spagitty does not have. A reword
	 * carrying no message is executed as a plain pick.
	 */
	message?: string | null;
}

// --- History operations ---------------------------------------------------

/**
 * How far back a reset takes the index and the working tree.
 *
 * `hard` is the one that loses uncommitted work, and it is the reason this is a
 * union of three names rather than a boolean called `force`: every screen that
 * offers it has to spell out which one it is offering.
 */
export type ResetMode = 'soft' | 'mixed' | 'hard';

/** What dropping one branch onto another can turn into. */
export type Integration = 'merge' | 'mergeNoFastForward' | 'fastForward' | 'rebase';

/**
 * How a pull brings the remote's commits in.
 *
 * `fastForwardOnly` is the safe one: it refuses unless the local branch can
 * simply move forward, so it can never write a merge commit or leave a conflict.
 */
export type PullMode = 'fastForwardOnly' | 'merge' | 'rebase';

/** What to do with a stash entry. */
export type StashAction = 'apply' | 'pop' | 'drop';

/** One row of the result: what a commit would become. */
export interface PreviewRow {
	id: string;
	short: string;
	summary: string;
	/** Commits folded into this one, oldest first. Empty for a plain pick. */
	absorbed: string[];
	reworded: boolean;
	/**
	 * An earlier row in the plan touched a path this one also touches, so
	 * replaying it may not apply cleanly. A heuristic — knowing for certain
	 * means performing the merges, which is execution.
	 */
	mayConflict: boolean;
}

export interface RebasePreview {
	rows: PreviewRow[];
	dropped: string[];
	/**
	 * Set when the plan cannot be executed as written. Dropping everything is
	 * not one of these — an empty result is a legitimate thing to look at.
	 */
	refusal: string | null;
	/** True when the plan would leave no commits at all. */
	emptiesTheBranch: boolean;
}

// --- Log search -----------------------------------------------------------

/**
 * What to look for. Every field is optional and they compose as AND.
 *
 * Text matching is a case-insensitive substring, not a regular expression —
 * regexes are explicit non-scope for this pass.
 */
export interface SearchQuery {
	/** Matched against `Name <email>`, the way `git log --author` looks at it. */
	author?: string | null;
	/** Matched against the whole message, subject and body, like `--grep`. */
	message?: string | null;
	/** A path the commit changed, like `git log -- <path>`. */
	path?: string | null;
	/** Search within added/removed patch lines (FEAT-066). */
	diffContent?: string | null;
	/** Seconds since the unix epoch, like `--since`. */
	since?: number | null;
	/** Seconds since the unix epoch, like `--until`. */
	until?: number | null;
}

/**
 * One result: the graph's row without its lane.
 *
 * A filtered list has no lanes, and drawing them would draw edges between
 * commits that are not parent and child.
 */
export interface SearchRow {
	/** Position in the result list, not in history. */
	index: number;
	id: string;
	short: string;
	summary: string;
	authorName: string;
	authorEmail: string;
	initials: string;
	time: number;
	refs: RefChip[];
}

export interface SearchRowsEvent {
	token: number;
	rows: SearchRow[];
}

export interface SearchDoneEvent {
	token: number;
	total: number;
	/** True when the walk reached the end of history rather than being cancelled. */
	complete: boolean;
	error: string | null;
}

/** Why there is nothing to blame. Not an error — ordinary states of files. */
export type NotBlamable = 'binary' | 'tooLarge' | 'notAFile' | 'empty';

export interface BlameLine {
	/** 1-based line number in the blamed file. */
	line: number;
	text: string;
	commit: string;
	short: string;
	summary: string;
	authorName: string;
	time: number;
	/** Where the line lived before, when it arrived under another name. */
	sourcePath: string | null;
}

export interface Blame {
	path: string;
	/** The revision blamed, resolved to a full id. */
	revision: string;
	lines: BlameLine[];
	/** Set when `lines` is empty for a reason worth stating. */
	refused: NotBlamable | null;
}

/** One commit in a file's evolution history (FEAT-063). */
export interface FileHistoryEntry {
	commit: string;
	short: string;
	authorName: string;
	authorEmail: string;
	time: number;
	summary: string;
}

// --- Conflicts ------------------------------------------------------------

/**
 * What the repository is in the middle of.
 *
 * Read from the repository's own state, never inferred from the presence of
 * conflicts: merge, rebase, cherry-pick and revert all leave conflicts behind,
 * and naming the wrong one sends someone to the wrong command to get out.
 */
export type ConflictOperation =
	| 'merge'
	| 'rebase'
	| 'rebaseInteractive'
	| 'cherryPick'
	| 'revert'
	| 'applyMailbox'
	| 'bisect'
	| 'none';

/**
 * Which sides a conflicted path has, which is the same thing as what kind of
 * conflict it is.
 */
export type ConflictKind = 'bothModified' | 'bothAdded' | 'deletedByUs' | 'deletedByThem';

export interface ConflictFile {
	path: string;
	kind: ConflictKind;
}

/** One version of a conflicted file. */
export interface ConflictSide {
	/** Empty when `binary` or `tooLarge`, because there is nothing honest to show. */
	text: string;
	lines: number;
	bytes: number;
	binary: boolean;
	tooLarge: boolean;
}

/**
 * The three index stages of one conflicted path, plus what is on disk.
 *
 * A missing side is `null` rather than an empty side: "deleted on that side"
 * and "emptied on that side" are different things, and the second loses work if
 * acted on.
 */
export interface ConflictSides {
	path: string;
	kind: ConflictKind;
	/** Stage 1, the common ancestor. Null when both sides added the path. */
	base: ConflictSide | null;
	/** Stage 2 — ours, which is HEAD. */
	ours: ConflictSide | null;
	/** Stage 3 — theirs, the incoming side. */
	theirs: ConflictSide | null;
	/** The working-tree file, markers and all. Null when there is none on disk. */
	merged: ConflictSide | null;
}

export interface ConflictState {
	operation: ConflictOperation;
	files: ConflictFile[];
}

// --- Repository -----------------------------------------------------------

export interface HeadInfo {
	/** Short branch name, or null when detached. */
	branch: string | null;
	detached: boolean;
	id: string | null;
	short: string | null;
}

export interface RepoInfo {
	path: string;
	name: string;
	bare: boolean;
	head: HeadInfo;
	/**
	 * Unix seconds of the last fetch — the mtime of `.git/FETCH_HEAD`, which git
	 * writes on every fetch including one that brought nothing down. `null` for
	 * a repository that has never been fetched, which is a real answer and is
	 * said in words rather than shown as an empty time (FEAT-040).
	 */
	lastFetched: number | null;
}

/**
 * Right-aligned counts in the nav rail.
 *
 * `null` means "not computed yet" — the rail shows a `·` rather than a number.
 * Counts whose screens do not exist yet are null on purpose: a wrong count is
 * worse than no count, because it is what people use to decide whether a screen
 * is worth opening.
 */
export interface RepoCounts {
	commits: number | null;
	working: number | null;
	/** Paths staged for the next commit — what the Commit button counts. */
	staged: number | null;
	conflicts: number | null;
	branches: number | null;
	stashes: number | null;
	tags: number | null;
	submodules: number | null;
}

/**
 * One card on the All repositories screen.
 *
 * Read where the repository sits, without opening it as the current one.
 * `present` false means the path is gone or is no longer a repository, and
 * every other field is a default that means nothing.
 */
export interface RepoSummary {
	path: string;
	name: string;
	present: boolean;
	bare: boolean;
	branch: string | null;
	detached: boolean;
	short: string | null;
	/** First line of the tip's message, so a card says what it was last doing. */
	summary: string | null;
	time: number | null;
	/** Distinct changed paths, or null for a bare repository. */
	dirty: number | null;
	conflicts: number | null;
	stashes: number | null;
	branches: number | null;
}

export interface OpenResult {
	info: RepoInfo;
	counts: RepoCounts;
	/** Identifies this walk. Rows from any other token are stale. */
	token: number;
}

export interface Snapshot {
	info: RepoInfo;
	counts: RepoCounts;
}

export interface About {
	version: string;
	commit: string;
	license: string;
}

// --- Clone ----------------------------------------------------------------

/**
 * Why a clone cannot start.
 *
 * Every one of these is knowable without touching the network, which is the
 * point: the user is told while they are typing rather than after a round trip.
 */
export type CloneProblemKind =
	| 'noUrl'
	| 'unusableUrl'
	| 'noParent'
	| 'missingParent'
	| 'destinationNotEmpty';

export interface CloneProblem {
	kind: CloneProblemKind;
	/** The path the problem is about, where there is one. */
	detail?: string;
}

/** Where a clone would land, and what is wrong with that. */
export interface ClonePlan {
	name: string | null;
	/** The exact path that will be created — what the screen shows. */
	destination: string | null;
	/** True when the destination does not exist yet, so cancelling may remove it. */
	createsDestination: boolean;
	problem: CloneProblem | null;
	/** The sentence to show. Written by the core, so there is only one of them. */
	message: string | null;
}

/** One step of git's own progress reporting. */
export interface CloneProgress {
	phase: string;
	percent: number | null;
	/** git's own words, shown when there is no percentage. */
	line: string;
}

/** Payload of `clone-progress`. */
export interface CloneProgressEvent extends CloneProgress {
	token: number;
}

/** Payload of `clone-done`: the clone for `token` stopped, one way or another. */
export interface CloneDoneEvent {
	token: number;
	ok: boolean;
	/** False for a clone that failed on its own; true for one the user stopped. */
	cancelled: boolean;
	error: string | null;
	path: string;
}

/** Which network operation a worker is running (FEAT-018). */
export type NetworkOperation = 'fetch' | 'push';

/** Payload of `network-progress`: one step of git's own progress reporting. */
export interface NetworkProgressEvent extends CloneProgress {
	token: number;
	operation: NetworkOperation;
}

/** Payload of `network-done`: the fetch or push for `token` has stopped. */
export interface NetworkDoneEvent {
	token: number;
	operation: NetworkOperation;
	ok: boolean;
	error: string | null;
	/**
	 * git's last words on success — "Everything up-to-date", a ref update
	 * summary — so a fetch that brought nothing down can say so.
	 */
	summary: string | null;
}

/** One tag, peeled to the commit it names (FEAT-051). */
export interface Tag {
	/** Short name — `v1.0.0`, not `refs/tags/v1.0.0`. */
	name: string;
	/** The commit it points at, after peeling an annotated tag. */
	target: string;
	targetShort: string;
	/**
	 * True when this is a tag object rather than a ref pointing at a commit.
	 * Only an annotated tag can carry a message or a tagger.
	 */
	annotated: boolean;
	/** The tag's own message. Empty for a lightweight tag, which has none. */
	message: string;
	taggerName: string;
	/**
	 * Unix seconds: the tagger's time for an annotated tag, the commit's own
	 * for a lightweight one, which is the only date it has.
	 */
	time: number;
	/** First line of the tagged commit's message. */
	summary: string;
}

/** One move of one ref (FEAT-050). */
export interface ReflogEntry {
	/** Position from the newest, counting from 0 — the `n` in `HEAD@{n}`. */
	index: number;
	/** The revision this entry names: `HEAD@{3}`, `main@{0}`. */
	revision: string;
	/** Where the ref pointed before this move. */
	before: string;
	beforeShort: string;
	after: string;
	afterShort: string;
	/** True when this entry created the ref — there is nothing before it. */
	created: boolean;
	authorName: string;
	/** Unix seconds. */
	time: number;
	/** git's own description: `commit: subject`, `rebase (finish): …`. */
	message: string;
	/** The word before the colon, which is what a reader scans for. */
	operation: string;
}

/** A ref's reflog, newest first. */
export interface Reflog {
	/** The ref this is the log of, as it should be shown: `HEAD`, `main`. */
	reference: string;
	entries: ReflogEntry[];
	truncated: boolean;
	/**
	 * False when this ref has no reflog at all — reflogs turned off, or a ref
	 * that has never moved. Different from an empty list, and said differently.
	 */
	exists: boolean;
}

/** How far a branch has drifted from its upstream (FEAT-033). */
export interface BranchDivergence {
	/** Short name of the configured upstream. */
	upstream: string;
	ahead: number;
	behind: number;
}

/** One configured remote (FEAT-049). */
export interface Remote {
	name: string;
	/** Where it is fetched from. */
	url: string;
	/**
	 * Where it is pushed to, when that is configured separately. Null is the
	 * ordinary case and means "the same as `url`".
	 */
	pushUrl: string | null;
	/** Which forge the URL points at — the same glyph the graph's chips use. */
	host: Host;
	/**
	 * Refs under `refs/remotes/<name>/`. Zero means it has never been fetched,
	 * which is what tells "added a moment ago" from "gone stale".
	 */
	refs: number;
}

/** One linked working tree attached to a git repository (FEAT-062). */
export interface Worktree {
	/** Absolute filesystem path to the working tree. */
	path: string;
	/** Display name (trailing directory component). */
	name: string;
	/** HEAD commit object ID (full hex SHA). */
	head: string;
	/** Short 7-character commit hash for display. */
	headShort: string;
	/** Short branch name if on a branch, or null if detached. */
	branch: string | null;
	/** True if this is the main / root repository working tree. */
	isMain: boolean;
	/** True if the working tree is bare. */
	isBare: boolean;
	/** True if HEAD is detached (not on any named branch). */
	isDetached: boolean;
	/** Optional lock reason if locked against pruning. */
	lockedReason: string | null;
	/** Optional prunable reason if the worktree gitdir is orphaned or missing. */
	prunableReason: string | null;
}

/** One git submodule in a repository (FEAT-067). */
export interface Submodule {
	name: string;
	path: string;
	url: string;
	headCommit: string | null;
	headShort: string | null;
	initialized: boolean;
	inSync: boolean;
	hasConflict: boolean;
	describe: string | null;
}

/** Known external diff/merge tool information (FEAT-068). */
export interface ExternalToolInfo {
	id: string;
	name: string;
	command: string;
	isInstalled: boolean;
}

/** External tools configuration state (FEAT-068). */
export interface ExternalToolsConfig {
	diffTool: string | null;
	mergeTool: string | null;
	availableDiffTools: ExternalToolInfo[];
	availableMergeTools: ExternalToolInfo[];
}

/** Which side of a conflict to keep. */
export type ConflictSideName = 'ours' | 'theirs';

/**
 * One `<<<<<<< ======= >>>>>>>` block in a file on disk.
 *
 * Line numbers are 1-based and inclusive, matching the merged pane, so a region
 * can be pointed at on screen without a second numbering scheme.
 */
export interface ConflictRegion {
	/** Position in the file, counting from 0. What a caller names it by. */
	index: number;
	startLine: number;
	endLine: number;
	ours: string;
	/** Present only when the file was merged with `diff3` markers. */
	base: string | null;
	theirs: string;
}

/** How far a rebase that is running has got, from git's own state directory. */
export interface RebaseProgress {
	/** Which commit is being applied, counting from 1. */
	step: number;
	total: number;
	/** The branch being rebased. Null for one started from a detached HEAD. */
	branch: string | null;
	/** Short id of `ORIG_HEAD` — where the branch was before any of this. */
	original: string | null;
}

/** Payload of `rebase-progress`. */
export interface RebaseProgressEvent extends RebaseProgress {
	token: number;
}

/**
 * Payload of `rebase-done`: the rebase for `token` is no longer running.
 *
 * `stopped` is the hand-off, not a failure — git got part-way and is waiting
 * for a conflict to be resolved or for an `edit` to be finished.
 */
export interface RebaseDoneEvent {
	token: number;
	ok: boolean;
	stopped: boolean;
	error: string | null;
}

// --- Settings -------------------------------------------------------------

/** A configuration file Spagitty will write to. Never inferred — always chosen. */
export type IdentityScope = 'global' | 'local';

/** The two keys the Settings screen edits, and no others. */
export type IdentityKey = 'name' | 'email';

/**
 * Where the value git would actually use comes from.
 *
 * Wider than `IdentityScope`, because a value can come from somewhere Spagitty
 * will not write: saying "system" out loud is what explains why editing the
 * global field did not change the effective value.
 */
export type IdentityOrigin = 'unset' | 'system' | 'global' | 'local' | 'environment';

export interface IdentityValue {
	/** What git would use. */
	effective: string | null;
	origin: IdentityOrigin;
	/** What each writable scope holds, so an override can be shown beside what it hides. */
	global: string | null;
	local: string | null;
}

export interface Identity {
	name: IdentityValue;
	email: IdentityValue;
	/** False when no repository is open: the local scope is neither read nor written. */
	repository: boolean;
}

/** One saved identity profile (FEAT-069). */
export interface IdentityProfile {
	id: string;
	name: string;
	authorName: string;
	authorEmail: string;
	signingKey: string | null;
}

/** Spagitty's own behaviour toggles, stored in its config directory. */
export interface Settings {
	/**
	 * Ask the project whether there is a newer Spagitty, at startup.
	 *
	 * On by default — the one preference here that changes what the application
	 * does rather than what it checks. There is no package manager behind an
	 * AppImage or a bare `.exe`, so this is the only way somebody finds out
	 * their client is old. Turning it off stops every request.
	 */
	checkForUpdates: boolean;
	confirmHistoryRewrite: boolean;
	showGitCommands: boolean;
	/**
	 * Delete remote-tracking refs the remote no longer has, when fetching.
	 *
	 * Off by default: pruning deletes refs, and a branch vanishing from the
	 * graph because a fetch quietly pruned it is a surprise nobody asked for.
	 */
	pruneOnFetch: boolean;
	/**
	 * Fetch each author's real picture for the graph's nodes (FEAT-079).
	 *
	 * On by default, and the second preference here to change what the
	 * application does and still be on. A generated face disambiguates — these
	 * rows are one person — and does not identify, which is the question a node
	 * is looked at to answer. Off falls back to the generated face and empties
	 * the cache.
	 */
	fetchAvatars: boolean;
	/**
	 * How much personality the delight layer shows (FEAT-072).
	 *
	 * Opt-in intensity, never opt-in existence: badges are earned and the badge
	 * screen works at every level. What this changes is how loudly an unlock
	 * arrives.
	 */
	personality: Personality;
	/** Whether Spagitty makes a sound, and how loud. Off until asked. */
	sound: SoundLevel;
}

/**
 * What is known about one author, beyond what the commit already said.
 *
 * Mirrors `commands::AvatarAnswer`. Both halves are absent more often than
 * not, and neither absence is a failure: a handle needs an address that spells
 * one out, and a picture needs somebody to have uploaded one.
 */
export interface AvatarAnswer {
	handle: string | null;
	/** A `data:` URL, ready to draw. */
	picture: string | null;
}

/** Mirrors `settings::Personality`. */
export type Personality = 'professional' | 'balanced' | 'fullSpagitty';

/** Mirrors `settings::SoundLevel`. */
export type SoundLevel = 'off' | 'subtle' | 'full';

/** Which signing machinery git is configured to use — `gpg.format`. */
export type SigningFormat = 'openPgp' | 'ssh' | 'x509';

/**
 * Why a commit that is meant to be signed would not be.
 *
 * Both are known before the commit is attempted, which is the point: a
 * repository with no working signer is told so at the point of commit rather
 * than after one fails.
 */
export type SigningProblem =
	| { kind: 'missingProgram'; detail: string }
	| { kind: 'noSigningKey' };

/**
 * Commit signing, as git would resolve it here.
 *
 * `commit.gpgsign` is the authority — there is no separate Spagitty preference,
 * because two switches for one behaviour disagree the moment one of them is
 * changed outside this application.
 */
export interface Signing {
	/** What a commit made now would do. */
	enabled: boolean;
	/** Which file `enabled` came from. `unset` means nothing sets it. */
	origin: IdentityOrigin;
	format: SigningFormat;
	/** `user.signingkey`, effective. */
	key: string | null;
	/** The program git would run for this format. */
	program: string;
	/** Null when signing is off: a signer that will not be used cannot fail. */
	problem: SigningProblem | null;
	/** False when no repository is open. */
	repository: boolean;
	/** `commit.gpgsign` as each writable scope holds it. */
	global: boolean | null;
	local: boolean | null;
}

/** Which kind of build is asking about updates. */
export type UpdateChannel = 'released' | 'development';

/** What an update check found. */
export interface Update {
	channel: UpdateChannel;
	/** The tag this build was cut as, or null for a build somebody compiled. */
	current: string | null;
	/** The newest tag the project has published. */
	latest: string;
	/** True only when this is a released build and the latest is a different one. */
	newer: boolean;
	url: string;
}

export interface Dependency {
	name: string;
	version: string;
	/** Null where the package declares none — listed as "not declared", never hidden. */
	license: string | null;
}

/**
 * Every dependency this build is made of, generated from the lockfiles.
 *
 * `generated` false means the build could not produce the list; `notes` says
 * why. An empty list with no explanation would read as "no dependencies".
 */
export interface Licenses {
	generated: boolean;
	notes: string[];
	rust: Dependency[];
	npm: Dependency[];
}

// --- Events ---------------------------------------------------------------

/** Payload of `graph-rows`: one streamed chunk. */
export interface GraphRowsEvent {
	token: number;
	rows: GraphRow[];
}

/** Payload of `graph-done`: the walk for `token` stopped. */
export interface GraphDoneEvent {
	token: number;
	total: number;
	/** True when it reached the end of history rather than being cancelled. */
	complete: boolean;
	error: string | null;
}

/** Payload of `repo-changed`: the filesystem watcher saw something settle. */
export interface RepoChangedEvent {
	refs: boolean;
	worktree: boolean;
}

/**
 * How one executed `git` command ended.
 *
 * `started` is a clone: nothing waits for it, so its outcome arrives long after
 * the line is worth showing.
 */
export type CommandOutcome =
	| { kind: 'ok' }
	| { kind: 'failed'; code: number | null; stderr: string }
	| { kind: 'started' };

/**
 * One `git` command Spagitty actually ran, recorded at the spawn site.
 *
 * Reads are absent by design: log walking, refs, diff and status happen
 * in-process and have no command line. Nothing is invented for them.
 */
export interface ExecutedCommand {
	/** Monotonic within the app's run. Used to ask for everything newer. */
	seq: number;
	atMs: number;
	/** The whole command, `git` first, with any URL credentials already removed. */
	argv: string[];
	outcome: CommandOutcome;
	/** Wall time in milliseconds. Zero for a command that was only started. */
	durationMs: number;
}

export const CLONE_PROGRESS_EVENT = 'clone-progress';
export const CLONE_DONE_EVENT = 'clone-done';
export const SEARCH_ROWS_EVENT = 'search-rows';
export const SEARCH_DONE_EVENT = 'search-done';
export const GRAPH_ROWS_EVENT = 'graph-rows';
export const GRAPH_DONE_EVENT = 'graph-done';
export const REPO_CHANGED_EVENT = 'repo-changed';
export const GIT_COMMAND_EVENT = 'git-command';
export const REBASE_PROGRESS_EVENT = 'rebase-progress';
export const REBASE_DONE_EVENT = 'rebase-done';
export const NETWORK_PROGRESS_EVENT = 'network-progress';
export const NETWORK_DONE_EVENT = 'network-done';
