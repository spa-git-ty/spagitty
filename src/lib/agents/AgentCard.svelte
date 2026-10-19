<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { fileAttrs } from '$lib/files/open';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import StepDetails from './StepDetails.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import * as api from './api';
	import { agents } from './store.svelte';
	import {
		LEVELS,
		above,
		askOf,
		findingPlace,
		grouped,
		isLive,
		isWorking,
		levelWords,
		meter,
		nowLine,
		planOf,
		quietFor,
		quietWords,
		share,
		stepTime
	} from './levels';
	import type { Assignment, Proposal, Step } from './types';

	/**
	 * The Agent card (2.0): what an agent is doing, while it does it.
	 *
	 * Four layers, each quieter than the one before: one sentence that is
	 * always true; its presence on the work (drawn by the screen, not here);
	 * the timeline — each step folded to a line; and the raw output, folded
	 * under it.
	 *
	 * The sentence moves while the agent is at work and heard from, and only
	 * then. When it stops for the person, what it asks is under the sentence
	 * with its buttons, at the top of the card and never in the timeline, where
	 * it scrolled out of sight; what exactly is asked opens under them
	 * (FEAT-124).
	 *
	 * Every control is a button. Pause stops at the end of the step, Stop ends
	 * the run now, and both keep everything proposed.
	 */
	interface Props {
		assignment: Assignment;
		/** The tabs above the card, where it shares a place with another. */
		tabs?: Snippet;
		/** The merge's branch names, for a resolution's words. */
		names?: { a: string; b: string };
		/** Whether the screen follows the agent; absent where it cannot. */
		follow?: boolean;
		onfollow?: (on: boolean) => void;
		/** Take the person to what waits: the findings, the conflict. */
		ongo?: (proposals: Proposal[]) => void;
		/** Make what waits the person's: drafts accepted, choices made. */
		onaccept?: (proposals: Proposal[]) => Promise<void> | void;
		/** The person's last act, filled in: Finish review, Complete merge. */
		onlast?: () => void;
		/** Put the card away, where the screen lets it be. */
		onhide?: () => void;
		/** Carry on a stopped or failed run from its last finished step. */
		onresume?: () => void;
		/** Show a file the agent read in the diff, where there is one. */
		onfile?: (path: string) => void;
	}

	let { assignment, tabs, names, follow, onfollow, ongo, onaccept, onlast, onhide, onresume, onfile }: Props = $props();

	let now = $state(Math.floor(Date.now() / 1000));
	/** The step Details shows; the current one until another is chosen. */
	let selected = $state<number | null>(null);
	/** Everything about a step is hidden until this is opened (FEAT-118). */
	let details = $state(false);
	let raw = $state(false);
	let transcript = $state<string | null>(null);
	let told = $state('');
	/** The step whose question is open in full. A new question starts closed. */
	let asked = $state<number | null>(null);
	let menu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	onMount(() => {
		const timer = setInterval(() => (now = Math.floor(Date.now() / 1000)), 1000);
		return () => clearInterval(timer);
	});

	const a = $derived(assignment);
	const live = $derived(isLive(a));
	const ask = $derived(askOf(a, names));
	const more = $derived(ask?.step != null && asked === ask.step.index);
	const plan = $derived(planOf(a));
	const highest = $derived(agents.rules?.highest ?? 'signOff');
	const remote = $derived(a.agent.reach === 'remote');
	const budget = $derived(
		remote ? (agents.snapshot?.remote.find((r) => r.id === a.agent.id)?.tokensPerRun ?? null) : null
	);
	/** How long it has said nothing, once that is long enough to say so (FEAT-119). */
	const quiet = $derived(quietFor(a, now));
	const doing = $derived(nowLine(a, now));
	const tone = $derived(
		a.state === 'waiting'
			? 'waiting'
			: a.state === 'failed'
				? 'failed'
				: a.state === 'done'
					? 'done'
					: a.state === 'paused'
						? 'paused'
						: quiet !== null
							? 'quiet'
							: live
								? 'working'
								: 'stopped'
	);
	const working = $derived(isWorking(a, now));
	const steps = $derived(a.steps.filter((s) => s.kind.kind !== 'why'));
	const lines = $derived(agents.lines(a.id));
	const shown = $derived(
		steps.find((s) => s.index === selected) ??
			steps.findLast((s) => s.state === 'running' || s.state === 'waiting') ??
			steps.at(-1) ??
			null
	);

	function send(control: Parameters<typeof agents.control>[1]) {
		void agents.control(a.id, control);
	}

	function openLevels(event: MouseEvent) {
		const anchor = event.currentTarget as HTMLElement;
		const box = anchor.getBoundingClientRect();
		menu = menu ? null : { x: box.left, y: box.bottom + 4, anchor };
	}

	const levelItems = $derived<MenuItem[]>(
		LEVELS.map((level) => ({
			id: level.id,
			label: level.label,
			note: a.level === level.id ? 'now' : undefined,
			disabled: above(level.id, highest),
			reason: above(level.id, highest) ? `The most allowed here is ${levelWords(highest).label}` : undefined,
			run: () => send({ kind: 'level', level: level.id })
		}))
	);

	async function redo(step: Step) {
		const note = await dialog.prompt({
			title: 'Redo with a note',
			body: `${step.label} runs again with what was wrong. What it proposed is kept, marked superseded.`,
			label: 'What was wrong',
			confirmLabel: 'Redo'
		});
		if (note) send({ kind: 'redo', note });
	}

	async function accept(proposals: Proposal[]) {
		await onaccept?.(proposals);
		send({ kind: 'continue' });
	}

	function tell() {
		const text = told.trim();
		if (!text) return;
		send({ kind: 'tell', text });
		told = '';
	}

	async function showRaw() {
		raw = !raw;
		if (raw) transcript = await api.transcript(a.repo, a.id).catch(() => null);
	}

	function dropFromPlan(files: string[], path: string) {
		send({ kind: 'plan', files: files.filter((f) => f !== path) });
	}

	function icon(step: Step): 'check' | 'warning' | 'close' | 'circle' {
		if (step.state === 'done') return 'check';
		if (step.state === 'waiting') return 'warning';
		if (step.state === 'failed') return 'close';
		return 'circle';
	}

	const waitingProposals = $derived(ask?.proposals ?? []);
	const lastCommand = $derived([...a.steps].reverse().find((s) => s.command)?.command ?? null);
