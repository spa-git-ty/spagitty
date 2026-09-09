// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Typed wrappers around the Tauri commands.
 *
 * This is the only module that calls `invoke`. Everything else talks in the
 * types from `./types`, so a command rename is a one-file change.
 */

import { invoke } from '@tauri-apps/api/core';
import type { DesktopTheme } from './omarchy';
import type {
	About,
	AvatarAnswer,
	BinaryDiff,
	Blame,
	CommitDetail,
	BranchRow,
	ClonePlan,
	CommitDiff,
	ConflictRegion,
	ConflictSideName,
	ConflictSides,
	ConflictState,
	DiffSide,
	DraftComment,
	ExternalToolInfo,
	ExternalToolsConfig,
	ExecutedCommand,
	FileDiff,
	IdentityProfile,
	Identity,
	FileHistoryEntry,
	IdentityKey,
	IdentityScope,
	Licenses,
	OpenResult,
	MergeMethod,
	Integration,
	PullMode,
	PullRequest,
	PullRequestComment,
	PullRequestCommit,
	RebaseEdit,
	RebasePreview,
	RebaseProgress,
	RebaseTodo,
	Reflog,
	Tag,
	Remote,
	ResetMode,
	StashAction,
	RepoSummary,
	SearchQuery,
	ForgeAccount,
	ForgeKind,
	ForgeRepo,
	ReviewVerdict,
	Settings,
	Signing,
	Update,
	Snapshot,
	Submodule,
	StashEntry,
	Worktree,
	WorkingCopy
} from './types';

export function openRepo(path: string): Promise<OpenResult> {
	return invoke('open_repo', { path });
}

export function closeRepo(): Promise<void> {
	return invoke('close_repo');
}

/** Ask the walker for more rows. Resolves immediately; rows arrive as events. */
export function graphRequest(token: number, count: number): Promise<void> {
	return invoke('graph_request', { token, count });
}

/** Restart the walk after refs moved. Resolves to the new token. */
export function graphRestart(): Promise<number> {
	return invoke('graph_restart');
}

/**
 * Choose which refs the graph is rooted at, and restart the walk.
 *
 * An empty list is every branch. Hide, Solo and Smart Branch Visibility are the
 * same call with different lists — the backend is told what to show, not which
 * control was pressed. Resolves to the new token.
 */
export function graphVisibility(refs: string[], pinned: string[] = []): Promise<number> {
	return invoke('graph_visibility', { refs, pinned });
}

export function snapshot(): Promise<Snapshot> {
	return invoke('snapshot');
}

export function commitDetail(id: string): Promise<CommitDetail> {
	return invoke('commit_detail', { id });
}

/** The Diff screen's file list and header counts. One call per commit. */
export function commitDiff(id: string): Promise<CommitDiff> {
	return invoke('commit_diff', { id });
}

/** One file's hunks, fetched as that file is opened. */
export function fileDiff(id: string, path: string): Promise<FileDiff> {
	return invoke('file_diff', { id, path });
}

/** Detailed binary / image diff metadata for a commit file (FEAT-065). */
export function binaryFileDiff(id: string, path: string): Promise<BinaryDiff> {
	return invoke('binary_file_diff', { id, path });
}

/** The staged, unstaged and conflicted lists. One call per refresh. */
export function workingCopy(): Promise<WorkingCopy> {
	return invoke('working_copy');
}

/** One working-copy file's hunks, on either side of the index. */
export function workingDiff(path: string, side: DiffSide): Promise<FileDiff> {
	return invoke('working_diff', { path, side });
}

/** Detailed binary / image diff metadata for working copy (FEAT-065). */
export function binaryWorkingDiff(path: string, side: DiffSide): Promise<BinaryDiff> {
	return invoke('binary_working_diff', { path, side });
}

export function stage(paths: string[]): Promise<void> {
	return invoke('stage', { paths });
}

export function unstage(paths: string[]): Promise<void> {
	return invoke('unstage', { paths });
}

/**
 * Stage one hunk. `header` identifies which one, so a view that has gone stale
 * is refused rather than half-applied.
 */
export function stageHunk(path: string, index: number, header: string): Promise<void> {
	return invoke('stage_hunk', { path, index, header });
}

