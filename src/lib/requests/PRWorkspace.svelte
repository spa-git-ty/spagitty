<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { DiffView } from '$lib/diff/store.svelte';
	import { relativeTime } from '$lib/format';
	import PRDiffPane from '$lib/requests/PRDiffPane.svelte';
	import PRMarkdown from '$lib/requests/PRMarkdown.svelte';
	import {
		CHECK_LABELS,
		REVIEW_LABELS,
		requests,
		type UserReviewRole
	} from '$lib/requests/store.svelte';
	import type { MergeMethod, ReviewVerdict } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';

	const request = $derived(requests.open);
	const files = $derived(requests.files);
	const commits = $derived(requests.commits);
	const comments = $derived(requests.comments);
	const draftComments = $derived(requests.draftComments);
	/** Not every host has a draft pull request, so it gets no toggle there (FEAT-071). */
	const canDraft = $derived(requests.canDraft);
	const openPath = $derived(requests.openPath);
	const openFile = $derived(requests.openFile);

	/** Accordion state for left pane. */
	let filesExpanded = $state(true);
	let commitsExpanded = $state(true);
	let expandedCommitShas = $state<Record<string, boolean>>({});

	/** Diff view preference: unified vs split. */
	let diffView = $state<DiffView>('unified');

	/** Role override for viewing/testing. */
	let roleOverride = $state<UserReviewRole | null>(null);
	const activeRole = $derived(roleOverride ?? requests.role);

	/** Review submission modal state. */
	let reviewModalOpen = $state(false);
	let reviewVerdict = $state<ReviewVerdict>('comment');
	let reviewSummary = $state('');

	const isAuthor = $derived(
		Boolean(requests.currentUser && request && requests.currentUser === request.authorName)
	);

	const unresolvedCount = $derived(comments.filter((c) => !c.resolved).length);
	const resolvedCount = $derived(comments.filter((c) => c.resolved).length);

	function toggleCommit(sha: string) {
		expandedCommitShas[sha] = !expandedCommitShas[sha];
		if (expandedCommitShas[sha]) {
			requests.selectCommit(sha);
		}
	}

	function handleFileClick(path: string, commitSha: string | null = null) {
		requests.selectCommit(commitSha);
		requests.selectPath(path);
	}

	async function handlePublishReview() {
		const ok = await requests.review(reviewVerdict, reviewSummary);
		if (ok) {
			reviewModalOpen = false;
			reviewSummary = '';
		}
	}

	/** Merge modal state (FEAT-071). */
	let mergeModalOpen = $state(false);
	let mergeMethod = $state<MergeMethod>('merge');
	let mergeTitle = $state('');
	let mergeMessage = $state('');

	/** Close confirmation state (FEAT-071). */
	let closeConfirmOpen = $state(false);

	async function handleMerge() {
		const title = mergeTitle.trim() || undefined;
		const message = mergeMessage.trim() || undefined;
		const ok = await requests.merge(mergeMethod, title, message);
		if (ok) {
			mergeModalOpen = false;
			mergeTitle = '';
			mergeMessage = '';
		}
	}

	async function handleClose() {
		const ok = await requests.close();
		if (ok) {
			closeConfirmOpen = false;
		}
	}

	async function handleToggleDraft() {
		await requests.toggleDraft();
	}
</script>

