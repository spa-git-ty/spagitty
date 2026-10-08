<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { untrack } from 'svelte';
	import type { ReviewVerdict } from '$lib/types';
	import { sendable } from './agent-drafts';
	import { room } from './room.svelte';

	/**
	 * Finish review (FEAT-093): the verdict, the words for the pull request
	 * as a whole, and every pending comment written against this head, sent
	 * as one review. Nothing written in the room reaches the host before
	 * this, and this is what the host shows everybody, so it says what goes.
	 */

	interface Props {
		onclose: () => void;
		/**
		 * An agent at *Sign off* stops here (2.0): the card opens filled in
		 * with its summary and its suggested verdict, for the person to read,
		 * change and send.
		 */
		suggested?: { verdict: ReviewVerdict; summary: string } | null;
		/** Told when the review went, so the agent's assignment can finish. */
		onsent?: () => void;
	}

	let { onclose, suggested = null, onsent }: Props = $props();

	let verdict = $state<ReviewVerdict>(untrack(() => suggested?.verdict ?? 'comment'));
	untrack(() => {
		if (suggested && !room.body.trim()) room.setBody(suggested.summary);
	});
	const goes = $derived(sendable(room.currentDrafts));
	const count = $derived(goes.length);
	/** How many of them an agent drafted. */
	const drafted = $derived(goes.filter((draft) => draft.agent).length);
	const waiting = $derived(room.currentDrafts.length - count);
	const older = $derived(room.olderDrafts.length);
	/** A comment, or changes asked for, with nothing written says nothing. */
	const empty = $derived(verdict !== 'approve' && count === 0 && room.body.trim() === '');

	function going(n: number): string {
		if (n === 0) return 'No line comments go with it.';
		return n === 1 ? '1 line comment goes with it.' : `${n} line comments go with it.`;
	}

	async function send() {
		if (await room.finish(verdict)) {
			onsent?.();
			onclose();
		}
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="finish floating"
	role="dialog"
	aria-label="Finish review"
	tabindex="-1"
	onkeydown={(event) => {
		if (event.key === 'Escape') onclose();
	}}
>
	<div class="verdicts">
		<Chip active={verdict === 'comment'} onclick={() => (verdict = 'comment')}>Comment</Chip>
		<Chip active={verdict === 'approve'} onclick={() => (verdict = 'approve')}>Approve</Chip>
		<Chip active={verdict === 'requestChanges'} onclick={() => (verdict = 'requestChanges')}>Request changes</Chip>
	</div>
	<label for="finish-body" class="note">On the whole pull request</label>
	<textarea
		id="finish-body"
		rows="4"
		placeholder="Overall thoughts…"
		value={room.body}
		oninput={(event) => room.setBody(event.currentTarget.value)}
	></textarea>
	<p class="note">
		{going(count)}{#if drafted > 0}
			{drafted === count ? ' An agent drafted them.' : ` ${drafted} of them an agent drafted.`}{/if}{#if waiting > 0}
			{waiting === 1 ? ' 1 finding still waits for you and stays.' : ` ${waiting} findings still wait for you and stay.`}{/if}{#if older > 0}
			{older === 1 ? ' 1 written before the last push stays here.' : ` ${older} written before the last push stay here.`}{/if}
	</p>
	<div class="actions">
		<Btn onclick={onclose}>Cancel</Btn>
		<Btn primary disabled={empty || room.sending} onclick={send}>{room.sending ? 'Sending…' : 'Send review'}</Btn>
	</div>
</div>

<style>
	.finish {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		z-index: 5;
		width: 380px;
		padding: 14px;
		border-radius: var(--r-floating);
		display: flex;
		flex-direction: column;
		gap: 10px;
		outline: none;
	}

	.verdicts {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	textarea {
		border: 1px solid var(--pane-edge);
		background: var(--sunken);
		border-radius: var(--r-field);
		padding: 9px 12px;
		font-family: var(--read-font);
		font-size: var(--fs-secondary);
		line-height: 1.6;
		resize: vertical;
	}

	p {
		margin: 0;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
