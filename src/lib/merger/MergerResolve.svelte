<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import Resolver from '$lib/resolver/Resolver.svelte';
	import { resolving } from './resolve.svelte';
	import { merger } from './store.svelte';
	import AgentCard from '$lib/agents/AgentCard.svelte';
	import AssignPopover from '$lib/agents/AssignPopover.svelte';
	import { agents } from '$lib/agents/store.svelte';
	import { isLive } from '$lib/agents/levels';
	import type { Assigned, Proposal } from '$lib/agents/types';
	import * as agentWork from './agent.svelte';

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

	// ── Agents (2.0): with none set up, nothing below draws. ──────────────
	const offered = $derived(agents.usable('merge').length > 0);
	const assignment = $derived(agentWork.current());
	const working = $derived(assignment !== null && isLive(assignment));
	const proposals = $derived(agentWork.proposalsFor(assignment, resolving.files, names));
	const at = $derived(agentWork.workingOn(assignment));
	let assigning = $state(false);
	let starting = $state(false);
	/** The Agent card can be put away, like the Graph's detail panel. */
	let cardHidden = $state(false);

	async function assign(chosen: Assigned) {
		starting = true;
		const started = await agentWork.assign(chosen);
		starting = false;
		if (started) {
			assigning = false;
			cardHidden = false;
		}
	}

	/** The commit dialog, with the agents' trailers in the message. */
	function complete() {
		const names = resolving.agentsInResult;
		if (names.length) merger.setMessage(agentWork.withTrailers(merger.message, names));
		merger.openCommit('resolve');
	}

	// Landed by the person after an agent stopped at Land: its job is done.
	$effect(() => {
		if (merger.phase === 'done' && assignment && assignment.state === 'waiting') {
			void agents.control(assignment.id, { kind: 'acted', ok: true });
		}
	});

	function goTo(proposals: Proposal[]) {
		const first = proposals.find((p) => p.body.kind === 'resolution');
		if (first && first.body.kind === 'resolution') {
			const card = document.querySelector(`[id="conflict-${CSS.escape(first.body.path)}-${first.body.region}"]`);
			card?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
		}
	}

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
		{#if offered && !working && !stop}
			<span class="assign-anchor">
				<Btn onclick={() => (assigning = !assigning)}><Icon name="agent" size="1em" />Assign an agent…</Btn>
				{#if assigning}
					<div class="below">
						<AssignPopover job="merge" busy={starting} onassign={assign} oncancel={() => (assigning = false)} />
					</div>
				{/if}
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
				onclick={complete}
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
		<div class="work">
			<Resolver
				files={resolving.files}
				choices={resolving.choices}
				{names}
				{roles}
				baseShort={resolving.baseShort}
				{others}
				start={resolving.start}
				onchoose={(path, index, choice) => {
					resolving.choose(path, index, choice);
					agentWork.personChose(path, index, choice);
				}}
				onwhole={(path, side) => resolving.whole(path, side)}
				agent={assignment?.agent.name ?? null}
				authors={assignment ? resolving.authors : {}}
				{proposals}
				working={at}
				onaccept={assignment ? (path, index) => agentWork.accept(assignment, path, index) : undefined}
				onwhy={assignment && assignment.level !== 'unattended' ? (path, index) => agentWork.askWhy(assignment, path, index) : undefined}
			/>
			{#if assignment && !cardHidden}
				<div class="agent-inset">
					<AgentCard
						{assignment}
						{names}
						ongo={goTo}
						onaccept={(proposals) => agentWork.acceptAll(assignment, proposals)}
						onlast={complete}
						onhide={() => (cardHidden = true)}
					/>
				</div>
			{:else if assignment}
				<button class="agent-tab" title="Show the agent" onclick={() => (cardHidden = false)}>
					<Icon name="chevron-left" size="0.95em" weight={2.2} />
					<Icon name="agent" size="0.95em" weight={2} />
				</button>
			{/if}
		</div>
	{/if}
</div>

<style>
	.resolve {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	/* The resolver, and beside it the Agent card while one is assigned. */
	.work {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
	}

	.work > :global(:first-child) {
		flex: 1;
		min-width: 0;
	}

	.agent-inset {
		width: var(--room-conversation-w);
		flex: none;
		display: flex;
		min-height: 0;
		margin-bottom: 10px;
	}

	.agent-inset :global(.agent-card) {
		flex: 1;
		border-radius: var(--r-floating);
	}

	.agent-tab {
		flex: none;
		width: 30px;
		margin-bottom: 10px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding-top: 12px;
		border-radius: var(--r-panel);
		border: 1px solid var(--pane-edge);
		color: var(--agent);
	}

	.agent-tab:hover {
		background: var(--hover);
	}

	.assign-anchor {
		position: relative;
	}

	.below :global(.assign) {
		top: calc(100% + 8px);
		bottom: auto;
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