</script>

<aside class="card agent-card" aria-label="Agent">
	{#if tabs}<div class="tabs">{@render tabs()}</div>{/if}

	<div class="head">
		<span class="mark"><Icon name="agent" size="1.1em" weight={2} /></span>
		<span class="name">{a.agent.name}</span>
		{#if live}
			<Chip title="How far {a.agent.name} goes alone" onclick={openLevels}>
				{levelWords(a.level).short}<Icon name="chevron-down" size="0.85em" />
			</Chip>
			{#if a.state === 'paused'}
				<Btn title="Resume" onclick={() => send({ kind: 'resume' })}><Icon name="play" size="1em" /></Btn>
			{:else}
				<Btn title={a.pausing ? 'Pausing after this step' : 'Pause after this step'} disabled={a.pausing} onclick={() => send({ kind: 'pause' })}>
					<Icon name="pause" size="1em" />
				</Btn>
			{/if}
			<Btn title="Stop now. Everything proposed stays." onclick={() => send({ kind: 'stop' })}>
				<Icon name="stop" size="1em" />
			</Btn>
		{:else}
			<span class="note">{levelWords(a.level).label}</span>
		{/if}
		{#if onhide}
			<Btn title="Put the card away" onclick={onhide}><Icon name="chevron-right" size="1em" /></Btn>
		{/if}
	</div>

	<div class="now" class:asking={ask !== null}>
		<p class="sentence {tone}" aria-live="polite">
			{#if working}<span class="strands" aria-hidden="true"><Loader size="inline" label="" /></span>{/if}
			<span class:shimmer={working}>{a.pausing && live ? 'Pausing after this step' : a.sentence}</span>
		</p>
		{#if live && doing && a.state === 'working' && quiet === null}
			<p class="doing note mono" data-testid="doing">{doing}</p>
		{/if}
		{#if ask}
			{@const step = ask.step}
			<!-- The sentence often says it already: "Waiting for you: bun test failed". -->
			{#if !a.sentence.includes(ask.summary)}<span class="summary">{ask.summary}</span>{/if}
			<div class="acts">
				{#if !step}
					<Btn primary quiet onclick={() => send({ kind: 'carryOn' })}>Carry on</Btn>
					<Btn onclick={() => send({ kind: 'stop' })}>Stop</Btn>
				{:else if step.gate === 'plan'}
					<Btn primary quiet onclick={() => send({ kind: 'continue' })}>Go on</Btn>
					<Btn onclick={() => redo(step)}>Redo with a note</Btn>
				{:else if step.gate === 'send' || step.gate === 'land'}
					<Btn primary quiet onclick={() => onlast?.()}>{step.gate === 'send' ? 'Finish review' : 'Complete merge'}</Btn>
				{:else if step.gate === 'checks'}
					<Btn primary quiet onclick={() => send({ kind: 'continue' })}>Go on</Btn>
					<Btn onclick={() => send({ kind: 'redo', note: '' })}>Run again</Btn>
				{:else}
					{#if waitingProposals.length && onaccept}
						<Btn primary quiet onclick={() => accept(waitingProposals)}>
							{waitingProposals.length === 1 ? 'Accept' : waitingProposals.length === 2 ? 'Accept both' : 'Accept all'}
						</Btn>
					{/if}
					{#if waitingProposals.length && ongo && waitingProposals[0].body.kind !== 'verdict'}
						<Btn onclick={() => ongo?.(waitingProposals)}>{waitingProposals.length === 1 ? 'Go to it' : 'Go to them'}</Btn>
					{/if}
					<Btn onclick={() => send({ kind: 'continue' })}>{waitingProposals.length ? 'Go on, decide later' : 'Go on'}</Btn>
					<Btn onclick={() => redo(step)}>Redo with a note</Btn>
				{/if}
			</div>
			{#if step && ask.more}
				<button class="raw-toggle note" aria-expanded={more} onclick={() => (asked = more ? null : step.index)}>
					<Icon name={more ? 'chevron-down' : 'chevron-right'} size="0.9em" />{ask.more}
				</button>
				{#if more}
					<div class="asked">
						{#if step.gate === 'plan'}
							{@const files = plan?.files ?? []}
							<ol class="plan">
								{#each files as item (item.path)}
									<li>
										<span class="what">
											<span class="mono" {...fileAttrs(item.path, 'HEAD', undefined, { click: true })}>{item.path}</span>
											{#if item.why}<span class="note">{item.why}</span>{/if}
										</span>
										<Chip title="Leave {item.path} out" onclick={() => dropFromPlan(files.map((f) => f.path), item.path)}>
											<Icon name="close" size="0.8em" />
										</Chip>
									</li>
								{/each}
							</ol>
						{:else if step.gate === 'checks'}
							{@const failed = step.checks.find((c) => !c.passed)}
							{#if failed}<pre class="output">{failed.output.split('\n').slice(-12).join('\n')}</pre>{/if}
						{:else}
							<ul class="findings">
								{#each waitingProposals as proposal (proposal.id)}
									{#if proposal.body.kind === 'verdict'}
										<li><span class="said">{proposal.body.summary}</span></li>
									{:else if proposal.body.kind === 'comment'}
										<li>
											<span class="where">
												<span class="mono">{findingPlace(proposal.body)}</span>
												<span class="note">{proposal.body.severity}{proposal.sure ? '' : ' · unsure'}</span>
											</span>
											<span class="said">{proposal.body.body}</span>
										</li>
									{/if}
								{/each}
							</ul>
						{/if}
					</div>
				{/if}
			{/if}
		{/if}
	</div>
	{#if quiet !== null}
		<div class="quiet" role="status">
			<span>Nothing from {a.agent.name} for {quietWords(quiet)}.</span>
			<Btn title="Stop now. Everything proposed stays." onclick={() => send({ kind: 'stop' })}>Stop</Btn>
		</div>
	{/if}
	<div class="meter">
		<span class="bar"><span class="fill" style:width="{share(a) * 100}%"></span></span>
		<span class="note mono">{meter(a, now)}</span>
	</div>
	{#if remote}
		<span class="note mono">
			{grouped(a.tokens.input + a.tokens.output)}{budget ? ` / ${grouped(budget)}` : ''} tokens
		</span>
	{/if}
	{#if live}
		<div class="controls">
			<Chip title="Stop, and finish by hand with what it proposed" onclick={() => send({ kind: 'takeOver' })}>Take over</Chip>
			{#if onfollow}
				<Chip active={follow} title="Move the room with the agent" onclick={() => onfollow?.(!follow)}>Follow the agent</Chip>
			{/if}
		</div>
	{/if}

	<div class="timeline" class:tight={more}>
		<VirtualRows items={steps} key={(step) => String(step.index)} estimate={() => 34} label="Timeline">
			{#snippet row(step)}
				<div class="step {step.state}">
					<button
						class="line"
						onclick={() => {
							selected = step.index;
							details = true;
						}}
						aria-pressed={details && shown?.index === step.index}
						title="Show what was sent and what came back"
					>
						<span class="icon"><Icon name={icon(step)} size="0.95em" weight={2} /></span>
						<span class="label">{step.label}</span>
						<span class="note mono">{stepTime(step, now)}</span>
					</button>
				</div>
			{/snippet}
		</VirtualRows>
		{#if !live && (a.reason || a.tookOver)}
			<p class="ended note" class:failed={a.state === 'failed'}>{a.tookOver ?? a.reason}</p>
		{/if}
	</div>

	{#if live}
		<div class="tell">
			<input
				type="text"
				class="field"
				aria-label="Tell the agent"
				placeholder="Tell the agent…"
				bind:value={told}
				onkeydown={(event) => {
					if (event.key === 'Enter') tell();
				}}
			/>
			<Btn title="Send" disabled={!told.trim()} onclick={tell}><Icon name="arrow-right" size="1em" /></Btn>
		</div>
	{:else}
		<div class="controls">
			{#if onresume && (a.state === 'stopped' || a.state === 'failed')}
				<Btn primary quiet title="A new run from the last finished step, told what was already done" onclick={onresume}>Resume</Btn>
			{/if}
			<Chip onclick={() => agents.forget(a.id)}>forget</Chip>
		</div>
	{/if}
<button class="raw-toggle note" aria-expanded={details} onclick={() => (details = !details)}>
		<Icon name={details ? 'chevron-down' : 'chevron-right'} size="0.9em" />Details
	</button>
	{#if details}
		<div class="details">
			{#if shown}
				<StepDetails step={shown} {now} {onfile} />
			{:else}
				<p class="note">Nothing has happened yet.</p>
			{/if}
			<button class="raw-toggle note" aria-expanded={raw} onclick={showRaw}>
				<Icon name={raw ? 'chevron-down' : 'chevron-right'} size="0.9em" />Raw output
			</button>
			{#if raw}
				{#if lastCommand}<pre class="output mono">{lastCommand}</pre>{/if}
				<pre class="output mono">{(transcript ?? lines.join('\n')) || 'Nothing yet.'}</pre>
			{/if}
		</div>
	{/if}
</aside>

{#if menu}
	<Menu x={menu.x} y={menu.y} anchor={menu.anchor} items={levelItems} label="Level" onclose={() => (menu = null)} />
{/if}

<style>
	.agent-card {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px;
		min-height: 0;
		height: 100%;
	}

	.tabs {
		display: flex;
		gap: 6px;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.mark {
		display: inline-flex;
		color: var(--agent);
	}

	.name {
		flex: 1;
		font-size: var(--fs-title);
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* What it is doing now. When it stops for the person this is the one
	   tinted card, with what it asks and the buttons that answer it. */
	.now {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.now.asking {
		padding: 10px 12px;
		border-radius: var(--r-panel);
		background: var(--agent-soft);
		border: 1px solid var(--agent-edge);
		animation: rise-in var(--t-slow) var(--ease) both;
	}

	.sentence {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.strands {
		display: inline-flex;
		flex: none;
	}

	/* Everything that is asked, once it is asked for: it scrolls in its own
	   box, so the buttons over it stay where they are. */
	.asked {
		max-height: min(38vh, 300px);
		overflow: auto;
	}

	.findings {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.findings li,
	.what {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.where {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.said,
	.what .mono {
		overflow-wrap: anywhere;
	}

	.sentence.waiting {
		color: var(--accent);
	}

	.sentence.quiet {
		color: var(--warn);
	}

	.sentence.paused {
		color: var(--muted);
	}

	.doing {
		margin: -2px 0 0;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.quiet {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 6px 10px;
		border-radius: 10px;
		background: color-mix(in srgb, var(--warn) 14%, transparent);
	}

	.sentence.failed {
		color: var(--danger);
	}

	.meter {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.bar {
		flex: 1;
		height: 4px;
		border-radius: var(--r-pill);
		background: var(--soft);
		overflow: hidden;
	}

	.fill {
		display: block;
		height: 100%;
		background: var(--agent);
		transition: width var(--t-slow) var(--ease);
	}

	.controls {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.timeline {
		flex: 1;
		min-height: 160px;
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--soft);
		padding-top: 6px;
	}

	/* What is asked is open over it: the steps give way before the card overflows. */
	.timeline.tight {
		min-height: 72px;
	}

	.step {
		padding: 1px 0;
	}

	.line {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 5px 2px;
		text-align: left;
		border-radius: var(--r-row);
	}

	.line:hover {
		background: var(--hover);
	}

	.icon {
		display: inline-flex;
		color: var(--muted);
	}

	.step.done .icon {
		color: var(--ok);
	}

	.step.waiting .icon {
		color: var(--accent);
	}

	.step.failed .icon {
		color: var(--danger);
	}

	.step.running .icon {
		color: var(--agent);
	}

	.step.superseded .label {
		text-decoration: line-through;
		color: var(--muted);
	}

	.label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ended {
		margin: 6px 0;
		padding: 10px 12px;
		border-radius: var(--r-panel);
		background: var(--surface-2);
		border: 1px solid var(--soft);
	}

	.ended.failed {
		color: var(--danger);
	}

	.plan {
		margin: 0;
		padding-left: 18px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.plan li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
	}

	.acts {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.tell {
		display: flex;
		gap: 6px;
	}

	.tell .field {
		flex: 1;
		font-size: var(--fs-secondary);
	}

	.raw-toggle {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		align-self: flex-start;
	}

	.output {
		margin: 0;
		max-height: 220px;
		overflow: auto;
		padding: 8px 10px;
		border-radius: var(--r-field);
		background: var(--sunken);
		font-size: var(--fs-mono);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.line[aria-pressed='true'] {
		background: color-mix(in srgb, var(--ink) 7%, transparent);
	}

	/* Everything about a step, once it is asked for (FEAT-118). */
	.details {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-height: 0;
		overflow: auto;
		padding-top: 4px;
	}
</style>
