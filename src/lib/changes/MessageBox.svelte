<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { changes } from '$lib/changes/store.svelte';
	import { describeSigningProblem as signingProblem } from '$lib/settings/describe';
	import Chip from '$lib/ui/Chip.svelte';
	import { onMount } from 'svelte';
	import HooksDialog from '$lib/hooks/HooksDialog.svelte';
	import { hooks } from '$lib/hooks/store.svelte';

	/**
	 * The commit bar, along the bottom of the screen (TASK-047).
	 *
	 * It was a well across the top of the diff — subject, divider, a three-line
	 * body, the amend chip — and it took up to 40% of the column whether or not
	 * anybody was typing in it, so the hunks being committed were pushed into
	 * the lower half. Most commits are a summary line and nothing else. So the
	 * bar is one line: the summary, a way to add a description, amend, and the
	 * Commit button, which the page hands in as `action`. The description opens
	 * under the summary when asked for, and stays open while it holds anything.
	 *
	 * The subject counter appears only once the line is long: git's own
	 * convention is 50 characters, and a number that is always on screen reads
	 * as a limit rather than as a warning.
	 *
	 * Signing is said here, before the button, rather than reported after a
	 * failure (FEAT-019).
	 */
	interface Props {
		/** The Commit button, owned by the page. */
		action?: Snippet;
	}

	let { action }: Props = $props();

	const SUBJECT_HINT = 50;

	const subject = $derived(changes.subject);
	const over = $derived(subject.length > SUBJECT_HINT);

	/** Asked for, or already written in: either way it is on screen. */
	let asked = $state(false);
	const describing = $derived(asked || changes.body.length > 0);

	const signing = $derived(changes.signing);

	/**
	 * What this commit will do about a signature, or null to say nothing.
	 * Silent when signing is off, which is the ordinary case.
	 */
	const willSign = $derived.by(() => {
		if (signing === null || !signing.enabled) return null;
		if (signing.problem) return { tone: 'warn' as const, text: signingProblem(signing.problem) };
		return {
			tone: 'note' as const,
			text: `This commit will be signed with ${signing.program}.`
		};
	});

	let bodyField = $state<HTMLTextAreaElement | null>(null);

	/**
	 * The hooks a commit here runs (FEAT-107): a chip that opens what they are,
	 * and one that skips them for the next commit. Said only when there are any.
	 */
	let showingHooks = $state(false);
	const commitHooks = $derived(hooks.info?.hooks.filter((hook) => hook.onCommit) ?? []);
	const hooksOff = $derived(hooks.info !== null && !hooks.info.enabled);

	onMount(() => {
		void hooks.load();
	});

	function describe() {
		asked = true;
		// After the field exists, so the focus lands in it.
		queueMicrotask(() => bodyField?.focus());
	}
</script>

{#if showingHooks}
	<HooksDialog onclose={() => (showingHooks = false)} />
{/if}

<div class="message">
	{#if willSign || changes.amend}
		<div class="notes">
			{#if willSign}
				<span class="note signing" class:warn={willSign.tone === 'warn'}>{willSign.text}</span>
			{/if}
			{#if changes.amend}
				<span class="note">This rewrites the last commit rather than adding to history.</span>
			{/if}
		</div>
	{/if}

	<div class="bar">
		<div class="field">
			<input
				class="subject"
				type="text"
				placeholder="Summary of this commit"
				value={subject}
				oninput={(event) => changes.setSubject(event.currentTarget.value)}
				aria-label="Commit subject"
			/>
			{#if over}
				<span class="mono muted count" title="git's convention is {SUBJECT_HINT} characters">
					{subject.length}
				</span>
			{/if}
		</div>

		{#if !describing}
			<button class="text-action" onclick={describe}>Add description</button>
		{/if}

		<Chip
			active={changes.amend}
			onclick={() => changes.setAmend(!changes.amend)}
			title="Replace the previous commit instead of adding one"
		>
			amend
		</Chip>

		{#if commitHooks.length > 0}
			<Chip
				active={changes.skipHooks || hooksOff}
				disabled={hooksOff}
				onclick={() => changes.setSkipHooks(!changes.skipHooks)}
				title={hooksOff
					? 'Hooks are off for this repository (Settings → Hooks)'
					: 'Skip the hooks for this commit only'}
			>
				skip hooks
			</Chip>
			<button
				class="text-action hooks"
				onclick={() => (showingHooks = true)}
				title="What runs when you commit"
			>
				{commitHooks.length === 1 ? '1 hook' : `${commitHooks.length} hooks`}
			</button>
		{/if}

		{@render action?.()}
	</div>

	{#if describing}
		<textarea
			class="body"
			rows="3"
			placeholder="Why, if the summary is not enough"
			value={changes.body}
			bind:this={bodyField}
			oninput={(event) => changes.setBody(event.currentTarget.value)}
			aria-label="Commit body"
		></textarea>
	{/if}
</div>

<style>
	/*
	 * A bar, not a well: it takes the height of what is in it — one line, until
	 * a description is asked for — so the diff above it keeps the screen.
	 */
	.message {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px 12px 12px;
	}

	.bar {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	/* The summary is the one field here, so it is the one filled well. */
	.field {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		height: 38px;
		padding: 0 14px;
		background: var(--sunken);
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		transition: border-color var(--t-fast) var(--ease);
	}

	.field:focus-within {
		border-color: color-mix(in srgb, var(--accent) 55%, var(--soft));
	}

	/* The inputs themselves are invisible: the well around them is the control.
	   `app.css` gives every input a border and a fill, so both come back off. */
	.subject {
		flex: 1;
		min-width: 0;
		background: transparent;
		border: none;
		box-shadow: none;
		padding: 0;
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-ui);
	}

	.subject:focus {
		outline: none;
		background: transparent;
		box-shadow: none;
	}

	.subject::placeholder,
	.body::placeholder {
		color: var(--placeholder);
	}

	/* Opens under the summary when asked for — the order a commit message is
	   read in. Capped, so a long description scrolls inside itself instead of
	   pushing the diff away. */
	.body {
		width: 100%;
		min-height: 4.2em;
		max-height: 12em;
		resize: vertical;
		padding: 10px 14px;
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		line-height: var(--lh-ui);
		background: var(--sunken);
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
	}

	.body:focus {
		outline: none;
		border-color: color-mix(in srgb, var(--accent) 55%, var(--soft));
		box-shadow: none;
	}

	.count {
		flex: none;
		color: var(--accent);
	}

	.text-action {
		flex: none;
		padding: 4px 10px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--accent);
		white-space: nowrap;
		transition: background var(--t-fast) var(--ease);
	}

	.text-action:hover {
		background: var(--accent-soft);
	}

	.notes {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 14px;
		padding: 0 4px;
	}

	/* A statement, not an alarm: signing being on is the ordinary case for
	   anyone who signs. The warning colour is kept for the case where it is on
	   and cannot work, which is the one worth interrupting for. */
	.signing.warn {
		color: var(--warn);
	}
</style>
