<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import Resolver from '$lib/resolver/Resolver.svelte';
	import { resolving } from './resolve.svelte';
	import { merger } from './store.svelte';

	/**
	 * Resolving a merge Merger planned (FEAT-102): what is merging into what,
	 * how far along, and the shared three-column resolver. Abort goes back to
	 * the plan and drops the choices; nothing is written until the commit.
	 */
	const forecast = $derived(merger.forecast);
	const who = $derived(merger.roles);
	const names = $derived({ a: forecast?.a.name ?? 'A', b: forecast?.b.name ?? 'B' });
	const progress = $derived(resolving.counts);
	const left = $derived(progress.total - progress.resolved);
	const strategy = $derived(
		merger.strategy === 'squash' ? 'squash' : merger.strategy === 'rebase' ? 'rebase' : 'merge commit'
	);

	const roles = $derived({
		a: who?.isNew ? 'starting point' : who?.onto === 'a' ? 'lands here' : 'comes in',
		b: who?.isNew ? 'comes in' : who?.onto === 'b' ? 'lands here' : 'comes in',
		result: `lands in ${who?.targetName ?? ''}`
	});

	const others = $derived(
		(forecast?.files ?? []).flatMap((file) =>
			file.touch === 'conflict' ? [] : [{ path: file.path, tone: file.touch === 'both' ? ('ok' as const) : file.touch }]
		)
	);

	const stop = $derived(resolving.stop);

	async function abort() {
		if (stop) {
			const agreed = await dialog.confirm({
				title: 'Abort this rebase',
				body: 'Back to the plan. The commits replayed so far are thrown away; neither branch was moved.',
				confirmLabel: 'Abort',
				danger: true
			});
			if (agreed) await resolving.abortRebase();
			return;
		}
		if (progress.resolved > 0) {
			const agreed = await dialog.confirm({
				title: 'Abort this merge',
				body: 'Back to the plan. The choices made here are dropped; nothing was written to either branch.',
				confirmLabel: 'Abort',
				danger: true
			});
			if (!agreed) return;
		}
		resolving.abort();
	}
</script>

<div class="resolve">
	<header class="head">
		<button class="back" onclick={() => resolving.toPlan()}>
			<Icon name="chevron-left" size="0.9em" weight={2} />Plan
		</button>
		<div class="what">
			<span class="title">{stop ? 'Rebasing' : 'Merging'}</span>
			{#if who}
				<span class="branch mono tone-{who.source}">{who.sourceName}</span>
				<span class="title">{stop ? 'onto' : 'into'}</span>
				<span class="branch mono tone-{who.isNew ? 'new' : who.onto}">{who.targetName}</span>
			{/if}
			{#if stop}
				<span class="note">· commit {stop.step} of {stop.total}</span>
				{#if stop.commit}<span class="note mono" title={stop.commit.summary}>{stop.commit.short} {stop.commit.summary}</span>{/if}
			{:else}
				<span class="note">· {strategy}</span>
			{/if}
		</div>
		{#if progress.total > 0}
			<span class="progress">
				<span class="note">{progress.resolved} of {progress.total} resolved</span>
				<span class="bar"><span class="fill" style:width="{(progress.resolved / progress.total) * 100}%"></span></span>
			</span>
		{/if}
		<Btn disabled={resolving.stepping} onclick={abort}>Abort</Btn>
		{#if stop}
			<Btn disabled={resolving.stepping} title="Drop this commit and carry on with the next" onclick={() => resolving.skipRebase()}>
				Skip this commit
			</Btn>
			<Btn
				primary
				busy={resolving.stepping}
				disabled={progress.total > 0 && !resolving.ready}
				title={progress.total > 0 && !resolving.ready ? 'Resolve every conflict first' : undefined}
				onclick={() => resolving.continueRebase()}
			>
				{resolving.stepping ? 'Replaying…' : progress.total > 0 && !resolving.ready ? `Continue · ${left} left` : 'Continue'}
			</Btn>
		{:else}
			<Btn
				primary
				disabled={!resolving.ready}
				title={resolving.ready ? undefined : 'Resolve every conflict first'}
				onclick={() => merger.openCommit('resolve')}
			>
				{resolving.ready ? 'Complete merge' : `Complete merge · ${left} left`}
			</Btn>
		{/if}
	</header>
	<p class="note under">
		{stop
			? 'The rebase runs in a worktree of its own, one commit at a time. Neither branch moves until you finish, and you can leave and come back.'
			: 'Nothing is written to either branch until you commit. Your choices are kept, so you can leave and come back.'}
	</p>

	{#if resolving.error}
		<p class="note error" role="alert">{resolving.error}</p>
	{/if}
	{#if stop && resolving.files.length === 0}
		<p class="note pad">This commit stopped without a conflict to resolve. Continue to commit it as it is, or skip it.</p>
	{:else if resolving.loading && resolving.files.length === 0}
		<Loader label="Reading the conflicts…" />
	{:else}
		<Resolver
			files={resolving.files}
			choices={resolving.choices}
			{names}
			{roles}
			baseShort={resolving.baseShort}
			{others}
			start={resolving.start}
			onchoose={(path, index, choice) => resolving.choose(path, index, choice)}
			onwhole={(path, side) => resolving.whole(path, side)}
		/>
	{/if}
</div>

<style>
	.resolve {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
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
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 4px 12px 4px 8px;
		font-size: var(--fs-secondary);
		background-color: var(--surface-veil);
	}

	.back:hover {
		background-color: var(--hover);
	}

	.what {
		flex: 1 1 420px;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.title {
		font-size: var(--fs-title);
	}

	.branch {
		border: 1px solid currentColor;
		border-radius: var(--r-pill);
		padding: 2px 10px;
		font-size: var(--fs-secondary);
	}

	.tone-a {
		color: var(--side-a);
	}

	.tone-b {
		color: var(--side-b);
	}

	.tone-new {
		color: var(--ok);
	}

	.progress {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.bar {
		width: 96px;
		height: 6px;
		border-radius: var(--r-pill);
		background: var(--soft);
		overflow: hidden;
	}

	.fill {
		display: block;
		height: 100%;
		background: var(--ok);
		border-radius: var(--r-pill);
	}

	.under {
		flex: none;
		margin: 0;
		padding: 0 18px 12px 92px;
	}

	.error {
		color: var(--danger);
		padding: 0 18px;
	}

	.pad {
		padding: 0 18px;
	}
</style>
