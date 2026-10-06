<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { eventLine } from '../describe';
	import { PLANNING_TASK } from '../store.svelte';
	import type { RecordedEvent, Task } from '../types';

	/**
	 * The farm's log (FEAT-074).
	 *
	 * # What this replaces
	 *
	 * Six lines of text along the bottom of the screen, with no times, no
	 * scrollback, no filter, and no transcript — while the whole of what an
	 * agent had said sat in a store nothing asked. Supervising a farm is
	 * reading, and there was nothing here to read.
	 *
	 * # Two tabs, because they answer different questions
	 *
	 * **Activity** is what the farm did: tasks created, statuses moved,
	 * verifications run, merges landed. It is short and it is the record.
	 * **Transcript** is what one agent said, which is thousands of lines and
	 * belongs to one task at a time. Putting them in one list would drown the
	 * first in the second, which is exactly what the strip did by keeping the
	 * transcript out entirely.
	 *
	 * # Following, and holding
	 *
	 * The pane sticks to the newest line while the reader is at the bottom, and
	 * lets go the moment they scroll up — that is *following*, and it is a
	 * property of where the scrollbar is rather than a setting.
	 *
	 * **Hold** is different and is a setting: it freezes the list so a person
	 * can read a line that is still arriving, and counts what arrived while it
	 * was held. A log that reflows under the eye is unreadable at exactly the
	 * moment it matters.
	 */

	interface Props {
		/** The farm's activity, oldest first. */
		events: RecordedEvent[];
		tasks: Task[];
		/** The lines one task's agent has produced this session. */
		transcript: (task: string) => string[];
		/**
		 * Read a task's whole log back from disk (TASK-031).
		 *
		 * What the store holds is what this session heard, capped; the run's own
		 * log outlives both, and `farm_transcript` has existed since FEAT-073
		 * with nothing calling it.
		 */
		onwholeLog?: ((task: string) => Promise<string>) | null;
		/** The task whose transcript opens first, usually the selected one. */
		selected?: string | null;
		/** True while a planning run is in flight, so its transcript is offered. */
		planning?: boolean;
		collapsed: boolean;
		ontoggle: () => void;
	}

	let {
		events,
		tasks,
		transcript,
		selected = null,
		planning = false,
		onwholeLog = null,
		collapsed,
		ontoggle
	}: Props = $props();

	/** A whole log read from disk, by task, for as long as the drawer is open. */
	let whole = $state<Record<string, string[]>>({});
	let fetching = $state(false);

	async function readWholeLog(): Promise<void> {
		if (!subject || !onwholeLog || fetching) return;
		fetching = true;
		try {
			const text = await onwholeLog(subject);
			whole = { ...whole, [subject]: text.split('\n') };
		} catch {
			// The log may not exist yet — a task that has never run has none.
			// Saying nothing is right: the pane already says it has heard
			// nothing this session.
		} finally {
			fetching = false;
		}
	}

	type Tab = 'activity' | 'transcript';

	let tab = $state<Tab>('activity');
	let filter = $state<string>('');
	let held = $state(false);
	let following = $state(true);
	let pane = $state<HTMLElement | null>(null);

	/** What was on screen when Hold was pressed. */
	let frozen = $state<string[]>([]);

	/** Which task's transcript is being read. Follows the selection until touched. */
	let reading = $state<string | null>(null);
	const subject = $derived(reading ?? (planning ? PLANNING_TASK : selected));

	const shown = $derived(
		filter ? events.filter((event) => taskOf(event) === filter) : events
	);

	const lines = $derived.by(() => {
		if (held) return frozen;
		if (tab === 'activity') {
			return shown.map((event) => `${clock(event.atMs)}  ${eventLine(event)}`);
		}
		if (!subject) return [];
		// The whole log wins once it has been read: it contains what this
		// session heard and everything before it.
		return whole[subject] ?? transcript(subject);
	});

	/** Tasks worth offering as a filter: the ones that have said something. */
	const spoken = $derived(new Set(events.map(taskOf).filter((id): id is string => id !== null)));

	/** How many lines arrived while the list was held. */
	let heldAt = $state(0);
	const missed = $derived(held ? Math.max(0, current() - heldAt) : 0);

	function current(): number {
		if (tab === 'transcript') return subject ? transcript(subject).length : 0;
		return shown.length;
	}

	function taskOf(event: RecordedEvent): string | null {
		return 'task' in event ? (event.task as string) : null;
	}

	/** `14:02:11`, or nothing at all for an event recorded before times existed. */
	function clock(ms: number): string {
		if (!ms) return '        ';
		const at = new Date(ms);
		return [at.getHours(), at.getMinutes(), at.getSeconds()]
			.map((part) => String(part).padStart(2, '0'))
			.join(':');
	}

	function hold(): void {
		if (held) {
			held = false;
			following = true;
			return;
		}
		frozen = lines;
		heldAt = current();
		held = true;
	}

	/**
	 * Following is where the scrollbar is, not a preference.
	 *
	 * Four pixels of slack, because a pane scrolled to the bottom is often a
	 * fraction of a pixel short of it and a reader who never scrolled should
	 * not have to notice that.
	 */
	function onscroll(): void {
		if (!pane) return;
		following = pane.scrollHeight - pane.scrollTop - pane.clientHeight < 4;
	}

	$effect(() => {
		// Re-runs when the lines change; `following` is read, not written.
		void lines.length;
		if (!pane || !following || held) return;
		pane.scrollTop = pane.scrollHeight;
	});

	async function copy(): Promise<void> {
		try {
			await navigator.clipboard.writeText(lines.join('\n'));
		} catch {
			// No clipboard permission. Nothing is lost; the text is on screen.
		}
	}
