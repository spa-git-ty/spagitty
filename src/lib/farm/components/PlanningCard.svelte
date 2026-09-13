<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import { duration } from '../describe';

	/**
	 * A planning run, while it is running (BUG-021).
	 *
	 * Asking an agent to break a goal into tasks takes minutes, and until this
	 * the only sign of it was the word "Planning" on a chip. Nothing said how
	 * long it had been going, nothing said what the planner was doing, and there
	 * was no way to stop it — a person watching an empty screen cannot tell a
	 * planner that is reading the repository from one that died three minutes
	 * ago.
	 *
	 * So: how long, the last thing it said, and a way out.
	 *
	 * # Why this one has a timer
	 *
	 * The rest of the Farm screen is a function of events, deliberately. An
	 * elapsed time is the exception, because the thing that changes is the
	 * clock and no event will ever fire for it. One interval, one second, and it
	 * exists only while a planning run does.
	 */
	interface Props {
		/** What the planner has said so far, narrated. */
		lines: string[];
		/** When the run started, from the run record rather than from the screen. */
		startedMs: number | null;
		busy?: boolean;
		oncancel: () => void;
	}

	let { lines, startedMs, busy = false, oncancel }: Props = $props();

	let now = $state(Date.now());

	$effect(() => {
		if (startedMs === null) return;
		const tick = setInterval(() => (now = Date.now()), 1000);
		return () => clearInterval(tick);
	});

	const elapsed = $derived(startedMs === null ? null : duration(now - startedMs));

	/**
	 * The last line worth showing.
	 *
	 * The last line and not the first: a planner narrates as it goes, and what
	 * it is doing now is the question. Blank lines are skipped — an agent's
	 * output is full of them and a card that flickers to empty reads as a stall.
	 */
	const latest = $derived(
		[...lines].reverse().find((line) => line.trim().length > 0) ?? null
	);
</script>

<section class="planning" aria-live="polite">
	<span class="dot" aria-hidden="true"></span>
	<div class="what">
		<span class="title">
			Planning
			{#if elapsed}<span class="note">&nbsp;{elapsed}</span>{/if}
		</span>
		<span class="said mono">
			{latest ?? 'The planner has not said anything yet.'}
		</span>
	</div>
	<Btn danger quiet disabled={busy} onclick={oncancel}>Stop planning</Btn>
</section>

<style>
	.planning {
		flex: none;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 12px;
		background-color: var(--surface-veil);
		border-bottom: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
	}

	.dot {
		flex: none;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background-color: var(--accent);
		animation: breathe 2.4s var(--ease) infinite;
	}

	.what {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.title {
		font-weight: 600;
	}

	.note {
		color: var(--muted);
		font-weight: 400;
	}

	/* One line, whatever the planner said. The whole transcript is in the log. */
	.said {
		color: var(--muted);
		font-size: var(--fs-mono);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	@keyframes breathe {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.dot {
			animation: none;
		}
	}
</style>
