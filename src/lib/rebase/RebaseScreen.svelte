<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { branches } from '$lib/branches/store.svelte';
	import { relativeTime } from '$lib/format';
	import BranchPicker from '$lib/merger/BranchPicker.svelte';
	import type { Pickable } from '$lib/merger/store.svelte';
	import { abortRebase, continueRebase, runRebase, skipCommit } from './actions';
	import { counts, sentence } from './plan';
	import RebaseHistory from './RebaseHistory.svelte';
	import RebasePlan from './RebasePlan.svelte';
	import { rebase } from './store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';

	/**
	 * Rebase (1E), drawn after Merger (FEAT-106): the branch being moved on one
	 * side, the branch it lands on on the other, and between them the result —
	 * how many commits, what each edit does, which may stop on a conflict —
	 * before anything is written. Below it the plan itself, one row a commit,
	 * and the history the plan leaves.
	 *
	 * Three states, and the Result card is where each shows: planning; running,
	 * with how far git has got; and stopped, with the ways on — resolve,
	 * continue, skip, or put everything back. The plan stays visible throughout
	 * and is locked while git owns the branch.
	 */

	interface Props {
		/** The branch checked out, or null on a detached HEAD. */
		branch: string | null;
		/** HEAD's short id, for a detached HEAD's name. */
		headShort: string | null;
		/** Choose what to replay onto, and plan it. */
		onpick: (name: string) => void;
	}

	let { branch, headShort, onpick }: Props = $props();

	const branchName = $derived(branch ?? '');
	const shownBranch = $derived(branchName || (headShort ?? 'HEAD'));
	const onto = $derived(rebase.upstream.trim());

	/** Anything to replay onto: every branch and tag but the one checked out. */
	const options = $derived<Pickable[]>(
		branches.rows
			.filter((row) => !row.current)
			.map((row) => ({
				name: row.name,
				group: row.kind === 'branch' ? 'local' : row.kind === 'remote' ? 'remote' : 'tag',
				time: row.time
			}))
	);
	const ontoRow = $derived(branches.rows.find((row) => row.name === onto) ?? null);

	const todoRows = $derived(rebase.todo?.rows ?? []);
	/** Newest first, the way a branch card lists commits. */
	const newest = $derived([...todoRows].reverse());
	const tally = $derived(counts(rebase.plan, rebase.preview));
	const dropped = $derived(
		(rebase.preview?.dropped ?? [])
			.map((id) => rebase.rowFor(id))
			.filter((row) => row !== null)
			.map((row) => ({ short: row.short, summary: row.summary }))
	);
	const refusal = $derived(rebase.preview?.refusal ?? null);
	const locked = $derived(rebase.running || rebase.stopped);
	const planned = $derived(rebase.loaded && todoRows.length > 0);

	const blocked = $derived(
		rebase.running
			? 'A rebase is already running'
			: rebase.stopped
				? 'Finish or abort the rebase that stopped first'
				: branchName === ''
					? 'HEAD is not on a branch'
					: refusal
	);

	function apply() {
		void runRebase(branchName, rebase.plan.length, tally.dropped);
	}

	const status = $derived.by((): { tone: string; text: string } => {
		if (rebase.running) {
			const p = rebase.progress;
			return { tone: 'busy', text: p ? `Replaying ${p.step} of ${p.total}` : 'Starting' };
		}
		if (rebase.stopped) {
			return { tone: 'stopped', text: `Stopped at ${rebase.progress?.step} of ${rebase.progress?.total}` };
		}
		if (rebase.outcome === 'ran') return { tone: 'ok', text: 'Done · old commits kept in the reflog' };
		return { tone: 'ok', text: 'Dry run · nothing written yet' };
	});
</script>