</script>

<section class="drawer" class:collapsed>
	<header class="bar">
		<button
			class="tab"
			class:on={tab === 'activity'}
			onclick={() => {
				tab = 'activity';
				held = false;
			}}
		>
			Activity
		</button>
		<button
			class="tab"
			class:on={tab === 'transcript'}
			onclick={() => {
				tab = 'transcript';
				held = false;
			}}
		>
			Transcript
		</button>

		{#if tab === 'activity'}
			<select class="picker" bind:value={filter} title="Show one task only">
				<option value="">All tasks</option>
				{#each tasks.filter((task) => spoken.has(task.id)) as task (task.id)}
					<option value={task.id}>{task.id}</option>
				{/each}
			</select>
		{:else}
			<select class="picker" bind:value={reading} title="Whose transcript to read">
				{#if planning}
					<option value={PLANNING_TASK}>Planning</option>
				{/if}
				{#each tasks as task (task.id)}
					<option value={task.id}>{task.id}</option>
				{/each}
			</select>
		{/if}

		<span class="gap"></span>

		{#if missed > 0}
			<span class="missed note">{missed} new</span>
		{/if}
		<button class="control" class:on={held} onclick={hold} title={held ? 'Resume' : 'Hold'}>
			{held ? '▶' : '❚❚'}
		</button>
		{#if tab === 'transcript' && subject && onwholeLog && !whole[subject]}
			<button class="control" onclick={readWholeLog} disabled={fetching}>
				{#if fetching}<Loader size="inline" label="Reading…" />{:else}Whole log{/if}
			</button>
		{/if}
		<button class="control" onclick={copy} title="Copy what is shown">Copy</button>
		<button
			class="control"
			onclick={ontoggle}
			title={collapsed ? 'Show the log' : 'Hide the log'}
			aria-expanded={!collapsed}
		>
			<Icon name={collapsed ? 'chevron-up' : 'chevron-down'} />
		</button>
	</header>

	{#if !collapsed}
		<!--
			A scrollable region has to be reachable from the keyboard or its
			content is unreadable without a pointer, which is what the rule
			below is protecting against rather than causing here.
		-->
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<div class="pane mono" bind:this={pane} {onscroll} tabindex="0" role="log">
			{#each lines as line, index (index)}
				<div class="line">{line}</div>
			{/each}
			{#if lines.length === 0}
				<p class="note empty">
					{tab === 'activity'
						? 'Nothing has happened yet.'
						: subject
							? 'This task has not said anything this session.'
							: 'Pick a task to read what its agent said.'}
				</p>
			{/if}
		</div>
		<footer class="foot">
			<span class="note">{lines.length} lines</span>
			<span class="gap"></span>
			<span class="note follow" class:on={following && !held}>
				{held ? 'held' : following ? 'following' : 'scrolled back'}
			</span>
		</footer>
	{/if}
</section>

<style>
	.drawer {
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
		height: var(--farm-log-h);
		background-color: var(--chrome-veil);
		border-top: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		/*
		 * The drawer opening is the one animation on this screen with a size
		 * change in it. It is short, and it is on height alone — a drawer that
		 * slides as well as grows reads as two things moving.
		 */
		transition: height var(--t-slow) var(--ease);
	}

	.drawer.collapsed {
		height: auto;
	}

	.bar {
		flex: none;
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 8px;
	}

	.gap {
		flex: 1;
	}

	.tab {
		padding: 3px 9px;
		border: 1px solid transparent;
		border-radius: var(--r-pill);
		background-color: transparent;
		color: var(--muted);
		font-size: var(--fs-secondary);
		transition:
			color var(--t-fast) var(--ease),
			background-color var(--t-fast) var(--ease);
	}

	.tab:hover {
		color: var(--ink);
	}

	.tab.on {
		color: var(--ink);
		background-color: var(--selection);
		border-color: color-mix(in srgb, var(--accent) 35%, transparent);
	}

	.picker {
		max-width: 160px;
		padding: 2px 6px;
		border: 1px solid var(--soft);
		border-radius: var(--r-row);
		background-color: var(--surface-veil);
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.control {
		padding: 2px 8px;
		border: 1px solid transparent;
		border-radius: var(--r-row);
		background-color: transparent;
		color: var(--muted);
		font-size: var(--fs-mono);
		display: inline-flex;
		align-items: center;
	}

	.control:hover {
		color: var(--ink);
		background-color: var(--stripe);
	}

	.control.on {
		color: var(--accent);
	}

	/* What arrived while the list was held, so holding is never losing. */
	.missed {
		padding: 1px 7px;
		border-radius: var(--r-pill);
		background-color: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
		font-size: var(--fs-mono);
	}

	.pane {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		overflow-x: auto;
		padding: 2px 10px 6px;
		font-size: var(--fs-mono);
		line-height: 1.55;
		scrollbar-gutter: stable;
	}

	.line {
		white-space: pre;
		color: var(--ink);
	}

	/*
	 * A new line arrives rather than appearing.
	 *
	 * 120ms and no movement of anything already on screen: the animation is on
	 * the incoming row's own opacity and a two-pixel rise, so the lines above it
	 * — the ones being read — do not shift.
	 */
	.line {
		animation: arrive var(--t-fast) var(--ease);
	}

	@keyframes arrive {
		from {
			opacity: 0;
			transform: translateY(2px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}

	.empty {
		margin: 6px 0 0;
	}

	.foot {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 2px 10px 5px;
		font-size: var(--fs-mono);
	}

	.follow.on {
		color: var(--accent);
	}

	@media (prefers-reduced-motion: reduce) {
		.drawer,
		.line {
			transition: none;
			animation: none;
		}
	}
</style>
