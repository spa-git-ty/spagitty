<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { goto } from '$app/navigation';
	import { clockTime, fullDate } from '$lib/format';
	import { cherryPick, revertCommit } from '$lib/graph/actions';
	import { avatars } from '$lib/graph/avatars.svelte';
	import AuthorAvatar from '$lib/graph/AuthorAvatar.svelte';
	import { graph } from '$lib/graph/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import FileName from '$lib/ui/FileName.svelte';
	import type { ChangedFile } from '$lib/types';

	interface Props {
		onopen?: (id: string) => void;
	}

	let { onopen }: Props = $props();

	let grouping = $state<'path' | 'tree'>('path');
	let copied = $state(false);

	const detail = $derived(graph.detail);
	const selected = $derived(graph.selected);
	const currentBranch = $derived(repo.info?.head.branch ?? null);

	/** Files grouped by directory, for the `tree` toggle. */
	const grouped = $derived.by(() => {
		const files = detail?.files ?? [];
		const map = new Map<string, ChangedFile[]>();
		for (const file of files) {
			const slash = file.path.lastIndexOf('/');
			const dir = slash === -1 ? '' : file.path.slice(0, slash);
			const list = map.get(dir);
			if (list) list.push(file);
			else map.set(dir, [file]);
		}
		return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
	});

	function basename(path: string): string {
		const slash = path.lastIndexOf('/');
		return slash === -1 ? path : path.slice(slash + 1);
	}

	async function copySha() {
		if (!detail) return;
		try {
			await navigator.clipboard.writeText(detail.id);
			copied = true;
			setTimeout(() => (copied = false), 1200);
		} catch {
			// Clipboard denied. Nothing useful to say beyond not claiming success.
		}
	}

	$effect(() => {
		const current = detail;
		if (!current) return;
		// The commit goes with the author only: it is the author the forge
		// associates with it, and the committer may be somebody else (FEAT-081).
		avatars.lookup(current.authorEmail ?? '', current.authorName, current.id);
		avatars.lookup(current.committerEmail ?? '', current.committerName);
	});

	const filesLabel = $derived(
		detail === null
			? ''
			: detail.files.length === 1
				? '1 file changed'
				: `${detail.files.length} files changed`
	);
</script>

