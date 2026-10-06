<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { untrack } from 'svelte';
	import { relativeTime } from '$lib/format';
	import DiffPane from '$lib/diff/DiffPane.svelte';
	import { CHECK_LABELS, REVIEW_LABELS, requests } from '$lib/requests/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import type { FileDiff, ReviewVerdict } from '$lib/types';

	/**
	 * The open pull request: what it is, what it changes, and what you have to
	 * say about it (FEAT-058).
	 *
	 * Before this, every action here was disabled and named FEAT-017 — there was
	 * a host but nothing that talked to it about one pull request. The files and
	 * the review are that conversation.
	 *
	 * The diff shown is **the host's**, not one computed here. A pull request's
	 * head branch is usually not fetched, and fetching it in order to list some
	 * files would turn opening a review into a network operation against the
	 * repository.
	 */
	const request = $derived(requests.open);

	// Reading is on selection, not on a timer: this spends somebody's rate
	// limit, and nobody asked for it to be spent while they are not looking.
	$effect(() => {
		const open = requests.open;
		if (open === null) return;
		untrack(() => requests.loadFiles());
	});

	/** What each file's status is called, in the words the Diff screen uses. */
	const STATUS_LABELS: Record<FileDiff['status'], string> = {
		added: 'added',
		modified: 'modified',
		deleted: 'deleted',
		renamed: 'renamed',
		untracked: 'untracked'
	};

	/** The three things a reviewer can say, in the order they escalate. */
	const VERDICTS: { value: ReviewVerdict; label: string; hint: string }[] = [
		{ value: 'comment', label: 'Comment', hint: 'Leave a note without a verdict' },
		{ value: 'approve', label: 'Approve', hint: 'Say it is good to merge' },
		{
			value: 'requestChanges',
			label: 'Request changes',
			hint: 'Say what has to change before it merges'
		}
	];

	let verdict = $state<ReviewVerdict>('comment');
	let comment = $state('');
	let reviewOpen = $state(false);

	/** Approving is the one verdict a host takes without a comment. */
	const needsComment = $derived(verdict !== 'approve');
	const canSubmit = $derived(
		!requests.reviewing && (!needsComment || comment.trim().length > 0)
	);

	/**
	 * Submit, once the reader has confirmed it.
	 *
	 * A review is outward-facing and cannot be taken back from here: everybody
	 * watching the pull request sees it the moment it lands. That is what the
	 * confirmation is for, and it names the verdict rather than asking "are you
	 * sure" about an action the reader would have to remember.
	 */
	async function submit() {
		const open = request;
		if (!open) return;

		const said = VERDICTS.find((entry) => entry.value === verdict)?.label ?? verdict;
		const agreed = await dialog.confirm({
			title: `${said} #${open.number}?`,
			body: `This is posted to ${open.authorName} and everybody watching the pull request, and Spagitty cannot take it back.`,
			confirmLabel: said,
			danger: verdict === 'requestChanges'
		});
		if (!agreed) return;

		if (await requests.review(verdict, comment)) {
			notice.ok(`${said} sent for #${open.number}.`);
			comment = '';
			reviewOpen = false;
		}
	}
</script>

