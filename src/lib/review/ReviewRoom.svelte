<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { codeStack, PLAIN, reading, uiStack } from '$lib/reading.svelte';
	import { CHECK_LABELS, requests } from '$lib/requests/store.svelte';
	import { scale } from '$lib/scale.svelte';
	import FinishReview from './FinishReview.svelte';
	import RoomConversation from './RoomConversation.svelte';
	import RoomDiff from './RoomDiff.svelte';
	import RoomFiles from './RoomFiles.svelte';
	import RoomPill from './RoomPill.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { room } from './room.svelte';
	import { review } from './store.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import AgentCard from '$lib/agents/AgentCard.svelte';
	import AssignPopover from '$lib/agents/AssignPopover.svelte';
	import { agents } from '$lib/agents/store.svelte';
	import { isLive, readingNow } from '$lib/agents/levels';
	import type { Assigned, Proposal } from '$lib/agents/types';
	import * as agentWork from './agent.svelte';

	/**
	 * The review room (FEAT-087, FEAT-091): one pull request, read file by
	 * file — the files it touches on the left with their viewed ticks, the
	 * diff in the middle with the review pill over it, the conversation on
	 * the right.
	 */
	const opened = $derived(review.room);
	const pr = $derived(opened?.pr ?? null);

	const HOSTS = { gitHub: 'GitHub', gitLab: 'GitLab', bitbucket: 'Bitbucket' } as const;
	const host = $derived(requests.repo ? HOSTS[requests.repo.kind] : null);

	$effect(() => {
		const now = opened;
		if (now) untrack(() => room.load(now.pr, now.key));
		return () => room.leave();
	});

	const viewed = $derived(room.viewedCount);
	/** The Finish review card is open (FEAT-093). */
	let finishing = $state(false);

	/**
	 * The Conversation card can be put away (BUG-054): the diff takes its room,
	 * and a tab at the edge, with the open thread count, brings it back.
	 * Remembered on this machine.
	 */
	const HIDDEN_KEY = 'spagitty.review.conversationHidden';
	let conversationHidden = $state(readHidden());

	function readHidden(): boolean {
		try {
			return localStorage.getItem(HIDDEN_KEY) === '1';
		} catch {
			return false;
		}
	}

	function setConversationHidden(next: boolean) {
		conversationHidden = next;
		try {
			localStorage.setItem(HIDDEN_KEY, next ? '1' : '0');
		} catch {
			// A private window: the choice lasts the session.
		}
	}

	const openThreads = $derived(room.threads.filter((thread) => !thread.resolved).length);

	// ── Agents (2.0) ──────────────────────────────────────────────────────
	// With no agent set up, none of this draws anything: the room is the
	// 1.3 room.

	const offered = $derived(agents.usable('review').length > 0);
	const assignment = $derived(
		opened ? agents.forReview(opened.key.owner, opened.key.name, opened.key.number) : null
	);
	const working = $derived(assignment !== null && isLive(assignment));
	let assigning = $state(false);
	let starting = $state(false);
	/** Which of the right-hand card's two tabs shows. */
	let side = $state<'conversation' | 'agent'>('conversation');
	/** Move the room with the agent: off by default, the reader is never moved. */
	let following = $state(false);
	let lastSeen = $state<string | null>(null);

	// A new assignment brings its tab forward once.
	$effect(() => {
		const id = assignment?.id ?? null;
		if (id && id !== untrack(() => lastSeen)) {
			lastSeen = id;
			side = 'agent';
			setConversationHidden(false);
		}
	});

	$effect(() => {
		const path = assignment && following ? readingNow(assignment) : null;
		if (path && path !== untrack(() => room.selected)) room.select(path);
	});

	// The author pushed while an agent works: it pauses and says so.
	$effect(() => {
		const fetched = room.head?.head;
		if (fetched && assignment && working && opened) {
			untrack(() => agentWork.watchHeads([{ ...opened.pr, headSha: fetched }]));
		}
	});

	async function assign(chosen: Assigned) {
		if (!opened) return;
		starting = true;
		const started = await agentWork.assign(opened.pr, opened.key, chosen);
		starting = false;
		if (started) assigning = false;
	}

	/** At Sign off the Finish review card opens filled in. */
	const suggested = $derived.by(() => {
		if (!assignment || assignment.state !== 'waiting') return null;
		const at = assignment.steps.find((s) => s.state === 'waiting');
		if (at?.gate !== 'send') return null;
		const verdict = [...assignment.proposals].reverse().find((p) => p.body.kind === 'verdict');
		return verdict && verdict.body.kind === 'verdict'
			? { verdict: verdict.body.verdict, summary: verdict.body.summary }
			: null;
	});

	function sent() {
		if (assignment && assignment.state === 'waiting') {
			void agents.control(assignment.id, { kind: 'acted', ok: true });
		}
	}

	function goTo(proposals: Proposal[]) {
		if (!assignment) return;
		const first = room.drafts.find((d) => proposals.some((p) => d.id === agentWork.draftId(assignment.id, p.id)));
		if (first) void room.jumpToDraft(first);
	}
	const total = $derived(room.files.length);

	/**
	 * `Aa`: code and comments in the reading set Settings › Reading chose, or
	 * as they were set before it. Comments in the reading set take the
	 * interface face chosen there, or Atkinson Hyperlegible when the
	 * interface is left on the desktop's.
	 */
	const readingStyle = $derived.by(() => {
		if (room.readingSet) {
			const face = reading.current.uiFont === 'system' ? 'atkinson' : reading.current.uiFont;
			return `--read-font: ${uiStack(face)};`;
		}
		const size = Math.round(PLAIN.size * scale.zoom * scale.text * 100) / 100;
		return [
			`--code-font: ${codeStack(PLAIN.codeFont)}`,
			`--fs-code: ${size}px`,
			`--code-lh: ${PLAIN.lineHeight}`,
			`--code-ls: ${PLAIN.letterSpacing}em`,
			'--read-font: var(--font-ui)',
			'--read-size: var(--fs-secondary)'
		].join('; ');
	});
