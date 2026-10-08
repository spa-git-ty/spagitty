<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import BranchCard from './BranchCard.svelte';
	import HistoryAfter from './HistoryAfter.svelte';
	import {
		afterLabel,
		cannotLand,
		conflictSummary,
		connector,
		fileStatus,
		filesLine,
		meter,
		primaryAction,
		sentence,
		splitPath,
		stats,
		type Into,
		type SideKey
	} from './plan';
	import SideBadge from './SideBadge.svelte';
	import { merger } from './store.svelte';
	import AssignPopover from '$lib/agents/AssignPopover.svelte';
	import { agents } from '$lib/agents/store.svelte';
	import { cardLine, isLive } from '$lib/agents/levels';
	import type { Assigned } from '$lib/agents/types';
	import * as agentWork from './agent.svelte';

	/**
	 * Merger's plan (FEAT-100): a dry run, and the screen says so. The two
	 * branches either side of the Result, where it lands and how, the history
	 * it leaves, and every file either branch touched. Nothing here writes.
	 *
	 * `onresolve` opens resolving at a file (or the first conflict); `onmerge`
	 * lands a merge that has no conflicts. Without them the buttons are shown,
	 * disabled.
	 */
	interface Props {
		onresolve?: (path: string | null) => void;
		onmerge?: () => void;
		/** A merge is being written. */
		busy?: boolean;
	}

	let { onresolve, onmerge, busy = false }: Props = $props();

	// ── Agents (2.0): nothing here unless one is set up for merges. ────────
	const offered = $derived(agents.usable('merge').length > 0);
	const assignment = $derived(merger.a && merger.b ? agentWork.current() : null);
	let assigning = $state(false);
	let starting = $state(false);

	async function assign(chosen: Assigned) {
		starting = true;
		const started = await agentWork.assign(chosen);
		starting = false;
		if (started) assigning = false;
	}

	const forecast = $derived(merger.forecast);
	const who = $derived(merger.roles);
	const strategy = $derived(merger.strategy);
	const choices = $derived(merger.choices);
	const blocked = $derived(
		forecast ? cannotLand(forecast, merger.into, merger.newName, merger.localNames) : null
	);
	const summary = $derived(forecast && who ? conflictSummary(forecast, who, strategy) : null);
	const action = $derived(forecast ? primaryAction(forecast, strategy) : null);
	const nothing = $derived(forecast && who ? forecast[who.source].ahead === 0 : false);
	/** Why the forecast's primary action cannot run, or null. */
	const stopped = $derived(
		nothing ? 'Nothing comes in' : blocked ?? (choices.find((c) => c.id === strategy)?.reason ?? null)
	);

	const DIRECTIONS: { into: Into; badge: SideKey | null }[] = [
		{ into: 'a', badge: 'a' },
		{ into: 'b', badge: 'b' },
		{ into: 'new', badge: null }
	];

	function directionLabel(into: Into): string {
		if (into === 'new') return 'Into a new branch';
		return `Into ${(into === 'a' ? merger.a : merger.b) ?? into.toUpperCase()}`;
	}

	function directionReason(into: Into): string | null {
		if (!forecast || into === 'new') return null;
		return cannotLand(forecast, into, merger.newName, merger.localNames);
	}

	/** What the target is drawn in: its side's colour, or green when new. */
	const targetTone = $derived(who ? (who.isNew ? 'new' : who.onto) : 'a');
</script>

