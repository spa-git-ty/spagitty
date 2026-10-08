<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import * as api from './api';
	import { agents } from './store.svelte';
	import { LEVELS, above, atGate, choiceWords, grouped, isLive, levelWords, meter, share, stepTime } from './levels';
	import type { Assignment, Proposal, Step } from './types';

	/**
	 * The Agent card (2.0): what an agent is doing, while it does it.
	 *
	 * Four layers, each quieter than the one before: one sentence that is
	 * always true; its presence on the work (drawn by the screen, not here);
	 * the timeline — finished steps folded to a line, the current one open, a
	 * gate drawn as the one tinted card; and the raw output, folded under it.
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
	}

	let { assignment, tabs, names, follow, onfollow, ongo, onaccept, onlast, onhide }: Props = $props();

	let now = $state(Math.floor(Date.now() / 1000));
	let open = $state<Record<number, boolean>>({});
	let raw = $state(false);
	let transcript = $state<string | null>(null);
	let told = $state('');
	let menu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	onMount(() => {
		const timer = setInterval(() => (now = Math.floor(Date.now() / 1000)), 1000);
		return () => clearInterval(timer);
	});

	const a = $derived(assignment);
	const live = $derived(isLive(a));
	const gate = $derived(atGate(a));
	const highest = $derived(agents.rules?.highest ?? 'signOff');
	const remote = $derived(a.agent.reach === 'remote');
	const budget = $derived(
		remote ? (agents.snapshot?.remote.find((r) => r.id === a.agent.id)?.tokensPerRun ?? null) : null
	);
	const tone = $derived(
		a.state === 'waiting' ? 'waiting' : a.state === 'failed' ? 'failed' : a.state === 'done' ? 'done' : live ? 'working' : 'stopped'
	);
	const steps = $derived(a.steps.filter((s) => s.kind.kind !== 'why'));
	const lines = $derived(agents.lines(a.id));

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
			reason: above(level.id, highest) ? `Capped at ${levelWords(highest).label} here` : undefined,
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

	const waitingProposals = $derived(gate?.proposals ?? []);
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

	<p class="sentence {tone}" aria-live="polite">
		<span class="state"></span>{a.pausing && live ? 'Pausing after this step' : a.sentence}
	</p>
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

	<div class="timeline">
		<VirtualRows items={steps} key={(step) => String(step.index)} estimate={(step) => (step.state === 'waiting' ? 150 : 34)} label="Timeline">
			{#snippet row(step)}
				<div class="step {step.state}">
					<button class="line" onclick={() => (open = { ...open, [step.index]: !open[step.index] })} aria-expanded={!!open[step.index] || step.state === 'running'}>
						<span class="icon"><Icon name={icon(step)} size="0.95em" weight={2} /></span>
						<span class="label">{step.label}</span>
						<span class="note mono">{stepTime(step, now)}</span>
					</button>
					{#if open[step.index] || step.state === 'running'}
						<ol class="events">
							{#each (step.state === 'running' ? step.events.slice(-4) : step.events) as event, i (i)}
								<li class:you={event.startsWith('You · ')}>{event}</li>
							{/each}
							{#each step.sent as path (path)}
								<li class="sent">sent {path}</li>
							{/each}
							{#each step.checks as check (check.command)}
								<li class:failed={!check.passed}>{check.command} · {check.passed ? 'passed' : 'failed'}</li>
							{/each}
						</ol>
					{/if}
					{#if step.state === 'waiting' && gate && gate.step.index === step.index}
						<div class="gate">
							{#if step.gate === 'plan'}
								{@const plan = [...a.proposals].reverse().find((p) => p.body.kind === 'plan')}
								{#if plan && plan.body.kind === 'plan'}
									{@const files = plan.body.files.map((f) => f.path)}
									<span>{plan.body.lookFor || 'The reading plan'}</span>
									<ol class="plan">
										{#each plan.body.files as item (item.path)}
											<li>
												<span class="mono">{item.path}</span>
												<Chip title="Leave {item.path} out" onclick={() => dropFromPlan(files, item.path)}><Icon name="close" size="0.8em" /></Chip>
											</li>
										{/each}
									</ol>
								{/if}
								<div class="acts">
									<Btn primary quiet onclick={() => send({ kind: 'continue' })}>Go on</Btn>
									<Btn onclick={() => redo(step)}>Redo with a note</Btn>
								</div>
							{:else if step.gate === 'send' || step.gate === 'land'}
								<span>{step.gate === 'send' ? 'Ready for you to send.' : 'Ready for you to land.'}</span>
								<div class="acts">
									<Btn primary quiet onclick={() => onlast?.()}>{step.gate === 'send' ? 'Finish review' : 'Complete merge'}</Btn>
								</div>
							{:else if step.gate === 'checks'}
								{@const failed = step.checks.find((c) => !c.passed)}
								<span>{failed ? `${failed.command} failed` : 'The checks passed.'}</span>
								{#if failed}<pre class="output">{failed.output.split('\n').slice(-12).join('\n')}</pre>{/if}
								<div class="acts">
									<Btn primary quiet onclick={() => send({ kind: 'continue' })}>Go on</Btn>
									<Btn onclick={() => send({ kind: 'redo', note: '' })}>Run again</Btn>
								</div>
							{:else}
								{#if waitingProposals.length}
									{#if waitingProposals.length === 1 && waitingProposals[0].body.kind === 'resolution'}
										<span>{waitingProposals[0].body.why || choiceWords(waitingProposals[0].body.choice, names ?? { a: 'A', b: 'B' })}</span>
									{:else if waitingProposals.length === 1 && waitingProposals[0].body.kind === 'verdict'}
										<span>{waitingProposals[0].body.summary}</span>
									{:else}
										<span>{waitingProposals.length} {waitingProposals.length === 1 ? 'finding' : 'findings'}</span>
									{/if}
								{:else}
									<span>Nothing to decide here.</span>
								{/if}
								<div class="acts">
									{#if waitingProposals.length && ongo && waitingProposals[0].body.kind !== 'verdict'}
										<Btn onclick={() => ongo?.(waitingProposals)}>{waitingProposals.length === 1 ? 'Go to it' : 'Go to them'}</Btn>
									{/if}
									{#if waitingProposals.length && onaccept}
										<Btn primary quiet onclick={() => accept(waitingProposals)}>
											{waitingProposals.length === 1 ? 'Accept' : waitingProposals.length === 2 ? 'Accept both' : 'Accept all'}
										</Btn>
									{/if}
									<Btn onclick={() => send({ kind: 'continue' })}>{waitingProposals.length ? 'Go on, decide later' : 'Go on'}</Btn>
									<Btn onclick={() => redo(step)}>Redo with a note</Btn>
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/snippet}
		</VirtualRows>
		{#if a.state === 'waiting' && !gate && a.reason}
			<div class="gate">
				<span>{a.reason}</span>
				<div class="acts">
					<Btn primary quiet onclick={() => send({ kind: 'carryOn' })}>Carry on</Btn>
					<Btn onclick={() => send({ kind: 'stop' })}>Stop</Btn>
				</div>
			</div>
		{/if}
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
			<Chip onclick={() => agents.forget(a.id)}>forget</Chip>
		</div>
	{/if}
	<button class="raw-toggle note" aria-expanded={raw} onclick={showRaw}>
		<Icon name={raw ? 'chevron-down' : 'chevron-right'} size="0.9em" />Raw output
	</button>
	{#if raw}
		{#if lastCommand}<pre class="output mono">{lastCommand}</pre>{/if}
		<pre class="output mono">{(transcript ?? lines.join('\n')) || 'Nothing yet.'}</pre>
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

	.sentence {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.state {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex: none;
		background: var(--muted);
	}

	.sentence.waiting {
		color: var(--accent);
	}

	.sentence.waiting .state {
		background: var(--accent);
	}

	.sentence.working .state {
		background: var(--agent);
		animation: breathe 1.6s var(--ease) infinite;
	}

	.sentence.done .state {
		background: var(--ok);
	}

	.sentence.failed {
		color: var(--danger);
	}

	.sentence.failed .state {
		background: var(--danger);
	}

	@keyframes breathe {
		50% {
			opacity: 0.35;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.sentence.working .state {
			animation: none;
		}
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

	.events {
		list-style: none;
		margin: 0 0 4px 26px;
		padding: 0;
		font-size: var(--fs-secondary);
		color: var(--muted);
		display: flex;
		flex-direction: column;
		gap: 2px;
		overflow-wrap: anywhere;
	}

	.events .you {
		color: var(--ink);
	}

	.events .failed {
		color: var(--danger);
	}

	.gate,
	.ended {
		margin: 6px 0;
		padding: 10px 12px;
		border-radius: var(--r-panel);
		background: var(--agent-soft);
		border: 1px solid var(--agent-edge);
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.ended {
		background: var(--surface-2);
		border-color: var(--soft);
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
</style>