export function unstageHunk(path: string, index: number, header: string): Promise<void> {
	return invoke('unstage_hunk', { path, index, header });
}

/**
 * Throw away unstaged changes to whole paths.
 *
 * The only call in this file that destroys work git cannot get back. The
 * confirmation is the caller's job, and every caller has one.
 */
export function discard(paths: string[]): Promise<void> {
	return invoke('discard', { paths });
}

/** Throw away one unstaged hunk. Refused if the view it came from is stale. */
export function discardHunk(path: string, index: number, header: string): Promise<void> {
	return invoke('discard_hunk', { path, index, header });
}

/** Commit what is staged. Resolves to the new commit's id. */
export function commit(subject: string, body: string, amend: boolean): Promise<string> {
	return invoke('commit', { subject, body, amend });
}

/** The message of the commit HEAD points at, for pre-filling an amend. */
export function headMessage(): Promise<string> {
	return invoke('head_message');
}

/** Every branch, with how far it has drifted. One call per refresh. */
export function branches(): Promise<BranchRow[]> {
	return invoke('branches');
}

export function checkout(name: string): Promise<void> {
	return invoke('checkout', { name });
}

/** Create a branch. An empty `start` means HEAD. */
export function createBranch(name: string, start: string, checkout: boolean): Promise<void> {
	return invoke('create_branch', { name, start, checkout });
}

/** Every stash entry, newest first. */
export function stashes(): Promise<StashEntry[]> {
	return invoke('stashes');
}

/** Stash the working copy. Refused when there is nothing to stash. */
export function stashPush(message: string, includeUntracked: boolean): Promise<void> {
	return invoke('stash_push', { message, includeUntracked });
}

/**
 * The todo list `git rebase -i <upstream>` would open, before any edit.
 *
 * Generated rather than read: opening the real one would start a rebase.
 */
export function rebaseTodo(upstream: string): Promise<RebaseTodo> {
	return invoke('rebase_todo', { upstream });
}

/**
 * What a plan would produce.
 *
 * The todo itself stays in Rust — only the edits cross, since the preview is
 * recomputed on every change.
 */
export function rebasePreview(edits: RebaseEdit[]): Promise<RebasePreview> {
	return invoke('rebase_preview', { edits });
}

/**
 * Start a query. Resolves to the token its rows will carry.
 *
 * Rows arrive as `search-rows` events; the walk ends with `search-done`.
 * Starting a query cancels whichever one was running.
 */
export function searchStart(query: SearchQuery): Promise<number> {
	return invoke('search_start', { query });
}

/** Stop the running query. */
export function searchStop(): Promise<void> {
	return invoke('search_stop');
}

/** Who last touched each line. An empty revision means HEAD. */
export function blame(path: string, revision: string): Promise<Blame> {
	return invoke('blame', { path, revision });
}

/** Read commit history for a single file path (FEAT-063). */
export function fileHistory(path: string, limit = 100): Promise<FileHistoryEntry[]> {
	return invoke('file_history', { path, limit });
}

/** What is in progress and what is conflicted. Reads only; nothing is written. */
export function conflicts(): Promise<ConflictState> {
	return invoke('conflicts');
}

/** The three index stages of one conflicted path, plus the file on disk. */
export function conflictSides(path: string): Promise<ConflictSides> {
	return invoke('conflict_sides', { path });
}

/** Every conflict region in a file's merged text. */
export function conflictRegions(text: string): Promise<ConflictRegion[]> {
	return invoke('conflict_regions', { text });
}

/**
 * Take one whole side of a conflicted file into the working tree.
 *
 * The file stays conflicted afterwards: marking it resolved is a separate call,
 * because looking at the result is the point of the screen.
 */
export function conflictTake(path: string, side: ConflictSideName): Promise<void> {
	return invoke('conflict_take', { path, side });
}

/**
 * Resolve one marker region, or every region when `index` is null.
 *
 * The file is re-read on the other side rather than sent from here, so a stale
 * screen cannot resolve a region that has moved.
 */