<div class="page">
	<header class="head">
		<span class="tile" aria-hidden="true"><Icon name="rebase" size="1.25em" weight={1.9} /></span>
		<div class="titles">
			<h1 class="title">Rebase</h1>
			<span class="note">Replay a branch onto another, editing its commits on the way.</span>
		</div>
		<span class="status tone-{status.tone}"><span class="pip" aria-hidden="true"></span>{status.text}</span>
	</header>

	<div class="stage">
		<section class="card side moves" aria-label="Branch being moved">
			<div class="top">
				<span class="badge"><Icon name="branch" size="0.95em" weight={2} /></span>
				<span class="name mono" title={shownBranch}>{shownBranch}</span>
				<span class="role">Moves</span>
			</div>
			{#if planned && rebase.todo}
				<span class="note">{todoRows.length} {todoRows.length === 1 ? 'commit' : 'commits'} since {rebase.todo.upstreamShort}</span>
				<ul class="commits">
					{#each newest.slice(0, 3) as commit (commit.id)}
						<li>
							<span class="dot" aria-hidden="true"></span>
							<span class="mono muted">{commit.short}</span>
							<span class="summary" title={commit.summary}>{commit.summary}</span>
						</li>
					{/each}
				</ul>
				{#if newest.length > 3}<span class="note more">+ {newest.length - 3} more</span>{/if}
				<span class="grow"></span>
				<span class="note">last commit {relativeTime(newest[0].time)}</span>
			{:else if branchName === ''}
				<span class="note error">HEAD is detached. Check out a branch to rebase it.</span>
			{/if}
		</section>

		<div class="connector" aria-hidden="true">
			<span class="caption">{planned ? `${tally.replayed} replayed` : ''}</span>
			<span class="line moves-line">
				<span class="rule"></span>
				<svg width="10" height="12" viewBox="0 0 10 12"><path d="M0 0 10 6 0 12z"></path></svg>
			</span>
		</div>

		<section class="result" aria-label="Result" aria-busy={rebase.loading}>
			{#if rebase.running}
				<div class="result-head">
					<span class="label">Rebasing</span>
					<span class="target mono">{shownBranch}</span>
				</div>
				<p class="sentence">
					{#if rebase.progress}
						Replaying commit {rebase.progress.step} of {rebase.progress.total}{#if rebase.progress.branch}&nbsp;on <span class="mono">{rebase.progress.branch}</span>{/if}.
					{:else}
						Starting the rebase…
					{/if}
				</p>
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
			{:else if rebase.stopped}
				<div class="box danger">
					<div class="box-head">
						<Icon name="conflict" size="1em" weight={2} />
						<span class="box-title">Stopped at commit {rebase.progress?.step} of {rebase.progress?.total}</span>
					</div>
					<span class="note small">Resolve what it stopped on, then continue.</span>
					{#if rebase.runError}<span class="note small">{rebase.runError}</span>{/if}
				</div>
				<div class="actions">
					<Btn primary disabled={rebase.busy} onclick={() => goto('/conflicts')}>
						Resolve conflicts<Icon name="chevron-right" size="0.95em" weight={2.2} />
					</Btn>
					<Btn busy={rebase.busy} onclick={() => continueRebase()}>Continue</Btn>
					<Btn disabled={rebase.busy} onclick={() => skipCommit()}>Skip this commit</Btn>
					<Btn disabled={rebase.busy} onclick={() => abortRebase()}>Abort</Btn>
				</div>
			{:else if rebase.error}
				<p class="note error" role="alert">{rebase.error}</p>
			{:else if onto === ''}
				<p class="note">Choose a branch to replay onto.</p>
			{:else if rebase.loading || !rebase.loaded}
				<Loader label="Working out the rebase…" />
			{:else if todoRows.length === 0}
				<div class="result-head">
					<span class="label">Result</span>
					<span class="target mono">{shownBranch}</span>
				</div>
				<p class="sentence">Nothing to replay: {shownBranch} has no commits that {onto} lacks.</p>
			{:else}
				<div class="result-head">
					<span class="label">Result</span>
					<span class="target mono">{shownBranch}</span>
					<span class="note">after the rebase</span>
				</div>
				<p class="sentence">{sentence(shownBranch, onto, tally)}</p>
				<div class="stats">
					<div class="stat"><span class="value">{tally.after}</span><span class="note small">commits after</span></div>
					<div class="stat"><span class="value">{tally.squashed}</span><span class="note small">folded</span></div>
					<div class="stat"><span class="value">{tally.dropped}</span><span class="note small">dropped</span></div>
				</div>
				{#if refusal}
					<div class="box danger">
						<div class="box-head">
							<Icon name="conflict" size="1em" weight={2} />
							<span class="box-title">This plan cannot run</span>
						</div>
						<span class="note small">{refusal}</span>
					</div>
				{:else}
					{@const rows = rebase.preview?.rows ?? []}
					<div class="box" class:danger={tally.risky > 0}>
						<div class="box-head">
							<Icon name={tally.risky > 0 ? 'conflict' : 'check'} size="1em" weight={2} />
							<span class="box-title">
								{tally.risky > 0
									? `${tally.risky} ${tally.risky === 1 ? 'commit' : 'commits'} may stop on a conflict`
									: 'Nothing looks likely to conflict'}
							</span>
						</div>
						{#if rows.length > 0}
							<div class="meter" aria-hidden="true">
								{#each rows as row (row.id)}<span class:red={row.mayConflict}></span>{/each}
							</div>
						{/if}
						<span class="note small">Judged by the files each commit touches; git decides as it replays.</span>
					</div>
				{/if}
				<div class="actions">
					<Btn primary disabled={blocked !== null} title={blocked ?? undefined} onclick={apply}>
						Rebase now
					</Btn>
					{#if rebase.edited}
						<Btn onclick={() => rebase.reset()}>Reset plan</Btn>
					{/if}
				</div>
			{/if}
		</section>

		<div class="connector" aria-hidden="true">
			<span class="caption tone-onto">onto</span>
			<span class="line onto-line">
				<svg width="10" height="12" viewBox="0 0 10 12"><path d="M10 0 0 6 10 12z"></path></svg>
				<span class="rule"></span>
			</span>
		</div>

		<section class="card side onto" aria-label="Branch to replay onto">
			<div class="top">
				<span class="badge"><Icon name="base" size="0.95em" weight={2} /></span>
				<BranchPicker
					value={onto === '' ? null : onto}
					which="onto"
					{options}
					other={null}
					current={null}
					{onpick}
				/>
				<span class="role">Starting point</span>
			</div>
			{#if ontoRow}
				<ul class="commits">
					<li>
						<span class="dot" aria-hidden="true"></span>
						<span class="mono muted">{ontoRow.short}</span>
						<span class="summary" title={ontoRow.summary}>{ontoRow.summary}</span>
					</li>
				</ul>
				<span class="grow"></span>
				<span class="note">last commit {relativeTime(ontoRow.time)}</span>
			{:else if rebase.todo}
				<span class="note mono">{rebase.todo.upstreamShort}</span>
			{/if}
		</section>
	</div>

	{#if planned}
		<div class="lower">
			<section class="card plan-card" aria-label="The plan">
				<div class="card-head">
					<span class="label">The plan</span>
					<span class="note small">replayed top to bottom · drag or Alt+↑ / Alt+↓ to reorder</span>
				</div>
				<RebasePlan {locked} />
				{#if rebase.todo?.truncated}
					<span class="note error">
						Only the first {todoRows.length} commits are shown. A rebase longer than that is a different
						operation from the one this screen is for.
					</span>
				{/if}
			</section>

			<section class="card history-card" aria-label="History after">
				<span class="label">History after</span>
				<RebaseHistory
					rows={rebase.preview?.rows ?? []}
					{dropped}
					{onto}
					ontoShort={rebase.todo?.upstreamShort ?? ''}
					branch={shownBranch}
				/>
			</section>
		</div>
	{/if}

	{#if rebase.outcome === 'failed' && rebase.runError}
		<p class="note error">{rebase.runError}</p>
	{/if}
</div>

<style>
	.page {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 16px 20px 96px;
	}

	.head,
	.stage,
	.lower {
		flex: none;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}

	.tile {
		width: 38px;
		height: 38px;
		border-radius: 12px;
		display: grid;
		place-items: center;
		background: color-mix(in srgb, var(--accent) 16%, transparent);
		color: var(--accent);
		flex: none;
	}

	.titles {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1 1 320px;
		min-width: 0;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.status {
		--tone: var(--ok);
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--tone) 45%, transparent);
		color: var(--tone);
		font-size: var(--fs-secondary);
		background-color: var(--surface-veil);
		white-space: nowrap;
	}

	.status.tone-busy {
		--tone: var(--accent);
	}

	.status.tone-stopped {
		--tone: var(--danger);
	}

	.pip {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--tone);
	}

	.stage {
		display: flex;
		align-items: stretch;
		min-height: 260px;
	}

	.side {
		flex: 0 1 300px;
		min-width: 210px;
		padding: 14px 16px;
		border-radius: 18px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.side.moves {
		--tone: var(--side-b);
	}

	.side.onto {
		--tone: var(--side-a);
		border-color: color-mix(in srgb, var(--side-a) 55%, transparent);
	}

	.top {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.badge {
		width: 24px;
		height: 24px;
		border-radius: 7px;
		display: grid;
		place-items: center;
		flex: none;
		color: var(--tone);
		background: color-mix(in srgb, var(--tone) 18%, transparent);
	}

	.name {
		font-weight: 650;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.role {
		margin-left: auto;
		flex: none;
		font-size: var(--fs-mono);
		padding: 2px 10px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--tone) 50%, transparent);
		color: var(--tone);
	}

	.commits {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.commits li {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
		font-size: var(--fs-secondary);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--tone);
		flex: none;
		align-self: center;
	}

	.muted {
		color: var(--muted);
		flex: none;
	}

	.summary {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.grow {
		flex: 1;
	}

	.connector {
		flex: 1 1 56px;
		min-width: 56px;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 6px;
		padding: 0 6px;
	}

	.caption {
		font-size: var(--fs-mono);
		text-align: center;
		font-weight: 600;
		color: var(--side-b);
		min-height: 1.2em;
	}

	.caption.tone-onto {
		color: var(--side-a);
	}

	.line {
		display: flex;
		align-items: center;
	}

	.moves-line {
		color: var(--side-b);
	}

	.onto-line {
		color: var(--side-a);
	}

	.line svg {
		flex: none;
		fill: currentColor;
	}

	.rule {
		flex: 1;
		height: 0;
		border-top: 2.5px solid currentColor;
	}

	.result {
		flex: 0 1 480px;
		min-width: 320px;
		padding: 18px 20px;
		border-radius: 22px;
		display: flex;
		flex-direction: column;
		gap: 13px;
		background: var(--surface-2);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		box-shadow: var(--ornament-shadow);
	}

	.result-head,
	.card-head {
		display: flex;
		align-items: baseline;
		gap: 8px;
		flex-wrap: wrap;
	}

	.label {
		font-size: var(--fs-mono);
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 600;
	}

	.target {
		font-size: calc(var(--fs-ui) * 1.1);
		font-weight: 700;
		color: var(--ok);
		overflow-wrap: anywhere;
	}

	.sentence {
		margin: 0;
		font-size: calc(var(--fs-ui) * 0.92);
		line-height: 1.55;
	}

	.stats {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}

	.stat {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding: 8px 10px;
		border-radius: 12px;
		background: color-mix(in srgb, var(--sunken) 60%, transparent);
	}

	.value {
		font-size: calc(var(--fs-title) * 1.15);
		font-weight: 650;
		line-height: 1.1;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.box {
		--tone: var(--ok);
		display: flex;
		flex-direction: column;
		gap: 7px;
		padding: 10px 12px;
		border-radius: 14px;
		background: color-mix(in srgb, var(--tone) 10%, transparent);
		border: 1px solid color-mix(in srgb, var(--tone) 30%, transparent);
		color: var(--tone);
	}

	.box.danger {
		--tone: var(--danger);
	}

	.box-head {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.box-title {
		font-weight: 650;
	}

	.meter {
		display: flex;
		gap: 3px;
		height: 6px;
	}

	.meter span {
		flex: 1;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--ok) 70%, transparent);
	}

	.meter span.red {
		background: var(--danger);
	}

	.bar {
		height: 6px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
		overflow: hidden;
	}

	.fill {
		display: block;
		height: 100%;
		background: var(--accent);
		transition: width 0.3s var(--ease);
	}

	.actions {
		display: flex;
		gap: 8px;
		align-items: center;
		flex-wrap: wrap;
	}

	.lower {
		display: flex;
		gap: 14px;
		flex-wrap: wrap;
		align-items: flex-start;
	}

	.plan-card {
		flex: 1 1 520px;
		min-width: 0;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		border-radius: 16px;
	}

	.history-card {
		flex: 0 1 340px;
		min-width: 260px;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		border-radius: 16px;
	}

	.error {
		color: var(--danger);
	}

	@media (max-width: 1100px) {
		.stage {
			flex-wrap: wrap;
			gap: 12px;
		}

		.connector {
			display: none;
		}

		.side,
		.result {
			flex: 1 1 300px;
		}
	}
</style>