<aside class="detail">
	{#if selected === null}
		<div class="empty note">Select a commit to see it here.</div>
	{:else}
		<header class="head">
			<span class="note">commit</span>
			<button class="sha mono" onclick={copySha} title="Copy the full SHA">
				{selected.short}
				<span aria-hidden="true">{copied ? '✔' : '⇩'}</span>
			</button>
		</header>

		{#if graph.detailError}
			<div class="pad error note">{graph.detailError}</div>
		{:else if detail === null}
			<Loader label="Loading…" />
		{:else}
			<div class="pad column">
				<div class="message">
					<div class="summary">{detail.summary}</div>
					{#if detail.body}<div class="body note">{detail.body}</div>{/if}
				</div>

				<div class="person">
					<AuthorAvatar email={detail.authorEmail} name={detail.authorName} letters={selected.initials} />
					<div class="who">
						<div class="name" title={detail.authorEmail}>{detail.authorName}</div>
						<div class="mono muted" title={fullDate(detail.authorTime)}>
							authored · {clockTime(detail.authorTime)}
						</div>
					</div>
				</div>

				<div class="person">
					<AuthorAvatar email={detail.committerEmail} name={detail.committerName} />
					<div class="who">
						<div class="name" title={detail.committerEmail}>{detail.committerName}</div>
						<div class="mono muted" title={fullDate(detail.commitTime)}>
							committed · {clockTime(detail.commitTime)}
						</div>
					</div>
				</div>

				<!--
					FEAT-019. Said in full here because there is room for the caveat,
					where the graph row has only a letter and a tooltip.

					Only when it *is* signed. "not signed" on every commit in a
					repository nobody signs is a line of noise on every commit, and
					the absence already says it.
				-->
				{#if detail.signed}
					<div class="note" title="Read from the commit's signature header">
						Signed, not verified here.
					</div>
				{/if}

				{#each detail.parents as parent (parent)}
					<div class="mono muted">parent {parent.slice(0, 7)}</div>
				{/each}
				{#if detail.parents.length === 0}
					<div class="mono muted">root commit</div>
				{/if}

				<div class="hr"></div>

				<div class="files-head">
					<span class="note">{filesLabel}</span>
					<div class="toggle">
						<Chip active={grouping === 'path'} onclick={() => (grouping = 'path')}>path</Chip>
						<Chip active={grouping === 'tree'} onclick={() => (grouping = 'tree')}>tree</Chip>
					</div>
				</div>

				<div class="files">
					{#if grouping === 'path'}
						{#each detail.files as file (file.path)}
							<button class="file" onclick={() => onopen?.(detail.id)} title={file.path}>
								<FileName path={file.path} status={file.status} />
							</button>
						{/each}
					{:else}
						{#each grouped as [dir, files] (dir)}
							<div class="dir mono muted">{dir === '' ? './' : dir + '/'}</div>
							{#each files as file (file.path)}
								<button
									class="file indent"
									onclick={() => onopen?.(detail.id)}
									title={file.path}
								>
									<FileName path={basename(file.path)} status={file.status} />
								</button>
							{/each}
						{/each}
					{/if}
					{#if detail.files.length === 0}
						<span class="note">No file changes.</span>
					{/if}
				</div>

				<Btn onclick={() => onopen?.(detail.id)}>Open full diff →</Btn>

				<div class="hr"></div>

				<div class="actions">
					<span class="note">Commit actions</span>
					<div class="chips">
						<!--
							Every chip here does what it says (TASK-046). "Merge into" and
							"Revert" were chips titled "Not built yet" that did nothing,
							the dead-control pattern BUG-030 took off the toolbar. Revert is
							built, and cherry-pick is what one commit is brought across
							with; both ask before they change anything.
						-->
						<Chip
							title="Apply this commit on top of {currentBranch ?? 'the current branch'}"
							onclick={() => void cherryPick([detail.id], [detail.short])}
						>
							Cherry-pick
						</Chip>
						<Chip onclick={() => void revertCommit(detail.id, detail.short)}>Revert</Chip>
						<Chip onclick={() => goto('/rebase')}>Interactive rebase</Chip>
						<Chip onclick={copySha}>{copied ? 'Copied' : 'Copy SHA'}</Chip>
					</div>
				</div>
			</div>
		{/if}
	{/if}
</aside>

<style>
	/*
	 * An inspector card floating in the pane (FEAT-083), not a strip cut off
	 * by a rule: inset from the pane's edges, the glass edge, its own corner.
	 * The margin sits inside the width the splitter sets, so resizing still
	 * moves the card's edge with the pointer.
	 */
	.detail {
		width: var(--detail-w);
		flex: none;
		margin: 8px 8px 8px 0;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		box-shadow: var(--ornament-shadow);
		position: relative;
		z-index: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 10px 12px;
		border-bottom: 1px solid var(--soft);
		flex: none;
	}

	.sha {
		color: var(--muted);
		display: inline-flex;
		align-items: center;
		gap: 5px;
	}

	.sha:hover {
		color: var(--accent);
	}

	.empty,
	.pad {
		padding: 10px;
	}

	.error {
		color: var(--danger);
	}

	.column {
		display: flex;
		flex-direction: column;
		gap: 8px;
		overflow-y: auto;
		min-height: 0;
	}

	/* The commit message is the one thing on this pane worth reading as a
	   document, so it is a card rather than a bordered box. */
	.message {
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
		background: var(--surface);
		box-shadow: var(--sheen), var(--shadow-1);
		padding: 9px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.summary {
		overflow-wrap: anywhere;
	}

	.body {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.person {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.who {
		min-width: 0;
		flex: 1;
	}

	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.files-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
	}

	.toggle {
		display: flex;
		gap: 4px;
	}

	.files {
		display: flex;
		flex-direction: column;
		gap: 1px;
		margin: 0 -8px;
	}

	.file {
		display: flex;
		align-items: center;
		min-height: 30px;
		padding: 0 8px;
		border-radius: var(--r-button);
		text-align: left;
		width: 100%;
		min-width: 0;
		transition: background var(--t-fast) var(--ease);
	}

	.file:hover {
		background: var(--hover);
	}

	.file.indent {
		padding-left: 10px;
	}



	.dir {
		margin-top: 4px;
	}

	.actions {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding-bottom: 10px;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
</style>