export function conflictResolveRegion(
	path: string,
	index: number | null,
	side: ConflictSideName
): Promise<void> {
	return invoke('conflict_resolve_region', { path, index, side });
}

/** Write the merged pane's text to the file, exactly as given. */
export function conflictWrite(path: string, text: string): Promise<void> {
	return invoke('conflict_write', { path, text });
}

/** Mark paths resolved: `git add`. */
export function conflictResolve(paths: string[]): Promise<void> {
	return invoke('conflict_resolve', { paths });
}

/** Carry on with whatever the repository is in the middle of. */
export function conflictContinue(): Promise<void> {
	return invoke('conflict_continue');
}

/** Abandon it and put the repository back. */
export function conflictAbort(): Promise<void> {
	return invoke('conflict_abort');
}

/** Every tag, newest first (FEAT-051). */
export function tags(): Promise<Tag[]> {
	return invoke('tags');
}

/**
 * Create a tag. A non-empty message makes it annotated.
 *
 * An empty `target` means HEAD, which is what `git tag` itself does.
 */
export function tagCreate(name: string, target: string, message: string): Promise<void> {
	return invoke('tag_create', { name, target, message });
}

/** Delete a local tag. */
export function tagDelete(name: string): Promise<void> {
	return invoke('tag_delete', { name });
}

/**
 * Rewrite an annotated tag's message, keeping it on the same commit.
 *
 * A tag object is immutable, so this deletes and recreates it — the date and
 * tagger become today's.
 */
export function tagRetag(name: string, target: string, message: string): Promise<void> {
	return invoke('tag_retag', { name, target, message });
}

/**
 * Where a ref has been (FEAT-050).
 *
 * An empty `reference` means `HEAD`, whose log records checkouts as well as
 * every move of whatever was checked out.
 */
export function reflog(reference: string, limit: number): Promise<Reflog> {
	return invoke('reflog', { query: { reference, limit } });
}

/** Every ref whose reflog is worth offering, `HEAD` first. */
export function reflogRefs(): Promise<string[]> {
	return invoke('reflog_refs');
}

/** Every configured remote, in name order. */
export function remotes(): Promise<Remote[]> {
	return invoke('remotes');
}

/** Add a remote. Configuration only — nothing is fetched. */
export function remoteAdd(name: string, url: string): Promise<void> {
	return invoke('remote_add', { name, url });
}

/** Rename a remote, its tracking refs, and every upstream pointing at it. */
export function remoteRename(from: string, to: string): Promise<void> {
	return invoke('remote_rename', { from, to });
}

/** Remove a remote, its tracking refs, and the upstreams pointing at it. */
export function remoteRemove(name: string): Promise<void> {
	return invoke('remote_remove', { name });
}

/** Change where a remote points. */
export function remoteSetUrl(name: string, url: string): Promise<void> {
	return invoke('remote_set_url', { name, url });
}

/** Every remembered repository, as a card. Reads each where it sits. */
export function recentRepos(): Promise<RepoSummary[]> {
	return invoke('recent_repos');
}

/**
 * Where a clone would land, and what is wrong with that.
 *
 * Recomputed as the user types: every refusal is knowable without the network.
 */
export function clonePlan(url: string, parent: string): Promise<ClonePlan> {
	return invoke('clone_plan', { url, parent });
}

/**
 * Start a clone. Resolves to the token its progress will carry.
 *
 * Progress arrives as `clone-progress` events; the clone ends with
 * `clone-done`. Rejects when a clone is already running, or when the plan is
 * refused — the plan is recomputed in Rust, so a destination that filled up
 * since the last keystroke is still refused.
 */
export function cloneStart(url: string, parent: string): Promise<number> {
	return invoke('clone_start', { url, parent });
}

/**
 * Stop the running clone and let go of it — the same call for "the user pressed
 * Stop" and "it finished". Cancelling removes the destination only if the clone
 * created it.
 */
export function cloneRelease(): Promise<void> {
	return invoke('clone_release');
}

/** Remove a repository from Spagitty's list. The directory is not touched. */
export function forgetRepo(path: string): Promise<void> {
	return invoke('forget_repo', { path });
}

export function about(): Promise<About> {
	return invoke('about');
}