<section class="detail">
	{#if !request}
		<p class="note state">Open a pull request to read it.</p>
	{:else}
		<header class="top">
			<span class="number mono">#{request.number}</span>
			{#if request.draft}<Chip>draft</Chip>{/if}
		</header>

		<h2 class="title">{request.title}</h2>

		<div class="who note">{request.authorName} · {relativeTime(request.updated)}</div>
		<div class="branches note mono">
			{request.sourceBranch} → {request.targetBranch}
		</div>

		<div class="chips">
			<Chip active={request.review === 'changesRequested'}>
				{REVIEW_LABELS[request.review]}
			</Chip>
			{#if request.checks}
				<Chip active={request.checks === 'failing'}>{CHECK_LABELS[request.checks]}</Chip>
			{/if}
			{#if request.mergeable === false}<Chip active>conflicts</Chip>{/if}
		</div>

		<div class="counts note">
			{request.changedFiles}
			{request.changedFiles === 1 ? 'file' : 'files'} · +{request.added} −{request.removed}
		</div>

		<!--
			The files, and whichever one is open.

			A pull request nobody can read the diff of is a notification, not a
			review — so this is the part of the panel that gets the room, and the
			panel itself resizes (FEAT-056's splitter is still there).
		-->
		<section class="files">
			<h3 class="note heading">Files</h3>

			{#if requests.filesError}
				<!-- The host's own sentence, for the same reason the list has one. -->
				<p class="note error">{requests.filesError}</p>
				<Btn onclick={() => requests.loadFiles()}>Try again</Btn>
			{:else if requests.filesLoading && requests.files.length === 0}
				<Loader label="Reading the files…" />
			{:else if requests.files.length === 0}
				<p class="note">This pull request changes no files.</p>
			{:else}
				<ul class="list">
					{#each requests.files as file (file.path)}
						<li>
							<button
								class="file"
								class:at={file.path === requests.openPath}
								title={`${file.path} — ${STATUS_LABELS[file.status]}`}
								aria-current={file.path === requests.openPath}
								onclick={() => requests.selectPath(file.path)}
							>
								<span class="path mono">{file.path}</span>
								<span class="tally note mono">
									{#if file.binary}
										binary
									{:else}
										<span class="added">+{file.added}</span>
										<span class="removed">−{file.removed}</span>
									{/if}
								</span>
							</button>
						</li>
					{/each}
				</ul>

				<div class="diff">
					<!--
						The Diff screen's own pane, not a second renderer. It takes a
						`FileDiff`, which is exactly what the host's patch was parsed
						into — so a pull request's diff and a commit's diff are read
						the same way, including the parts nobody thought about twice.

						Unified rather than split: this panel is a column, and a split
						view in a column is two columns nobody can read.
					-->
					<DiffPane
						file={requests.openFile}
						path={requests.openPath}
						error={null}
						loading={requests.filesLoading}
						view="unified"
					/>
				</div>
			{/if}
		</section>

		<footer class="actions">
			{#if reviewOpen}
				<div class="review">
					<div class="verdicts" role="radiogroup" aria-label="What to say">
						{#each VERDICTS as entry (entry.value)}
							<button
								class="verdict"
								class:at={verdict === entry.value}
								role="radio"
								aria-checked={verdict === entry.value}
								title={entry.hint}
								onclick={() => (verdict = entry.value)}
							>
								{entry.label}
							</button>
						{/each}
					</div>

					<textarea
						class="comment"
						rows="3"
						bind:value={comment}
						placeholder={needsComment
							? 'What has to change, or what you wanted to say'
							: 'Anything to add (optional)'}
						aria-label="Review comment"
					></textarea>

					{#if requests.reviewError}
						<p class="note error">{requests.reviewError}</p>
					{/if}

					<div class="row">
						<Btn primary disabled={!canSubmit} onclick={submit}>
							{requests.reviewing ? 'Sending…' : 'Submit review'}
						</Btn>
						<Btn disabled={requests.reviewing} onclick={() => (reviewOpen = false)}>
							Cancel
						</Btn>
					</div>
					{#if needsComment && comment.trim().length === 0}
						<p class="note">
							{verdict === 'requestChanges' ? 'Requesting changes' : 'A comment'} needs
							something written with it.
						</p>
					{/if}
				</div>
			{:else}
				<Btn onclick={() => (reviewOpen = true)}>
					<Icon name="check" size="1em" />
					Review
				</Btn>
				<Btn disabled title="Merging is not built yet">Merge</Btn>
			{/if}
		</footer>
	{/if}
</section>

<style>
	.detail {
		width: var(--requests-detail-w);
		flex: none;
		min-height: 0;
		overflow: auto;
		padding: 8px;
		border-left: 1px solid var(--soft);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.state {
		margin: 0;
	}

	.top {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.title {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.branches {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.chips {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}

	/* The part of the panel that grows, because it is the part being read. */
	.files {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.heading {
		margin: 6px 0 0;
		font-size: var(--fs-secondary);
		font-weight: inherit;
	}

	.list {
		margin: 0;
		padding: 0;
		list-style: none;
		/* Long enough to see the shape of the change, short enough that the diff
		   underneath is still on screen without scrolling to it. */
		max-height: 30vh;
		overflow: auto;
		flex: none;
	}

	.file {
		width: 100%;
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
		padding: 3px 6px;
		border-radius: var(--r-row);
		text-align: left;
		font-size: var(--fs-secondary);
		transition: background var(--t-fast) var(--ease);
	}

	.file:hover {
		background: var(--hover);
	}

	.file.at {
		background: var(--selection);
	}

	.path {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		/* The end of a path is the part that identifies it. */
		direction: rtl;
		text-align: left;
	}

	.tally {
		flex: none;
		display: flex;
		gap: 4px;
	}

	.added {
		color: var(--ok);
	}

	.removed {
		color: var(--danger);
	}

	.diff {
		flex: 1;
		min-height: 0;
		display: flex;
		overflow: hidden;
		border-top: 1px solid var(--soft);
	}

	.actions {
		display: flex;
		gap: 6px;
		margin-top: auto;
		padding-top: 6px;
	}

	.review {
		display: flex;
		flex-direction: column;
		gap: 6px;
		width: 100%;
	}

	.verdicts {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}

	.verdict {
		padding: 3px 8px;
		border-radius: var(--r-button);
		border: 1px solid var(--soft);
		font-size: var(--fs-secondary);
		transition:
			background var(--t-fast) var(--ease),
			border-color var(--t-fast) var(--ease);
	}

	.verdict:hover {
		background: var(--hover);
	}

	.verdict.at {
		background: var(--accent-soft);
		border-color: var(--accent);
	}

	.comment {
		width: 100%;
		resize: vertical;
		font: inherit;
		font-size: var(--fs-secondary);
	}

	.row {
		display: flex;
		gap: 6px;
	}
</style>
