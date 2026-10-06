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
	import { room } from './room.svelte';
	import { review } from './store.svelte';

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
			<Btn disabled={review.checkingOut !== null} onclick={() => review.checkOut(pr)}>
				<Icon name="branch" size="1em" />Check out branch
			</Btn>
			<span class="finish-anchor">
				<Btn primary disabled={room.phase !== 'ready'} onclick={() => (finishing = !finishing)}>
					Finish review · {room.currentDrafts.length}
				</Btn>
				{#if finishing}<FinishReview onclose={() => (finishing = false)} />{/if}
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
			<RoomFiles />
			<Splitter panel="roomFiles" label="Resize the files" />
			<section class="diff" aria-label="Changes">
				{#if room.fallback}
					<p class="fallback note" title={room.fallback}>
						From {host ?? 'the host'}'s patch: the pull request could not be fetched, so there is no
						whole file.
					</p>
				{/if}
				{#if room.phase === 'reading'}
					<p class="pad note">Fetching #{pr.number}…</p>
				{:else if room.phase === 'failed'}
					<p class="pad note error">{room.error}</p>
				{:else if room.phase === 'ready' && total === 0}
					<p class="pad note">This pull request changes no files.</p>
				{:else if room.phase === 'ready'}
					<RoomDiff />
					<RoomPill onfinish={() => (finishing = true)} />
				{/if}
			</section>
			<Splitter panel="roomConversation" label="Resize the conversation" />
			<RoomConversation />
		</div>
	</div>
{/if}

<style>
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