/** Every dependency this build is made of. Generated from the lockfiles. */
export function licenses(): Promise<Licenses> {
	return invoke('licenses');
}

/** `user.name` and `user.email`, per scope, and which one is in effect. */
export function identity(): Promise<Identity> {
	return invoke('identity');
}

/**
 * Write one identity key in one scope, resolving to the identity as it now
 * stands. A blank value unsets the key rather than storing an empty string.
 */
export function setIdentity(
	scope: IdentityScope,
	key: IdentityKey,
	value: string
): Promise<Identity> {
	return invoke('set_identity', { scope, key, value });
}

/**
 * Which repository on a hosting service the open one points at (FEAT-017).
 *
 * Null when no repository is open, when the remote is not a host Spagitty
 * reads, or when there are several remotes and no `origin`. None of those is an
 * error — plenty of repositories are not on a forge at all.
 */
export function forgeRepo(): Promise<ForgeRepo | null> {
	return invoke('forge_repo');
}

/** The connected accounts. Hosts and logins; never tokens. */
export function forgeAccounts(): Promise<ForgeAccount[]> {
	return invoke('forge_accounts');
}

/**
 * Connect an account by proving the token works.
 *
 * The token goes straight to the OS keychain and is not returned. The login
 * comes back from the host rather than being typed, so it cannot be wrong.
 */
export function forgeConnect(
	kind: ForgeKind,
	host: string,
	token: string
): Promise<ForgeAccount[]> {
	return invoke('forge_connect', { kind, host, token });
}

/** Disconnect an account and forget its token. */
export function forgeDisconnect(host: string, user: string): Promise<ForgeAccount[]> {
	return invoke('forge_disconnect', { host, user });
}

/** The open pull requests for the open repository. */
export function pullRequests(): Promise<PullRequest[]> {
	return invoke('pull_requests');
}

/** Create a new pull request on the hosting forge (FEAT-070). */
export function createPullRequest(
	title: string,
	body: string,
	head: string,
	base: string,
	draft = false
): Promise<PullRequest> {
	return invoke('create_pull_request', { title, body, head, base, draft });
}

/**
 * The files one pull request changes, with the host's own diff of each
 * (FEAT-058).
 *
 * The diff comes from the host rather than from the repository: the head
 * branch of a pull request is usually not fetched, and fetching it to render a
 * file list would turn reading a review into a network operation on the
 * repository.
 */
export function pullRequestFiles(number: number): Promise<FileDiff[]> {
	return invoke('pull_request_files', { number });
}

/** The commits belonging to a pull request (FEAT-059). */
export function pullRequestCommits(number: number): Promise<PullRequestCommit[]> {
	return invoke('pull_request_commits', { number });
}

/** Files changed in one specific commit (FEAT-059). */
export function commitFiles(sha: string): Promise<FileDiff[]> {
	return invoke('commit_files', { sha });
}

/** Inline review comments for a pull request (FEAT-059). */
export function pullRequestComments(number: number): Promise<PullRequestComment[]> {
	return invoke('pull_request_comments', { number });
}

/**
 * Leave a review on a pull request with optional draft comments (FEAT-058 / FEAT-059).
 *
 * Outward-facing and not undoable from here: a submitted review is visible to
 * everybody watching the pull request as soon as it lands. The screen confirms
 * before calling this.
 */
export function submitReview(
	number: number,
	verdict: ReviewVerdict,
	comment: string,
	draftComments?: DraftComment[]
): Promise<void> {
	return invoke('submit_review', {
		number,
		verdict,
		comment,
		draftComments: draftComments && draftComments.length > 0 ? draftComments : undefined
	});
}

/** Reply to an existing inline review comment thread (FEAT-059). */
export function replyComment(
	number: number,
	commentId: number,
	body: string
): Promise<PullRequestComment> {
	return invoke('reply_comment', { number, commentId, body });
}

/** Merge a pull request on the configured forge (FEAT-071). */
export function mergePullRequest(
	number: number,
	method: MergeMethod,
	commitTitle?: string,
	commitMessage?: string
): Promise<void> {
	return invoke('merge_pull_request', { number, method, commitTitle, commitMessage });
}