</script>

{#snippet sideTabs()}
	<div class="side-tabs" role="tablist" aria-label="Side card">
		<Chip active={side === 'conversation'} onclick={() => (side = 'conversation')}>Conversation {openThreads}</Chip>
		<Chip active={side === 'agent'} onclick={() => (side = 'agent')}>
			<Icon name="agent" size="0.95em" weight={2} />Agent
		</Chip>
	</div>
{/snippet}

{#if pr}
	<div class="screen" style={readingStyle}>
		<header class="head">
			<button class="back" onclick={() => review.close()}>
				<Icon name="chevron-left" size="0.9em" weight={2} />Review
			</button>
			<span class="name">
				<span class="number mono">#{pr.number}</span>
				<span class="title">{pr.title}</span>
			</span>
			{#if total > 0}
				<span class="progress">
					<span class="note">{viewed} of {total} viewed</span>
					<span class="bar"><span class="fill" style:width="{(viewed / total) * 100}%"></span></span>
				</span>
			{/if}
			{#if offered && !working}
				<span class="assign-anchor">
					<Btn disabled={room.phase !== 'ready'} onclick={() => (assigning = !assigning)}>
						<Icon name="agent" size="1em" />Assign an agent…
					</Btn>
					{#if assigning}
						<div class="below">
							<AssignPopover job="review" busy={starting} onassign={assign} oncancel={() => (assigning = false)} />
						</div>
					{/if}
				</span>
			{/if}
			<Btn disabled={review.checkingOut !== null} onclick={() => review.checkOut(pr)}>
				<Icon name="branch" size="1em" />Check out branch
			</Btn>
			<span class="finish-anchor">
				<Btn primary disabled={room.phase !== 'ready'} onclick={() => (finishing = !finishing)}>
					Finish review · {room.currentDrafts.length}
				</Btn>
				{#if finishing}<FinishReview {suggested} onsent={sent} onclose={() => (finishing = false)} />{/if}
			</span>
		</header>
		<div class="meta note">
			<span><span class="who">{pr.authorName}</span> wants to merge</span>
			<span class="tag mono">{pr.sourceBranch}</span>
			<span>into</span>
			<span class="tag mono">{pr.targetBranch}</span>
			{#if pr.checks}
				<span>·</span>
				<span class="checks {pr.checks}">{CHECK_LABELS[pr.checks]}</span>
			{/if}
			{#if host}<span>·</span><span>{host}</span>{/if}
		</div>

		<div class="body">
			<RoomFiles {assignment} />
			<Splitter panel="roomFiles" label="Resize the files" />
			<section class="diff" aria-label="Changes">
				{#if room.fallback}
					<p class="fallback note" title={room.fallback}>
						From {host ?? 'the host'}'s patch: the pull request could not be fetched, so there is no
						whole file.
					</p>
				{/if}
				{#if room.phase === 'reading'}
					<Loader label="Fetching #{pr.number}…" />
				{:else if room.phase === 'failed'}
					<p class="pad note error">{room.error}</p>
				{:else if room.phase === 'ready' && total === 0}
					<p class="pad note">This pull request changes no files.</p>
				{:else if room.phase === 'ready'}
					<RoomDiff />
					<RoomPill onfinish={() => (finishing = true)} />
				{/if}
			</section>
			{#if conversationHidden}
				<button
					class="conversation-tab"
					onclick={() => setConversationHidden(false)}
					title="Show the conversation"
				>
					<Icon name="chevron-left" size="0.95em" weight={2.2} />
					<span class="tab-label">Conversation</span>
					{#if openThreads > 0}<span class="tab-count">{openThreads}</span>{/if}
				</button>
			{:else}
				<Splitter panel="roomConversation" label="Resize the conversation" />
				{#if assignment && side === 'agent'}
					<div class="agent-side">
						<AgentCard
							{assignment}
							tabs={sideTabs}
							follow={following}
							onfollow={(on) => (following = on)}
							ongo={goTo}
							onaccept={(proposals) => agentWork.acceptAll(assignment, proposals)}
							onlast={() => (finishing = true)}
							onhide={() => setConversationHidden(true)}
						/>
					</div>
				{:else}
					<RoomConversation onhide={() => setConversationHidden(true)} tabs={assignment ? sideTabs : undefined} />
				{/if}
			{/if}
		</div>
	</div>
{/if}

<style>
	.assign-anchor {
		position: relative;
	}

	/* The popover opens under the header button rather than over it. */
	.below :global(.assign) {
		top: calc(100% + 8px);
		bottom: auto;
	}

	.agent-side {
		width: var(--room-conversation-w);
		flex: none;
		display: flex;
		min-height: 0;
		margin: 0 0 10px;
	}

	.agent-side :global(.agent-card) {
		flex: 1;
		border-radius: var(--r-floating);
	}

	.side-tabs {
		display: flex;
		gap: 6px;
		margin-right: auto;
	}

	/* The put-away Conversation card, as a tab down the room's right edge. */
	.conversation-tab {
		flex: none;
		width: 30px;
		margin: 0 0 10px 6px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 10px 0;
		border-radius: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		color: var(--muted);
	}

	.conversation-tab:hover {
		color: var(--ink);
		background: var(--hover);
	}

	.tab-label {
		writing-mode: vertical-rl;
		font-size: var(--fs-secondary);
	}

	.tab-count {
		min-width: 18px;
		height: 18px;
		padding: 0 4px;
		border-radius: var(--r-pill);
		display: grid;
		place-items: center;
		font-size: var(--fs-mono);
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
	}

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
		gap: 12px;
		padding: 14px 18px 4px;
	}

	.back {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 4px 12px 4px 8px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		background-color: var(--surface-veil);
		font-size: var(--fs-secondary);
	}

	.back:hover {
		background-color: var(--hover);
	}

	.name {
		flex: 1 1 380px;
		min-width: 0;
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.number {
		color: var(--muted);
		font-size: var(--fs-ui);
	}

	.title {
		font-size: var(--fs-title);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.finish-anchor {
		position: relative;
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
		transition: width var(--t-fast) var(--ease);
	}

	.meta {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		padding: 0 18px 12px 112px;
	}

	.who {
		color: var(--ink);
	}

	.tag {
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 2px 9px;
		background-color: var(--surface-veil);
	}

	.checks.passing {
		color: var(--ok);
	}

	.checks.failing {
		color: var(--danger);
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 8px;
		padding: 0 10px;
	}

	.diff {
		flex: 1;
		min-width: 0;
		position: relative;
		display: flex;
		flex-direction: column;
	}

	.pad {
		padding: 10px 16px;
		margin: 0;
	}

	.error {
		color: var(--danger);
	}

	.fallback {
		margin: 0 8px 8px;
		padding: 6px 12px;
		border-radius: var(--r-panel);
		background: color-mix(in srgb, var(--warn) 12%, transparent);
	}
</style>
