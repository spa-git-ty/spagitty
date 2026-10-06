<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
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

	async function abort() {
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
			<span class="title">Merging</span>
			{#if who}
				<span class="branch mono tone-{who.source}">{who.sourceName}</span>
				<span class="title">into</span>
				<span class="branch mono tone-{who.isNew ? 'new' : who.onto}">{who.targetName}</span>
			{/if}
			<span class="note">· {strategy}</span>
		</div>
		{#if progress.total > 0}
			<span class="progress">
				<span class="note">{progress.resolved} of {progress.total} resolved</span>
				<span class="bar"><span class="fill" style:width="{(progress.resolved / progress.total) * 100}%"></span></span>
			</span>
		{/if}
		<Btn onclick={abort}>Abort</Btn>
		<Btn
			primary
			disabled={!resolving.ready}
			title={resolving.ready ? undefined : 'Resolve every conflict first'}
			onclick={() => merger.openCommit('resolve')}
		>
			{resolving.ready ? 'Complete merge' : `Complete merge · ${left} left`}
		</Btn>
	</header>
	<p class="note under">Nothing is written to either branch until you commit. Your choices are kept, so you can leave and come back.</p>

	{#if resolving.error}
		<p class="note error" role="alert">{resolving.error}</p>
	{:else if resolving.loading && resolving.files.length === 0}
		<p class="note pad">Reading the conflicts…</p>
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