/** Close / reject a pull request without merging (FEAT-071). */
export function closePullRequest(number: number): Promise<void> {
	return invoke('close_pull_request', { number });
}

/**
 * Toggle draft / ready-for-review status on a pull request (FEAT-071).
 *
 * Both an `id` and a `number`, because the hosts disagree about which one
 * addresses a pull request: GitHub converts draft state over GraphQL and needs
 * the node id, GitLab rewrites the title on the numbered merge request.
 */
export function setPrDraft(
	number: number,
	id: string,
	title: string,
	draft: boolean
): Promise<void> {
	return invoke('set_pr_draft', { number, id, title, draft });
}

/**
 * Commit signing as git would resolve it: `commit.gpgsign` and everything that
 * decides whether it can work (FEAT-019).
 */
export function signing(): Promise<Signing> {
	return invoke('signing');
}

/**
 * Turn signing on or off in one scope, resolving to signing as it now stands.
 *
 * Written in both directions rather than unset when off: `commit.gpgsign =
 * false` is a deliberate "not in this repository", and unsetting would let a
 * global `true` through and spring the switch back on.
 */
export function setSigning(scope: IdentityScope, on: boolean): Promise<Signing> {
	return invoke('set_signing', { scope, on });
}

/** Clear `commit.gpgsign` in one scope, so the next one up decides again. */
export function clearSigning(scope: IdentityScope): Promise<Signing> {
	return invoke('clear_signing', { scope });
}

/**
 * Is there a newer Spagitty than this one?
 *
 * One unauthenticated request to the project's own releases endpoint. Whether
 * to ask at startup is `checkForUpdates`; pressing the button in Settings asks
 * regardless, because that is somebody asking.
 */
export function checkUpdate(): Promise<Update> {
	return invoke('check_update');
}

/** Spagitty's own behaviour toggles. */
export function settings(): Promise<Settings> {
	return invoke('settings');
}

/**
 * One author's picture and handle, for a node on the graph (FEAT-079).
 *
 * Per address, because that is what is cached: a screen of three hundred rows
 * is a handful of distinct authors, and the second look at a repository makes
 * no requests at all.
 *
 * Never rejects for an ordinary absence — no picture, no network, the
 * preference turned off — because none of those is an error and all of them
 * mean the same thing to the caller: draw the generated face.
 */
export function avatar(email: string): Promise<AvatarAnswer> {
	return invoke('avatar', { email });
}

/** Every linked worktree for the open repository (FEAT-062). */
export function worktrees(): Promise<Worktree[]> {
	return invoke('worktrees');
}

/** Add a new linked worktree (FEAT-062). */
export function worktreeAdd(
	path: string,
	branch?: string | null,
	newBranch?: string | null,
	detach?: boolean
): Promise<Worktree> {
	return invoke('worktree_add', {
		path,
		branch: branch || undefined,
		newBranch: newBranch || undefined,
		detach: detach ?? false
	});
}

/** Remove a worktree (FEAT-062). */
export function worktreeRemove(path: string, force = false): Promise<void> {
	return invoke('worktree_remove', { path, force });
}

/** Lock a worktree against pruning (FEAT-062). */
export function worktreeLock(path: string, reason?: string | null): Promise<void> {
	return invoke('worktree_lock', { path, reason: reason || undefined });
}

/** Unlock a locked worktree (FEAT-062). */
export function worktreeUnlock(path: string): Promise<void> {
	return invoke('worktree_unlock', { path });
}

/** Prune stale worktrees (FEAT-062). */
export function worktreePrune(): Promise<void> {
	return invoke('worktree_prune');
}

/** List all submodules for the open repository (FEAT-067). */
export function submodules(): Promise<Submodule[]> {
	return invoke('submodules');
}

/** Update submodules recursively (FEAT-067). */
export function submoduleUpdate(
	paths: string[] = [],
	init = true,
	recursive = true
): Promise<string> {
	return invoke('submodule_update', { paths, init, recursive });
}

/** Sync submodule URLs from .gitmodules (FEAT-067). */
export function submoduleSync(recursive = true): Promise<string> {
	return invoke('submodule_sync', { recursive });
}

