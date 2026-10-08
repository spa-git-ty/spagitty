<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { goto } from '$app/navigation';
	import { requests } from '$lib/requests/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import ScreenHead from '$lib/ui/ScreenHead.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import InboxCard from './InboxCard.svelte';
	import InboxPreview from './InboxPreview.svelte';
	import { review } from './store.svelte';
	import { untrack } from 'svelte';
	import type { PullRequest } from '$lib/types';
	import { agents } from '$lib/agents/store.svelte';
	import { isLive } from '$lib/agents/levels';
	import type { Assigned } from '$lib/agents/types';
	import * as agentWork from './agent.svelte';

	/**
	 * The Review inbox (FEAT-087): pull requests grouped by what each needs
	 * from you, and a preview of the chosen one.
	 */
	const groups = $derived(review.groups);
	const selected = $derived(review.selected);
	let opening = $state(false);

	const HOSTS = { gitHub: 'GitHub', gitLab: 'GitLab', bitbucket: 'Bitbucket' } as const;

	/** `GitHub · signed in as mahmoud`, as much of it as is known. */
	const signedIn = $derived.by(() => {
		const parts: string[] = [];
		const where = requests.repo;
		if (where) parts.push(HOSTS[where.kind]);
		if (review.me) parts.push(`signed in as ${review.me}`);
		return parts.join(' · ');
	});

	// The saved records behind each card's progress, read as rows arrive, and
	// on GitLab the checks and threads its list leaves out (FEAT-088).
	$effect(() => {
		void review.loadRecords(review.list);
		void review.loadSummaries();
	});

	/**
	 * No account to ask with — said as Pull requests says it, not as the host's
	 * refusal (BUG-055). The refusal is matched too, for a host that answers
	 * before the inbox has asked about accounts.
	 */
	const noAccount = $derived(
		review.error ? review.error.includes('no account is connected') : !review.connected
	);

	if (review.scope !== 'repo') review.setScope('repo');

	// ── Agents (2.0): nothing here draws unless one is set up. ─────────────
	const assignable = $derived(agents.usable('review').length > 0);
	let starting = $state(false);

	function live(a: ReturnType<typeof agentOf>): boolean {
		return a !== null && isLive(a);
	}

	function agentOf(pr: PullRequest) {
		const key = review.keyOf(pr);
		return key ? agents.forReview(key.owner, key.name, key.number) : null;
	}

	$effect(() => {
		const prs = requests.all;
		untrack(() => agentWork.watchHeads(prs));
	});

	async function assign(pr: PullRequest, chosen: Assigned): Promise<boolean> {
		const key = review.keyOf(pr);
		if (!key) return false;
		starting = true;
		const started = await agentWork.assign(pr, key, chosen);
		starting = false;
		return started !== null;
	}

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
	<!--
		The same head as Pull requests (BUG-056): the title and this repository.
		"All my repos" is gone from the screen, at the author's request.
	-->
	<ScreenHead
		title="Review"
		repository={requests.repo ? `${requests.repo.owner}/${requests.repo.name}` : null}
	>
		{#snippet detail()}
			{#if signedIn}<span class="note">{signedIn}</span>{/if}
		{/snippet}
		{#snippet actions()}
			{#if review.loading}<Loader size="inline" label="Reading…" />{/if}
			<Btn disabled={review.loading} onclick={() => review.refresh()}>Refresh</Btn>
		{/snippet}
	</ScreenHead>

	<div class="body">
		<div class="lists">
			{#if review.loading && groups.length === 0 && !review.error}
				<Loader label="Reading pull requests…" />
			{:else if requests.repo === null && !review.error && !review.loading}
				<EmptyState message="This repository is not on a service Spagitty can read." />
			{:else if noAccount && !review.loading}
				<EmptyState message="No account is connected.">
					{#snippet action()}
						<Btn primary onclick={() => goto('/settings#accounts')}>Settings → Accounts</Btn>
					{/snippet}
				</EmptyState>
			{:else if review.error}
				<EmptyState message={review.error} error>
					{#snippet action()}
						<Btn onclick={() => goto('/settings#accounts')}>Settings → Accounts</Btn>
					{/snippet}
				</EmptyState>
			{:else if groups.length === 0 && !review.loading}
				<EmptyState message="Nothing to review." />
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
								agent={agentOf(pr)}
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
				opening={opening || review.checkingOut !== null}
				onopen={() => open(selected.id)}
				oncheckout={review.isHere(selected) ? () => review.checkOut(selected) : undefined}
				assignable={assignable && review.isHere(selected) && !live(agentOf(selected))}
				{starting}
				onassign={(chosen) => assign(selected, chosen)}
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
</style>