{#if request}
	<div class="pr-workspace" data-role={activeRole}>
		<!-- Workspace Header -->
		<header class="workspace-head">
			<div class="head-left">
				<Btn onclick={() => requests.closeWorkspace()}>
					<Icon name="chevron-left" size="0.9em" />
					<span>Back</span>
				</Btn>

				<div class="pr-title-group">
					<span class="pr-number mono">#{request.number}</span>
					<div class="pr-title-wrapper" title={request.title}>
						<span class="pr-title">{request.title}</span>
					</div>
				</div>

				<div class="head-meta">
					<span class="meta-author note">by <strong>{request.authorName}</strong></span>
					<span class="meta-time note">{relativeTime(request.updated)}</span>

					{#if request.checks}
						<Chip active={request.checks === 'failing'}>{CHECK_LABELS[request.checks]}</Chip>
					{/if}

					<Chip active={request.review === 'changesRequested'}>
						{REVIEW_LABELS[request.review]}
					</Chip>

					{#if request.draft}
						<Chip>draft</Chip>
					{/if}
				</div>
			</div>

		<div class="head-right">
			<div class="pr-actions">
				{#if canDraft}
					<Btn
						onclick={handleToggleDraft}
						disabled={requests.togglingDraft}
						title={request.draft
							? 'Mark this PR as ready for review'
							: 'Convert this PR to draft'}
					>
						{#if requests.togglingDraft}
							…
						{:else}
							{request.draft ? 'Mark Ready' : 'Draft'}
						{/if}
					</Btn>
				{/if}

			<Btn
				onclick={() => (mergeModalOpen = true)}
				disabled={requests.merging}
				title="Merge this pull request"
			>
				Merge
			</Btn>

				<Btn
					onclick={() => (closeConfirmOpen = true)}
					disabled={requests.closing}
					title="Close this pull request without merging"
				>
					Close
				</Btn>
			</div>

			<div class="role-badge-container">
				<span class="role-label note">Mode:</span>
				<button
					class="role-toggle-btn mono"
					title="Click to toggle reviewer / developer preview mode"
					onclick={() => (roleOverride = activeRole === 'reviewer' ? 'developer' : 'reviewer')}
				>
					{activeRole === 'reviewer' ? 'Reviewer View' : 'Developer View'}
				</button>
			</div>
		</div>
		</header>

		<!-- Workspace Body: Left Accordion Pane + Center Diff Pane -->
		<div class="workspace-body">
			<!-- Left Navigation Pane -->
			<aside class="left-pane">
				<!-- CHANGELOG entry -->
				<div class="changelog-entry-section">
					<button
						class="changelog-btn"
						class:selected={openPath === '__changelog__'}
						onclick={() => requests.selectPath('__changelog__')}
						title="View pull request description and changelog"
					>
						<span class="changelog-badge mono">DOC</span>
						<span class="changelog-label">CHANGELOG</span>
					</button>
				</div>

				<!-- Section 1: All Changed Files -->
				<div class="accordion-section">
					<button
						class="accordion-header"
						onclick={() => (filesExpanded = !filesExpanded)}
						aria-expanded={filesExpanded}
					>
						<span class="chevron" class:open={filesExpanded}>▸</span>
						<span class="section-title">All Changed Files</span>
						<span class="count-badge mono">({files.length})</span>
					</button>

					{#if filesExpanded}
						<ul class="file-list">
							{#if requests.filesLoading}
								<li class="empty-note note">Loading files…</li>
							{:else if files.length === 0}
								<li class="empty-note note">No files changed.</li>
							{:else}
								{#each files as f (f.path)}
									{@const selected = requests.selectedCommitSha === null && openPath === f.path}
									<li class="file-item" class:selected>
										<button
											class="file-btn"
											onclick={() => handleFileClick(f.path, null)}
											title={f.path}
										>
											<span class="status-tag status-{f.status}">
												{f.status[0].toUpperCase()}
											</span>
											<span class="file-path">{f.path}</span>
											<span class="diff-stat mono">
												{#if f.added > 0}<span class="added">+{f.added}</span>{/if}
												{#if f.removed > 0}<span class="removed">−{f.removed}</span>{/if}
											</span>
										</button>
									</li>
								{/each}
							{/if}
						</ul>
					{/if}
				</div>

				<!-- Section 2: List Of Commits -->
				<div class="accordion-section">
					<button
						class="accordion-header"
						onclick={() => (commitsExpanded = !commitsExpanded)}
						aria-expanded={commitsExpanded}
					>
						<span class="chevron" class:open={commitsExpanded}>▸</span>
						<span class="section-title">List Of Commits</span>
						<span class="count-badge mono">({commits.length})</span>
					</button>

					{#if commitsExpanded}
						<ul class="commit-list">
							{#if requests.commitsLoading}
								<li class="empty-note note">Loading commits…</li>
							{:else if commits.length === 0}
								<li class="empty-note note">No commits found.</li>
							{:else}
								{#each commits as commit (commit.sha)}
									{@const isCommitExpanded = expandedCommitShas[commit.sha] ?? false}
									{@const commitFiles = requests.commitFilesCache[commit.sha] ?? []}

									<li class="commit-item">
										<button
											class="commit-row-btn"
											onclick={() => toggleCommit(commit.sha)}
											title={`Toggle ${commit.short}: ${commit.summary}`}
										>
											<span class="chevron" class:open={isCommitExpanded}>▸</span>
											<span class="commit-sha mono">{commit.short}</span>

											<!-- Fixed width with marquee auto-scroll on hover -->
											<div class="marquee-wrapper" title={commit.summary}>
												<span class="marquee-content">{commit.summary}</span>
											</div>
										</button>

										<!-- Nested files under commit -->
										{#if isCommitExpanded}
											<ul class="nested-files">
												{#if requests.commitFilesLoading && commitFiles.length === 0}
													<li class="empty-note note">Reading commit files…</li>
												{:else if commitFiles.length === 0}
													<li class="empty-note note">No files modified in this commit.</li>
												{:else}
													{#each commitFiles as cf (cf.path)}
														{@const selected =
															requests.selectedCommitSha === commit.sha && openPath === cf.path}
														<li class="file-item nested" class:selected>
															<button
																class="file-btn"
																onclick={() => handleFileClick(cf.path, commit.sha)}
																title={cf.path}
															>
																<span class="status-tag status-{cf.status}">
																	{cf.status[0].toUpperCase()}
																</span>
																<span class="file-path">{cf.path}</span>
																<span class="diff-stat mono">
																	{#if cf.added > 0}<span class="added">+{cf.added}</span>{/if}
																	{#if cf.removed > 0}<span class="removed">−{cf.removed}</span>{/if}
																</span>
															</button>
														</li>
													{/each}
												{/if}
											</ul>
										{/if}
									</li>
								{/each}
							{/if}
						</ul>
					{/if}
				</div>
			</aside>

			<!-- Center Pane: Diff & Inline Comments or Changelog -->
			<main class="center-pane">
				{#if openPath === '__changelog__'}
					<div class="diff-header">
						<div class="diff-file-info">
							<span class="diff-path mono">CHANGELOG</span>
							<span class="diff-file-stat note">Pull Request Description</span>
						</div>
					</div>
					<PRMarkdown markdown={request.body ?? ''} />
				{:else}
					<div class="diff-header">
						<div class="diff-file-info">
							<span class="diff-path mono">{openPath ?? 'No file selected'}</span>
							{#if openFile}
								<span class="diff-file-stat mono">
									<span class="added">+{openFile.added}</span>
									<span class="removed">−{openFile.removed}</span>
								</span>
							{/if}
						</div>

						<div class="diff-controls">
							<div class="view-switch">
								<button
									class="switch-btn"
									class:active={diffView === 'unified'}
									onclick={() => (diffView = 'unified')}
								>
									Unified
								</button>
								<button
									class="switch-btn"
									class:active={diffView === 'split'}
									onclick={() => (diffView = 'split')}
								>
									Split
								</button>
							</div>
						</div>
					</div>

					<PRDiffPane
						file={openFile}
						path={openPath}
						error={requests.filesError}
						loading={requests.filesLoading}
						view={diffView}
					/>
				{/if}
			</main>
		</div>

		<!-- Bottom Footer -->
		<footer class="workspace-footer">
			{#if activeRole === 'reviewer'}
				<!-- Reviewer View Footer -->
				<div class="footer-left">
					<span class="footer-stat">
						<strong>{draftComments.length}</strong> pending review drafts
					</span>
					<span class="footer-separator">·</span>
					<span class="footer-stat">
						<strong>{comments.length}</strong> published comments
					</span>
				</div>

				<div class="footer-right">
					{#if requests.reviewError}
						<span class="review-error error note">{requests.reviewError}</span>
					{/if}
					<Btn
						primary
						disabled={requests.reviewing}
						onclick={() => (reviewModalOpen = true)}
					>
						Publish Review ({draftComments.length})
					</Btn>
				</div>
			{:else}
				<!-- Developer View Footer -->
				<div class="footer-left">
					<span class="footer-stat" class:attention={unresolvedCount > 0}>
						<strong>{unresolvedCount}</strong> unresolved requests
					</span>
					<span class="footer-separator">·</span>
					<span class="footer-stat">
						<strong>{resolvedCount}</strong> resolved
					</span>
				</div>

				<div class="footer-right">
					<span class="developer-status note">
						{unresolvedCount === 0 ? '✓ All review comments addressed' : 'Reply or resolve comments to notify reviewer'}
					</span>
				</div>
			{/if}
		</footer>

		<!-- Review Submission Modal -->
		{#if reviewModalOpen}
			<div
				class="modal-backdrop"
				onclick={() => (reviewModalOpen = false)}
				onkeydown={(e) => {
					if (e.key === 'Escape') reviewModalOpen = false;
				}}
				role="presentation"
			>
				<div
					class="review-modal"
					onclick={(e) => e.stopPropagation()}
					onkeydown={(e) => e.stopPropagation()}
					role="dialog"
					aria-modal="true"
					tabindex="-1"
				>
					<h2 class="modal-title">Publish Pull Request Review</h2>

					<div class="verdict-options">
						{#if isAuthor}
							<div class="author-review-note note">
								As the author of this pull request, you can submit comments. Approvals and change requests are reserved for reviewers.
							</div>
						{/if}

						{#if !isAuthor}
							<label class="verdict-option" class:selected={reviewVerdict === 'approve'}>
								<input
									type="radio"
									name="verdict"
									value="approve"
									checked={reviewVerdict === 'approve'}
									onchange={() => (reviewVerdict = 'approve')}
								/>
								<div class="verdict-info">
									<strong>Approve</strong>
									<span class="note">Submit feedback and approve merging.</span>
								</div>
							</label>

							<label class="verdict-option" class:selected={reviewVerdict === 'requestChanges'}>
								<input
									type="radio"
									name="verdict"
									value="requestChanges"
									checked={reviewVerdict === 'requestChanges'}
									onchange={() => (reviewVerdict = 'requestChanges')}
								/>
								<div class="verdict-info">
									<strong>Request Changes</strong>
									<span class="note">Submit feedback that must be addressed before merging.</span>
								</div>
							</label>
						{/if}

						<label class="verdict-option" class:selected={reviewVerdict === 'comment'}>
							<input
								type="radio"
								name="verdict"
								value="comment"
								checked={reviewVerdict === 'comment'}
								onchange={() => (reviewVerdict = 'comment')}
							/>
							<div class="verdict-info">
								<strong>Comment</strong>
								<span class="note">Submit general feedback without approval.</span>
							</div>
						</label>
					</div>

					<div class="modal-field">
						<label for="review-summary" class="field-label">Overall review notes (optional):</label>
						<textarea
							id="review-summary"
							class="summary-textarea"
							rows="4"
							placeholder="Leave summary remarks on this pull request…"
							bind:value={reviewSummary}
						></textarea>
					</div>

					{#if requests.reviewError}
						<div class="modal-error note error">
							{requests.reviewError}
						</div>
					{/if}

					{#if draftComments.length > 0}
						<div class="drafts-preview note">
							Includes <strong>{draftComments.length}</strong> inline line comment(s).
						</div>
					{/if}

					<div class="modal-actions">
						<Btn onclick={() => (reviewModalOpen = false)}>Cancel</Btn>
						<Btn primary disabled={requests.reviewing} onclick={handlePublishReview}>
							{requests.reviewing ? 'Submitting…' : 'Submit Review'}
						</Btn>
					</div>
				</div>
			</div>
		{/if}

	<!-- Merge Modal (FEAT-071) -->
	{#if mergeModalOpen}
		<div
			class="modal-backdrop"
			onclick={() => (mergeModalOpen = false)}
			onkeydown={(e) => {
				if (e.key === 'Escape') mergeModalOpen = false;
			}}
			role="presentation"
		>
			<div
				class="review-modal"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
				role="dialog"
				aria-modal="true"
				tabindex="-1"
			>
				<h2 class="modal-title">Merge Pull Request #{request.number}</h2>

				<div class="verdict-options">
					<label class="verdict-option" class:selected={mergeMethod === 'merge'}>
						<input
							type="radio"
							name="merge-method"
							value="merge"
							checked={mergeMethod === 'merge'}
							onchange={() => (mergeMethod = 'merge')}
						/>
						<div class="verdict-info">
							<strong>Create a merge commit</strong>
							<span class="note">All commits will be added with a merge commit.</span>
						</div>
					</label>

					<label class="verdict-option" class:selected={mergeMethod === 'squash'}>
						<input
							type="radio"
							name="merge-method"
							value="squash"
							checked={mergeMethod === 'squash'}
							onchange={() => (mergeMethod = 'squash')}
						/>
						<div class="verdict-info">
							<strong>Squash and merge</strong>
							<span class="note">Commits will be squashed into one commit.</span>
						</div>
					</label>

					<label class="verdict-option" class:selected={mergeMethod === 'rebase'}>
						<input
							type="radio"
							name="merge-method"
							value="rebase"
							checked={mergeMethod === 'rebase'}
							onchange={() => (mergeMethod = 'rebase')}
						/>
						<div class="verdict-info">
							<strong>Rebase and merge</strong>
							<span class="note">Commits will be rebased onto the base branch.</span>
						</div>
					</label>
				</div>

				{#if mergeMethod !== 'rebase'}
					<div class="modal-field">
						<label for="merge-title" class="field-label">Commit title (optional):</label>
						<input
							id="merge-title"
							class="summary-input"
							type="text"
							placeholder="Merge pull request #{request.number}"
							bind:value={mergeTitle}
						/>
					</div>

					<div class="modal-field">
						<label for="merge-message" class="field-label">Commit message (optional):</label>
						<textarea
							id="merge-message"
							class="summary-textarea"
							rows="3"
							placeholder="Add extra details…"
							bind:value={mergeMessage}
						></textarea>
					</div>
				{/if}

				{#if requests.mergeError}
					<div class="modal-error note error">
						{requests.mergeError}
					</div>
				{/if}

				<div class="modal-actions">
					<Btn onclick={() => (mergeModalOpen = false)}>Cancel</Btn>
					<Btn primary disabled={requests.merging} onclick={handleMerge}>
						{requests.merging ? 'Merging…' : 'Confirm Merge'}
					</Btn>
				</div>
			</div>
		</div>
	{/if}

	<!-- Close Confirmation Modal (FEAT-071) -->
	{#if closeConfirmOpen}
		<div
			class="modal-backdrop"
			onclick={() => (closeConfirmOpen = false)}
			onkeydown={(e) => {
				if (e.key === 'Escape') closeConfirmOpen = false;
			}}
			role="presentation"
		>
			<div
				class="review-modal"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
				role="dialog"
				aria-modal="true"
				tabindex="-1"
			>
				<h2 class="modal-title">Close Pull Request #{request.number}?</h2>
				<p class="close-warning">
					This will close the pull request without merging. The branch will not be deleted.
				</p>

				{#if requests.closeError}
					<div class="modal-error note error">
						{requests.closeError}
					</div>
				{/if}

				<div class="modal-actions">
					<Btn onclick={() => (closeConfirmOpen = false)}>Cancel</Btn>
					<Btn primary disabled={requests.closing} onclick={handleClose}>
						{requests.closing ? 'Closing…' : 'Close Pull Request'}
					</Btn>
				</div>
			</div>
		</div>
	{/if}
	</div>
{/if}

<style>
	.pr-workspace {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg);
	}

	/* Top Header */
	.workspace-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 8px 12px;
		background: var(--surface);
		border-bottom: 1px solid var(--line);
		flex: none;
		z-index: 10;
	}

	.head-left {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		flex: 1;
	}

	.pr-title-group {
		display: flex;
		align-items: baseline;
		gap: 6px;
		min-width: 0;
		max-width: 480px;
	}

	.pr-number {
		color: var(--muted);
		font-size: var(--fs-ui);
		font-weight: 600;
		flex: none;
	}

	.pr-title-wrapper {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		position: relative;
	}

	.pr-title {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: 600;
		display: inline-block;
		color: var(--ink);
		transition: transform 5s linear;
		white-space: nowrap;
	}

	.pr-title-wrapper:hover .pr-title {
		transform: translateX(min(0px, calc(320px - 100%)));
	}

	.head-meta {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: none;
	}

	.meta-author,
	.meta-time {
		white-space: nowrap;
	}

	.role-badge-container {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.role-toggle-btn {
		background: var(--panel);
		border: 1px solid var(--line);
		border-radius: var(--r-button);
		color: var(--accent);
		padding: 3px 8px;
		font-size: var(--fs-secondary);
		cursor: pointer;
	}

	.role-toggle-btn:hover {
		background: var(--hover);
	}

	/* Workspace Body */
	.workspace-body {
		flex: 1;
		min-height: 0;
		display: flex;
		min-width: 0;
	}

	/* Left Pane */
	.left-pane {
		width: 320px;
		flex: none;
		border-right: 1px solid var(--line);
		background: var(--surface);
		display: flex;
		flex-direction: column;
		overflow-y: auto;
	}

	.changelog-entry-section {
		padding: 4px 6px;
		border-bottom: 1px solid var(--line);
		background: var(--surface);
	}

	.changelog-btn {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 7px 10px;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--r-panel);
		color: var(--ink);
		cursor: pointer;
		text-align: left;
		transition: background var(--t-fast) var(--ease);
	}

	.changelog-btn:hover {
		background: var(--hover);
	}

	.changelog-btn.selected {
		background: var(--selection);
		border-color: var(--accent);
	}

	.changelog-badge {
		font-size: var(--fs-mono);
		font-weight: 700;
		padding: 2px 5px;
		background: color-mix(in srgb, var(--accent) 15%, var(--panel));
		color: var(--accent);
		border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
		border-radius: var(--r-button);
		flex: none;
	}

	.changelog-label {
		font-size: var(--fs-secondary);
		font-weight: 700;
		letter-spacing: 0.5px;
	}

	.accordion-section {
		border-bottom: 1px solid var(--line);
	}

	.accordion-header {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 10px;
		background: var(--panel);
		border: none;
		font: inherit;
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--ink);
		cursor: pointer;
		text-align: left;
	}

	.accordion-header:hover {
		background: var(--hover);
	}

	.chevron {
		display: inline-block;
		transition: transform var(--t-fast) var(--ease);
		font-size: 0.85em;
		color: var(--muted);
	}

	.chevron.open {
		transform: rotate(90deg);
	}

	.section-title {
		flex: 1;
	}

	.count-badge {
		color: var(--muted);
		font-weight: normal;
	}

	.file-list,
	.commit-list,
	.nested-files {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.empty-note {
		padding: 8px 12px;
		font-size: var(--fs-secondary);
	}

	.file-item {
		display: flex;
		border-bottom: 1px solid color-mix(in srgb, var(--line) 40%, transparent);
	}

	.file-item.nested {
		padding-left: 18px;
		background: color-mix(in srgb, var(--panel) 40%, transparent);
	}

	.file-item.selected {
		background: var(--selection);
		border-left: 2px solid var(--accent);
	}

	.file-btn {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		background: transparent;
		border: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	.status-tag {
		flex: none;
		width: 16px;
		height: 16px;
		font-size: var(--fs-mono);
		font-weight: 600;
		display: grid;
		place-items: center;
		border-radius: 2px;
		border: 1px solid var(--line);
	}

	.status-added {
		color: var(--ok);
		border-color: var(--ok);
	}

	.status-deleted {
		color: var(--danger);
		border-color: var(--danger);
	}

	.status-modified {
		color: var(--accent);
		border-color: var(--accent);
	}

	.file-path {
		flex: 1;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: var(--fs-secondary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.diff-stat {
		flex: none;
		font-size: var(--fs-mono);
		display: flex;
		gap: 4px;
	}

	.added {
		color: var(--ok);
	}

	.removed {
		color: var(--danger);
	}

	/* Commit Item & Marquee Hover */
	.commit-item {
		border-bottom: 1px solid color-mix(in srgb, var(--line) 40%, transparent);
	}

	.commit-row-btn {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 8px;
		background: var(--surface);
		border: none;
		font: inherit;
		color: inherit;
		text-align: left;
		cursor: pointer;
		transition: background var(--t-fast) var(--ease);
	}

	.commit-row-btn:hover {
		background: var(--hover);
	}

	.commit-sha {
		font-size: var(--fs-mono);
		color: var(--accent);
		background: var(--panel);
		padding: 1px 4px;
		border-radius: 2px;
		border: 1px solid var(--line);
		flex: none;
	}

	/* Fixed-width marquee container */
	.marquee-wrapper {
		flex: 1;
		width: 180px;
		max-width: 180px;
		overflow: hidden;
		white-space: nowrap;
		position: relative;
	}

	.marquee-content {
		display: inline-block;
		font-size: var(--fs-secondary);
		color: var(--ink);
		transition: transform 4s ease-in-out;
		white-space: nowrap;
	}

	.commit-row-btn:hover .marquee-content {
		transform: translateX(min(0px, calc(180px - 100%)));
	}

	/* Center Pane */
	.center-pane {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		background: var(--bg);
	}

	.diff-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 6px 12px;
		background: var(--panel);
		border-bottom: 1px solid var(--line);
		flex: none;
	}

	.diff-file-info {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.diff-path {
		font-weight: 600;
		font-size: var(--fs-secondary);
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.diff-file-stat {
		font-size: var(--fs-secondary);
		display: flex;
		gap: 4px;
	}

	.view-switch {
		display: flex;
		border: 1px solid var(--line);
		border-radius: var(--r-button);
		overflow: hidden;
	}

	.switch-btn {
		background: var(--surface);
		border: none;
		padding: 3px 8px;
		font: inherit;
		font-size: var(--fs-secondary);
		color: var(--muted);
		cursor: pointer;
	}

	.switch-btn.active {
		background: var(--selection);
		color: var(--accent);
		font-weight: 500;
	}

	/* Workspace Footer */
	.workspace-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 8px 12px;
		background: var(--surface);
		border-top: 1px solid var(--line);
		flex: none;
		font-size: var(--fs-ui);
	}

	.footer-left {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.footer-stat.attention {
		color: var(--danger);
	}

	.footer-separator {
		color: var(--muted);
	}

	.footer-right {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.review-error {
		color: var(--danger);
	}

	/* Modal Dialog */
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: color-mix(in srgb, var(--umbra) 40%, transparent);
		display: grid;
		place-items: center;
		z-index: 100;
	}

	.review-modal {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--r-panel);
		width: 480px;
		max-width: 90vw;
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		box-shadow: none;
	}

	.modal-title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.verdict-options {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.verdict-option {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		padding: 8px 10px;
		border: 1px solid var(--line);
		border-radius: var(--r-panel);
		cursor: pointer;
		background: var(--bg);
	}

	.verdict-option.selected {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 8%, var(--surface));
	}

	.verdict-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.modal-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.field-label {
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.summary-textarea {
		width: 100%;
		padding: 8px;
		font: inherit;
		font-size: var(--fs-ui);
		background: var(--bg);
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		box-sizing: border-box;
		resize: vertical;
	}

	.author-review-note {
		padding: 8px 10px;
		background: color-mix(in srgb, var(--accent) 8%, var(--bg));
		border-left: 3px solid var(--accent);
		border-radius: var(--r-panel);
		font-size: var(--fs-secondary);
	}

	.modal-error {
		padding: 8px 10px;
		background: color-mix(in srgb, var(--danger) 10%, var(--bg));
		border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
		border-radius: var(--r-panel);
		font-size: var(--fs-secondary);
		word-break: break-word;
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 4px;
	}

	.pr-actions {
		display: flex;
		gap: 6px;
		align-items: center;
	}

	.summary-input {
		width: 100%;
		padding: 6px 8px;
		font: inherit;
		font-size: var(--fs-ui);
		background: var(--bg);
		color: var(--ink);
		border: 1px solid var(--line);
		border-radius: var(--r-panel);
		outline: none;
	}

	.summary-input:focus {
		border-color: var(--accent);
	}

	.close-warning {
		margin: 0;
		font-size: var(--fs-ui);
		color: var(--muted);
		line-height: 1.5;
	}
</style>