/** De-initialize a submodule (FEAT-067). */
export function submoduleDeinit(path: string, force = false): Promise<string> {
	return invoke('submodule_deinit', { path, force });
}

/** Read configured external diff/merge tools (FEAT-068). */
export function externalToolsConfig(): Promise<ExternalToolsConfig> {
	return invoke('external_tools_config');
}

/** Set configured external tool (FEAT-068). */
export function setExternalTool(
	toolType: 'diff' | 'merge',
	toolName: string | null,
	global = false
): Promise<void> {
	return invoke('set_external_tool', { toolType, toolName, global });
}

/** Launch external diff tool (FEAT-068). */
export function launchExternalDiff(
	path: string,
	tool?: string | null,
	commit?: string | null
): Promise<void> {
	return invoke('launch_external_diff', {
		path,
		tool: tool || undefined,
		commit: commit || undefined
	});
}

/** Launch external merge tool (FEAT-068). */
export function launchExternalMerge(path: string, tool?: string | null): Promise<void> {
	return invoke('launch_external_merge', {
		path,
		tool: tool || undefined
	});
}

/** Read all saved identity profiles (FEAT-069). */
export function identityProfiles(): Promise<IdentityProfile[]> {
	return invoke('identity_profiles');
}

/** Save an identity profile (FEAT-069). */
export function saveIdentityProfile(profile: IdentityProfile): Promise<void> {
	return invoke('save_identity_profile', { profile });
}

/** Delete an identity profile (FEAT-069). */
export function deleteIdentityProfile(id: string): Promise<void> {
	return invoke('delete_identity_profile', { id });
}

/** Apply an identity profile to current repository or globally (FEAT-069). */
export function applyIdentityProfile(profile: IdentityProfile, global = false): Promise<Identity> {
	return invoke('apply_identity_profile', { profile, global });
}

/** Store the behaviour toggles. Rejects when the write did not reach the disk. */
export function setSettings(settings: Settings): Promise<void> {
	return invoke('set_settings', { settings });
}

export function launchPath(): Promise<string | null> {
	return invoke('launch_path');
}

/** Rust's view of the shared structural constants. */
export function metrics(): Promise<{ rowPitch: number }> {
	return invoke('metrics');
}

// --- The desktop's own palette (FEAT-080) ----------------------------------
//
// Three commands and no path among them. The backend answers "what is the
// desktop painted with"; it does not take a file to read, which is the
// difference between a theme feature and a filesystem hole in the webview.

export function desktopTheme(): Promise<DesktopTheme> {
	return invoke('desktop_theme');
}

/**
 * Start being told when it changes. Returns whether a watch is actually
 * running — a machine can have Omarchy and no inotify watches left, and
 * "the theme works, live updates do not" is a thing Appearance can say.
 */
export function desktopThemeWatch(): Promise<boolean> {
	return invoke('desktop_theme_watch');
}

export function desktopThemeUnwatch(): Promise<void> {
	return invoke('desktop_theme_unwatch');
}

/** True when running inside the Tauri webview rather than a plain browser. */
export function inTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// --- History operations -----------------------------------------------------
//
// Every call below writes to the repository. None of them asks first: the
// confirmation belongs to the screen, next to the sentence describing what is
// about to happen, and a wrapper that prompted would put that sentence in the
// wrong file.

/**
 * Move the current branch to `commit`.
 *
 * `'hard'` discards uncommitted work. The mode is a union rather than a string
 * so a typo cannot become a harder reset than the one that was offered.
 */
export function reset(commit: string, mode: ResetMode): Promise<void> {
	return invoke('reset', { commit, mode });
}

/** Commit the inverse of `commit`. A merge is reverted against its first parent. */
export function revert(commit: string): Promise<void> {
	return invoke('revert', { commit });
}

/** Replay `commits` onto the current branch, in the order given (oldest first). */
export function cherryPick(commits: string[]): Promise<void> {
	return invoke('cherry_pick', { commits });
}

/** Merge, fast-forward or rebase `source` into the branch that is checked out. */
export function integrate(source: string, how: Integration): Promise<void> {
	return invoke('integrate', { source, how });
}

