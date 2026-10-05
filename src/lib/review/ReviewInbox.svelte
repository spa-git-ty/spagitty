<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { requests } from '$lib/requests/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import InboxCard from './InboxCard.svelte';
	import InboxPreview from './InboxPreview.svelte';
	import { review } from './store.svelte';

	/**
	 * The Review inbox (FEAT-087): pull requests grouped by what each needs
	 * from you, and a preview of the chosen one.
	 */
	const groups = $derived(review.groups);
	const selected = $derived(review.selected);
	let opening = $state(false);

	const HOSTS = { gitHub: 'GitHub', gitLab: 'GitLab', bitbucket: 'Bitbucket' } as const;

	/** `spagitty · GitHub · signed in as mahmoud`, as much of it as is known. */
	const signedIn = $derived.by(() => {
		const parts: string[] = [];
		const where = requests.repo;
		if (where) parts.push(where.name, HOSTS[where.kind]);
		if (review.me) parts.push(`signed in as ${review.me}`);
		return parts.join(' · ');
	});

	// The saved records behind each card's progress, read as rows arrive, and
	// on GitLab the checks and threads its list leaves out (FEAT-088).
	$effect(() => {
		void review.loadRecords(review.list);
		void review.loadSummaries();
	});

	async function open(id: string) {
		const pr = review.list.find((candidate) => candidate.id === id);
		if (!pr || opening) return;
		review.select(id);
		opening = true;
		try {
			await review.open(pr);
		} catch (e) {
			notice.failed('The review could not be opened', e);
		} finally {
			opening = false;
		}
	}
</script>

<div class="screen">
	<header class="head">
		<span class="title">Review</span>
		<Chip active={review.scope === 'repo'} onclick={() => review.setScope('repo')}>This repo</Chip>
		<Chip active={review.scope === 'all'} onclick={() => review.setScope('all')}>All my repos</Chip>
		<span class="grow"></span>
		{#if review.loading}<span class="note">Reading…</span>{/if}
		{#if signedIn}<span class="note">{signedIn}</span>{/if}
		<Btn disabled={review.loading} onclick={() => review.refresh()}>Refresh</Btn>
	</header>

	<div class="body">
		<div class="lists">
			{#if review.error}
				<div class="empty">
					<p class="note error">{review.error}</p>
					<Btn onclick={() => goto('/settings#accounts')}>Settings → Accounts</Btn>
				</div>
			{:else if review.scope === 'repo' && requests.repo === null && !review.loading}
				<p class="note">This repository is not on a service Spagitty can read.</p>
			{:else if review.scope === 'repo' && !requests.connected && !review.loading}
				<div class="empty">
					<p class="note">No account is connected.</p>
					<Btn primary onclick={() => goto('/settings#accounts')}>Settings → Accounts</Btn>
				</div>
			{:else if groups.length === 0 && !review.loading}
				<p class="note">Nothing to review.</p>
			{:else}
				{#each groups as group (group.id)}
					<section class="group">
						<h2 class="note heading">{group.title} <span class="hint">· {group.hint}</span></h2>
						{#each group.items as pr (pr.id)}
							<InboxCard
								{pr}
								record={review.recordOf(pr)}
								group={group.id}
								selected={selected?.id === pr.id}
								showRepository={review.scope === 'all'}
								onselect={() => review.select(pr.id)}
								onopen={() => open(pr.id)}
							/>
						{/each}
					</section>
				{/each}
			{/if}
		</div>

		{#if selected}
			<Splitter panel="reviewPreview" label="Resize the preview" />
			<InboxPreview
				pr={selected}
				record={review.recordOf(selected)}
				notHere={review.notHere}
				opening={opening || review.makingWorktree !== null}
				onopen={() => open(selected.id)}
				onworktree={review.isHere(selected) ? () => review.openWorktree(selected) : undefined}
			/>
		{/if}
	</div>
</div>

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
		gap: 8px;
		padding: 14px 18px 10px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
	}

	.title {
		font-size: var(--fs-title);
		margin-right: 4px;
	}

	.grow {
		flex: 1;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 10px 10px;
	}

	.lists {
		flex: 1;
		min-width: 0;
		overflow: auto;
		padding: 4px 8px 20px;
		display: flex;
		flex-direction: column;
		gap: 22px;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.heading {
		margin: 0;
		padding: 0 4px;
		font-weight: inherit;
	}

	.hint {
		opacity: 0.7;
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		max-width: 520px;
	}

	.error {
		color: var(--danger);
	}
</style>
