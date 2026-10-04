<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { CHECK_LABELS, requests } from '$lib/requests/store.svelte';
	import { review } from './store.svelte';

	/**
	 * The review room (FEAT-087): one pull request, read file by file.
	 */
	const opened = $derived(review.room);
	const pr = $derived(opened?.pr ?? null);

	const HOSTS = { gitHub: 'GitHub', gitLab: 'GitLab', bitbucket: 'Bitbucket' } as const;
</script>

{#if pr}
	<div class="screen">
		<header class="head">
			<button class="back" onclick={() => review.close()}>
				<Icon name="chevron-left" size="0.9em" weight={2} />Review
			</button>
			<span class="name">
				<span class="number mono">#{pr.number}</span>
				<span class="title">{pr.title}</span>
			</span>
			<Btn disabled={review.makingWorktree !== null} onclick={() => review.openWorktree(pr)}>
				<Icon name="folder" size="1em" />Open in worktree
			</Btn>
		</header>
		<div class="meta note">
			<span><span class="who">{pr.authorName}</span> wants to merge</span>
			<span class="tag mono">{pr.sourceBranch}</span>
			<span>into</span>
			<span class="tag mono">{pr.targetBranch}</span>
			{#if pr.checks}
				<span>·</span>
				<span class="checks {pr.checks}">{CHECK_LABELS[pr.checks]}</span>
			{/if}
			{#if requests.repo}<span>·</span><span>{HOSTS[requests.repo.kind]}</span>{/if}
		</div>
	</div>
{/if}

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
		padding: 14px 18px 4px;
	}

	.back {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 4px 12px 4px 8px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		background-color: var(--surface-veil);
		font-size: var(--fs-secondary);
	}

	.back:hover {
		background-color: var(--hover);
	}

	.name {
		flex: 1 1 380px;
		min-width: 0;
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.number {
		color: var(--muted);
		font-size: var(--fs-ui);
	}

	.title {
		font-size: var(--fs-title);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.meta {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		padding: 0 18px 12px 112px;
	}

	.who {
		color: var(--ink);
	}

	.tag {
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 2px 9px;
		background-color: var(--surface-veil);
	}

	.checks.passing {
		color: var(--ok);
	}

	.checks.failing {
		color: var(--danger);
	}
</style>