/**
 * Replay commits onto `onto`.
 *
 * `upstream` empty moves the whole branch. A commit in `upstream` moves only
 * what comes after it — the graph's "rebase these N commits onto". `branch`
 * empty means HEAD.
 */
export function rebaseOnto(onto: string, upstream = '', branch = ''): Promise<void> {
	return invoke('rebase_onto', { onto, upstream, branch });
}

/** Run the plan the Rebase screen built against the todo it is holding. */
/**
 * Execute the plan. Resolves to the token its events carry, not to the outcome.
 *
 * The rebase runs on a worker: progress arrives as `rebase-progress` and it
 * ends with `rebase-done`, which says whether git finished or stopped part-way
 * waiting for a conflict to be resolved.
 */
export function rebaseRun(edits: RebaseEdit[]): Promise<number> {
	return invoke('rebase_run', { edits });
}

/** How far a rebase that is running has got, or null when none is. */
export function rebaseProgress(): Promise<RebaseProgress | null> {
	return invoke('rebase_progress');
}

/** Carry on with a rebase that stopped, once its conflicts are resolved. */
export function rebaseContinue(): Promise<void> {
	return invoke('rebase_continue');
}

/** Drop the commit a rebase stopped on and carry on with the rest. */
export function rebaseSkip(): Promise<void> {
	return invoke('rebase_skip');
}

/** Unwind a rebase and put the branch back where it started. */
export function rebaseAbort(): Promise<void> {
	return invoke('rebase_abort');
}

/** Check out a commit with no branch attached — a detached HEAD. */
export function checkoutDetached(revision: string): Promise<void> {
	return invoke('checkout_detached', { revision });
}

export function renameBranch(from: string, to: string): Promise<void> {
	return invoke('rename_branch', { from, to });
}

/** Delete a local branch. `force` loses commits that are not merged anywhere. */
export function deleteBranch(name: string, force = false): Promise<void> {
	return invoke('delete_branch', { name, force });
}

/** Create a tag at `target`. A non-empty message makes it annotated. */
export function createTag(name: string, target: string, message = ''): Promise<void> {
	return invoke('create_tag', { name, target, message });
}

export function deleteTag(name: string): Promise<void> {
	return invoke('delete_tag', { name });
}

/** Apply, pop or drop `stash@{index}`. */
export function stashAction(index: number, action: StashAction): Promise<void> {
	return invoke('stash_action', { index, action });
}

/**
 * Fetch. Resolves to the token its events carry, not to the outcome (FEAT-018).
 *
 * An empty remote fetches all of them. `prune` deletes remote-tracking refs the
 * remote no longer has, and is the caller's choice: it used to happen on every
 * fetch whether or not anybody wanted it.
 *
 * Progress arrives as `network-progress` and it ends with `network-done`.
 */
export function fetch(remote = '', prune = false): Promise<number> {
	return invoke('fetch', { remote, prune });
}

/**
 * Pull: fetch and integrate, in one `git pull`.
 *
 * One call rather than fetch-then-merge, because git resolves which upstream the
 * current branch tracks — from `branch.<name>.remote` and `branch.<name>.merge`,
 * either of which may be configured per branch — and that resolution is exactly
 * the part not worth reimplementing.
 */
export function pull(mode: PullMode, remote = ''): Promise<string> {
	return invoke('pull', { remote, mode });
}

/**
 * Push. `force` is `--force-with-lease`, never a plain force.
 *
 * Resolves to the token its events carry, the same as `fetch`.
 */
export function push(remote = '', refspec = '', force = false): Promise<number> {
	return invoke('push', { remote, refspec, force });
}

/** Let go of a finished fetch or push, so the next one may start. */
export function networkRelease(): Promise<void> {
	return invoke('network_release');
}

/**
 * Every `git` command Spagitty has run since `since`, oldest first.
 *
 * New ones also arrive as `git-command` events; this is the catch-up read for
 * what ran before the panel was opened.
 */
export function gitCommands(since = 0): Promise<ExecutedCommand[]> {
	return invoke('git_commands', { since });
}

/** Forget the recorded commands. */
export function clearGitCommands(): Promise<void> {
	return invoke('clear_git_commands');
}