<div class="plan">
	<header class="head">
		<span class="tile" aria-hidden="true"><Icon name="merge" size="1.25em" weight={1.9} /></span>
		<div class="titles">
			<h1 class="title">Merger</h1>
			<span class="note">Pick two branches and where the result lands. You see the outcome, and every conflict, before anything is written.</span>
		</div>
		<span class="dry"><span class="pip" aria-hidden="true"></span>Dry run · nothing written yet</span>
	</header>

	<div class="stage">
		<BranchCard
			key="a"
			name={merger.a}
			side={forecast?.a ?? null}
			baseShort={forecast?.baseShort ?? ''}
			{who}
			options={merger.pickables}
			other={merger.b}
			current={merger.current}
			onpick={(name) => merger.pickA(name)}
		/>

		{#if forecast && who}
			{@const arrow = connector(forecast, 'a', who)}
			<div class="connector" aria-hidden="true">
				<span class="caption" class:solid={arrow.solid} class:tone-a={arrow.solid}>{arrow.label}</span>
				<span class="line">
					<span class="rule" class:solid={arrow.solid} class:tone-a={arrow.solid}></span>
					<svg width="10" height="12" viewBox="0 0 10 12" class:tone-a={arrow.solid}><path d="M0 0 10 6 0 12z"></path></svg>
				</span>
			</div>
		{:else}
			<div class="connector" aria-hidden="true"></div>
		{/if}

		<section class="result" aria-label="Result" aria-busy={merger.loading}>
			{#if forecast && who && summary && action}
				<div class="result-head">
					<span class="label">Result</span>
					<span class="target mono tone-{targetTone}">{who.targetName}</span>
					<span class="note">{afterLabel(strategy)}</span>
				</div>
				{#if who.isNew}
					<div class="new-name">
						<label for="merger-new-branch" class="note">New branch</label>
						<input
							id="merger-new-branch"
							class="mono"
							value={merger.newName}
							spellcheck="false"
							oninput={(event) => merger.setNewName((event.currentTarget as HTMLInputElement).value)}
						/>
					</div>
				{/if}
				<p class="sentence">{sentence(forecast, who, strategy)}</p>
				<div class="stats">
					{#each stats(forecast, who, strategy) as stat (stat.label)}
						<div class="stat"><span class="value">{stat.value}</span><span class="note small">{stat.label}</span></div>
					{/each}
				</div>
				<div class="conflicts" class:none={forecast.conflicts === 0}>
					<div class="conflicts-head">
						<Icon name={forecast.conflicts === 0 ? 'check' : 'conflict'} size="1em" weight={2} />
						<span class="conflicts-title">{summary.title}</span>
						<span class="grow"></span>
						<span class="note small">{summary.clean}</span>
					</div>
					<div class="meter" aria-hidden="true">
						{#each meter(forecast) as red, index (index)}
							<span class:red></span>
						{/each}
					</div>
					<span class="note small">{summary.note}</span>
				</div>
				<div class="actions">
					{#if action.kind === 'resolve'}
						<Btn
							primary
							disabled={!onresolve || stopped !== null || busy}
							title={stopped ?? undefined}
							onclick={() => onresolve?.(null)}
						>
							{action.label}<Icon name="chevron-right" size="0.95em" weight={2.2} />
						</Btn>
						{#if offered && !(assignment && isLive(assignment))}
							<span class="assign-anchor">
								<Btn disabled={stopped !== null || busy} onclick={() => (assigning = !assigning)}>
									<Icon name="agent" size="1em" />Assign an agent…
								</Btn>
								{#if assigning}
									<AssignPopover job="merge" busy={starting} onassign={assign} oncancel={() => (assigning = false)} />
								{/if}
							</span>
						{/if}
						<Btn disabled title="Resolve the conflicts first">Merge now</Btn>
					{:else}
						<Btn
							primary
							{busy}
							disabled={!onmerge || stopped !== null}
							title={stopped ?? undefined}
							onclick={() => onmerge?.()}
						>
							{busy ? 'Merging…' : 'Merge now'}
						</Btn>
					{/if}
					{#if blocked}<span class="note error">{blocked}</span>{/if}
				</div>
				{#if assignment}
					<span class="agent-line note" title={assignment.sentence}>
						<Icon name="agent" size="0.95em" weight={2} />{cardLine(assignment)}
					</span>
				{/if}
			{:else if merger.error}
				<p class="note error" role="alert">{merger.error}</p>
			{:else if !merger.a || !merger.b}
				<p class="note">Choose two branches.</p>
			{:else}
				<Loader label="Working out the merge…" />
			{/if}
		</section>

		{#if forecast && who}
			{@const arrow = connector(forecast, 'b', who)}
			<div class="connector" aria-hidden="true">
				<span class="caption" class:solid={arrow.solid} class:tone-b={arrow.solid}>{arrow.label}</span>
				<span class="line">
					<svg width="10" height="12" viewBox="0 0 10 12" class:tone-b={arrow.solid}><path d="M10 0 0 6 10 12z"></path></svg>
					<span class="rule" class:solid={arrow.solid} class:tone-b={arrow.solid}></span>
				</span>
			</div>
		{:else}
			<div class="connector" aria-hidden="true"></div>
		{/if}

		<BranchCard
			key="b"
			name={merger.b}
			side={forecast?.b ?? null}
			baseShort={forecast?.baseShort ?? ''}
			{who}
			options={merger.pickables}
			other={merger.a}
			current={merger.current}
			onpick={(name) => merger.pickB(name)}
		/>
	</div>

	<div class="lands">
		<span class="note">The result lands</span>
		<span class="segments" role="group" aria-label="Where the result lands">
			{#each DIRECTIONS as direction (direction.into)}
				{@const reason = directionReason(direction.into)}
				<button
					class="segment"
					aria-pressed={merger.into === direction.into}
					title={reason ?? undefined}
					onclick={() => merger.setInto(direction.into)}
				>
					{#if direction.badge}<SideBadge side={direction.badge} />{/if}
					<span class="mono-name">{directionLabel(direction.into)}</span>
				</button>
			{/each}
		</span>
		<Btn onclick={() => merger.swap()} title="Swap the two branches' roles">
			<Icon name="swap" size="1em" weight={1.9} />Swap
		</Btn>
	</div>

	{#if forecast && who}
		<div class="lower">
			<section class="card how" aria-label="How it lands">
				<span class="label">How it lands</span>
				{#each choices as choice (choice.id)}
					<button
						class="strategy"
						aria-pressed={strategy === choice.id}
						disabled={choice.reason !== null}
						title={choice.reason ?? undefined}
						onclick={() => merger.setStrategy(choice.id)}
					>
						<span class="radio" aria-hidden="true"></span>
						<span class="strategy-text">
							<span class="strategy-label">{choice.label}</span>
							<span class="note small">{choice.description}</span>
						</span>
					</button>
				{/each}
			</section>

			<HistoryAfter
				{strategy}
				target={who.isNew ? 'new' : who.onto}
				source={who.source}
				targetName={who.targetName}
				sourceName={who.sourceName}
				baseShort={forecast.baseShort}
			/>

			<section class="card changes" aria-label="What changes">
				<div class="changes-head">
					<span class="label">What changes</span>
					<span class="note small">{filesLine(forecast, who)}</span>
				</div>
				<ul class="files">
					{#each forecast.files as file (file.path)}
						{@const status = fileStatus(file, forecast, who)}
						{@const parts = splitPath(file.path)}
						<li>
							{#if file.touch === 'conflict'}
								<button
									class="file conflict"
									disabled={!onresolve || blocked !== null || busy}
									onclick={() => onresolve?.(file.path)}
								>
									<span class="pip tone-{status.dot}"></span>
									<span class="path mono"><span class="muted">{parts.dir}</span>{parts.name}</span>
									<span class="grow"></span>
									<span class="status tone-{status.tone} strong">{status.label}</span>
									<Icon name="chevron-right" size="0.95em" weight={2.2} />
								</button>
							{:else}
								<span class="file">
									<span class="pip tone-{status.dot}"></span>
									<span class="path mono"><span class="muted">{parts.dir}</span>{parts.name}</span>
									<span class="grow"></span>
									<span class="status tone-{status.tone}">{status.label}</span>
								</span>
							{/if}
						</li>
					{:else}
						<li class="note">Neither branch changed a file since they split.</li>
					{/each}
				</ul>
			</section>
		</div>
	{/if}
</div>

<style>
	.assign-anchor {
		position: relative;
		display: inline-flex;
	}

	.agent-line {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		color: var(--agent);
	}

	.plan {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 16px 20px 96px;
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
		flex: 1 1 420px;
		min-width: 0;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.dry {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--ok) 45%, transparent);
		color: var(--ok);
		font-size: var(--fs-secondary);
		background-color: var(--surface-veil);
		white-space: nowrap;
	}

	.pip {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--ok);
		flex: none;
	}

	/* Rows of the plan keep their content's height: the plan scrolls, its
	   rows never squeeze under one another. */
	.head,
	.stage,
	.lands,
	.lower {
		flex: none;
	}

	.stage {
		display: flex;
		align-items: stretch;
		min-height: 300px;
	}

	.connector {
		flex: 1 1 60px;
		min-width: 64px;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 6px;
		padding: 0 6px;
	}

	.caption {
		font-size: var(--fs-mono);
		text-align: center;
		line-height: 1.25;
		color: var(--muted);
	}

	.caption.solid {
		font-weight: 600;
	}

	.line {
		display: flex;
		align-items: center;
		color: var(--muted);
	}

	.line svg {
		flex: none;
		fill: currentColor;
	}

	.rule {
		flex: 1;
		height: 0;
		border-top: 2px dashed currentColor;
	}

	.rule.solid {
		border-top: 2.5px solid currentColor;
	}

	.tone-a {
		color: var(--side-a);
	}

	.tone-b {
		color: var(--side-b);
	}

	.tone-new,
	.tone-ok {
		color: var(--ok);
	}

	.tone-danger {
		color: var(--danger);
	}

	.tone-muted {
		color: var(--muted);
	}

	.result {
		flex: 0 1 460px;
		min-width: 340px;
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

	.result-head {
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
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.new-name {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.new-name label {
		flex: none;
	}

	.new-name input {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-secondary);
		border: 1px solid var(--pane-edge);
		background: var(--sunken);
		color: var(--ink);
		border-radius: 8px;
		padding: 6px 10px;
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

	.conflicts {
		display: flex;
		flex-direction: column;
		gap: 7px;
		padding: 10px 12px;
		border-radius: 14px;
		background: color-mix(in srgb, var(--danger) 10%, transparent);
		border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent);
		color: var(--danger);
	}

	.conflicts.none {
		background: color-mix(in srgb, var(--ok) 10%, transparent);
		border-color: color-mix(in srgb, var(--ok) 30%, transparent);
		color: var(--ok);
	}

	.conflicts-head {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.conflicts-title {
		font-weight: 650;
	}

	.grow {
		flex: 1;
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

	.actions {
		display: flex;
		gap: 8px;
		align-items: center;
		flex-wrap: wrap;
	}

	.lands {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}

	.segments {
		display: flex;
		flex-wrap: wrap;
		padding: 3px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
		border: 1px solid var(--pane-edge);
	}

	.segment {
		display: flex;
		align-items: center;
		gap: 7px;
		height: 32px;
		padding: 0 12px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		color: var(--muted);
		white-space: nowrap;
	}

	.segment:hover {
		background: var(--hover);
	}

	.segment[aria-pressed='true'] {
		background: var(--surface-2);
		color: var(--ink);
		font-weight: 600;
		box-shadow: var(--shadow-1);
	}

	.lower {
		display: flex;
		gap: 14px;
		flex-wrap: wrap;
		align-items: stretch;
	}

	.how {
		flex: 0 1 330px;
		min-width: 260px;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		border-radius: 16px;
	}

	.how .label {
		padding: 0 4px 6px;
	}

	.strategy {
		text-align: left;
		display: flex;
		gap: 10px;
		padding: 9px 10px;
		border-radius: 12px;
		align-items: flex-start;
	}

	.strategy:hover:not(:disabled) {
		background: var(--hover);
	}

	.strategy[aria-pressed='true'] {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
	}

	.strategy:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.radio {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		flex: none;
		margin-top: 2px;
		box-sizing: border-box;
		border: 1.5px solid var(--line);
	}

	.strategy[aria-pressed='true'] .radio {
		border: 5px solid var(--accent);
	}

	.strategy-text {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.strategy-label {
		font-weight: 600;
		font-size: var(--fs-secondary);
	}

	.changes {
		flex: 1 1 380px;
		min-width: 0;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		border-radius: 16px;
	}

	.changes-head {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
		padding: 0 4px 6px;
	}

	.files {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.file {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 7px 10px;
		border-radius: 10px;
		text-align: left;
		min-width: 0;
	}

	button.file:hover:not(:disabled) {
		background: var(--hover);
	}

	button.file :global(svg) {
		color: var(--danger);
		flex: none;
	}

	.file .pip {
		width: 8px;
		height: 8px;
		background: currentColor;
	}

	.path {
		font-size: var(--fs-secondary);
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status {
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 3px 10px;
		font-size: var(--fs-mono);
		white-space: nowrap;
		background-color: var(--surface-veil);
		flex: none;
	}

	.status.strong {
		font-weight: 600;
		border-color: color-mix(in srgb, var(--danger) 45%, transparent);
	}

	.error {
		color: var(--danger);
	}
</style>
