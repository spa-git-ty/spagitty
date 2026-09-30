<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import * as api from '$lib/api';
	import PreviewPane from '$lib/rebase/PreviewPane.svelte';
	import { rebase } from '$lib/rebase/store.svelte';
	import { abortRebase, continueRebase, runRebase, skipCommit } from '$lib/rebase/actions';
	import TodoList from '$lib/rebase/TodoList.svelte';
	import { branches } from '$lib/branches/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * Plan a history rewrite, see the result, and run it.
	 *
	 * The screen has three shapes and they are deliberately not one. Planning is
	 * what it was before FEAT-015. Running replaces the plan with a step count,
	 * because there is nothing to edit while git is replaying and a live todo
	 * list would invite the attempt. Stopped is a screen about one commit: the
	 * conflicts are elsewhere, and what belongs here is the way onwards —
	 * continue, skip, or put everything back.
	 */

	onMount(() => {
		if (!api.inTauri() || !repo.info) return;

		// A rebase left unfinished — by a previous session, or started from the
		// command line — is the screen's state on arrival, not something it
		// discovers only after running one itself.
		void rebase.refreshProgress();

		// The upstream a branch is already tracking is the usual thing to
		// replay onto, so it is filled in. Anything else would be a guess, and
		// a wrong default here plans a rebase nobody asked for.
		branches.load().then(() => {
			const current = branches.rows.find((row) => row.current);
			if (rebase.upstream === '' && current?.upstream) {
				rebase.upstream = current.upstream;
			}
		});

		return () => rebase.clear();
	});

	/** Branches worth offering as an upstream: anything that is not HEAD. */
	const candidates = $derived(branches.rows.filter((row) => !row.current));

	const branchName = $derived(repo.info?.head.branch ?? '');
	const dropped = $derived(rebase.preview?.dropped.length ?? 0);

	/**
	 * Whether Apply would do anything, and why not when it would not.
	 *
	 * A refusal from the preview is the interesting one: the plan is already
	 * known to be unrunnable, and the reason is better than "disabled".
	 */
	const applicable = $derived(
		rebase.loaded &&
			!rebase.running &&
			!rebase.stopped &&
			rebase.plan.length > 0 &&
			rebase.preview?.refusal == null
	);

	const applyTitle = $derived(
		rebase.running
			? 'A rebase is already running'
			: rebase.stopped
				? 'Finish or abort the rebase that stopped first'
				: (rebase.preview?.refusal ?? 'Run this plan — history is rewritten')
	);
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Interactive rebase</span>
			{#if rebase.todo}
				<span class="note">
					{rebase.todo.rows.length}
					{rebase.todo.rows.length === 1 ? 'commit' : 'commits'} onto
					<span class="mono">{rebase.todo.upstreamShort}</span>
				</span>
			{/if}
		</div>
		<div class="right">
			<label class="field">
				<span class="note">onto</span>
				<input
					bind:value={rebase.upstream}
					list="rebase-upstreams"
					placeholder="branch or revision"
				/>
			</label>
			<datalist id="rebase-upstreams">
				{#each candidates as row (row.fullName)}
					<option value={row.name}></option>
				{/each}
			</datalist>
			<Btn
				disabled={rebase.upstream.trim() === '' || rebase.loading || rebase.running}
				onclick={() => rebase.load()}
			>
				Plan
			</Btn>
			<Btn disabled={!rebase.edited || rebase.running} onclick={() => rebase.reset()}>Reset</Btn>
			<Btn
				primary
				disabled={!applicable}
				title={applyTitle}
				onclick={() => runRebase(branchName, rebase.plan.length, dropped)}
			>
				Apply
			</Btn>
		</div>
	</header>

	{#if rebase.running}
		<div class="running">
			<span class="note">
				{#if rebase.progress}
					Replaying commit {rebase.progress.step} of {rebase.progress.total}
					{#if rebase.progress.branch}
						on <span class="mono">{rebase.progress.branch}</span>
					{/if}
				{:else}
					Starting the rebase…
				{/if}
			</span>
			<div
				class="bar"
				role="progressbar"
				aria-valuemin="0"
				aria-valuemax={rebase.progress?.total ?? 0}
				aria-valuenow={rebase.progress?.step ?? 0}
			>
				<span
					class="fill"
					style="width: {rebase.progress
						? Math.round((rebase.progress.step / Math.max(1, rebase.progress.total)) * 100)
						: 0}%"
				></span>
			</div>
		</div>
	{:else if rebase.stopped}
		<div class="stopped">
			<div class="what">
				<span class="note">
					git stopped at commit {rebase.progress?.step} of {rebase.progress?.total}.
					Resolve what it stopped on, then continue.
				</span>
				{#if rebase.runError}
					<span class="note error">{rebase.runError}</span>
				{/if}
			</div>
			<div class="ways">
				<Btn primary disabled={rebase.busy} onclick={() => goto('/conflicts')}>
					Resolve conflicts
				</Btn>
				<Btn disabled={rebase.busy} onclick={() => continueRebase()}>Continue</Btn>
				<Chip disabled={rebase.busy} danger onclick={() => skipCommit()}>skip this commit</Chip>
				<Chip disabled={rebase.busy} danger onclick={() => abortRebase()}>abort</Chip>
			</div>
		</div>
	{/if}

	<div class="body">
		{#if rebase.running}
			<div class="empty pad">
				<p class="note">
					Nothing is editable while git is replaying. What it is doing is above; what
					it lands on, if anything, will be waiting here.
				</p>
			</div>
		{:else if rebase.error}
			<p class="note error pad">{rebase.error}</p>
		{:else if rebase.loading}
			<p class="note pad">Reading…</p>
		{:else if !rebase.loaded}
			<div class="empty pad"></div>
		{:else if rebase.todo && rebase.todo.rows.length === 0}
			<p class="note pad">
				There is nothing to rebase: this branch has no commits the upstream does not
				already have.
			</p>
		{:else}
			<div class="columns">
				<section class="column">
					<h2 class="note heading">Plan</h2>
					<div class="scroll">
						<TodoList />
					</div>
				</section>
				<section class="column">
					<h2 class="note heading">Result</h2>
					<PreviewPane />
				</section>
			</div>
		{/if}
	</div>

	<footer class="foot">
		{#if rebase.todo?.truncated}
			<span class="note error">
				Only the first {rebase.todo.rows.length} commits are shown. A rebase longer than
				that is a different operation from the one this screen is for.
			</span>
		{:else if rebase.outcome === 'ran'}
			<span class="note">The rebase finished. The old commits are in the reflog for 30 days.</span>
		{:else if rebase.outcome === 'failed' && rebase.runError}
			<span class="note error">{rebase.runError}</span>
		{/if}
	</footer>
</div>

<style>
	.running,
	.stopped {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--soft);
		background: var(--panel);
	}

	.what,
	.ways {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.what {
		flex-direction: column;
		align-items: flex-start;
		gap: 2px;
	}

	/* A bar rather than a spinner: a rebase has a known length, and the one
	   question people have while it runs is how much is left. */
	.bar {
		flex: none;
		width: 200px;
		height: 4px;
		border-radius: 2px;
		background: var(--soft);
		overflow: hidden;
	}

	.fill {
		display: block;
		height: 100%;
		background: var(--accent);
	}

	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
		flex: none;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.title {
		font-size: var(--fs-title);
	}

	.field {
		display: flex;
		align-items: baseline;
		gap: 4px;
	}

	.field input {
		width: 160px;
		border: 1px solid var(--soft);
		border-radius: var(--r-field);
		padding: 3px 6px;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: var(--fs-secondary);
	}

	.field input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		overflow: hidden;
	}

	.pad {
		padding: 12px;
		margin: 0;
	}

	.empty {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 520px;
	}

	/* Two equal columns: the plan and what it produces, side by side, because
	   the whole argument for this screen is seeing both at once. */
	.columns {
		flex: 1;
		min-width: 0;
		display: flex;
		gap: 8px;
		padding: 8px 12px;
	}

	.column {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.heading {
		flex: none;
		margin: 0 0 6px;
		font-size: var(--fs-secondary);
		font-weight: inherit;
	}

	.scroll {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}

	.foot {
		flex: none;
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-top: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}
</style>
