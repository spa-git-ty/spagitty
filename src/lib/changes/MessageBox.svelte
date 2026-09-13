<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { changes } from '$lib/changes/store.svelte';
	import { describeSigningProblem as signingProblem } from '$lib/settings/describe';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * Subject, a divider, then the body — the shape of a commit message rather
	 * than a single box people are expected to remember the convention for.
	 *
	 * The subject counter appears only once the line is long: git's own
	 * convention is 50 characters, and a number that is always on screen reads
	 * as a limit rather than as a warning.
	 *
	 * Signing is said here, before the button, rather than reported after a
	 * failure (FEAT-019). "commit failed" is a bad way to learn that gpg was
	 * never installed, and both of the things that stop a signature happening
	 * are knowable now.
	 */

	const SUBJECT_HINT = 50;

	const subject = $derived(changes.subject);
	const over = $derived(subject.length > SUBJECT_HINT);

	const signing = $derived(changes.signing);

	/**
	 * What this commit will do about a signature, or null to say nothing.
	 *
	 * Silent when signing is off, which is the ordinary case: a note saying
	 * "this will not be signed" on every commit in a repository that never signs
	 * is noise on every commit.
	 */
	const willSign = $derived.by(() => {
		if (signing === null || !signing.enabled) return null;
		if (signing.problem) return { tone: 'warn' as const, text: signingProblem(signing.problem) };
		return {
			tone: 'note' as const,
			text: `This commit will be signed with ${signing.program}.`
		};
	});
</script>

<div class="message">
	<div class="subject-row">
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

	<div class="hr"></div>

	<textarea
		class="body"
		rows="3"
		placeholder="Why, if the summary is not enough"
		value={changes.body}
		oninput={(event) => changes.setBody(event.currentTarget.value)}
		aria-label="Commit body"
	></textarea>

	{#if willSign}
		<div class="note signing" class:warn={willSign.tone === 'warn'}>{willSign.text}</div>
	{/if}

	<div class="row">
		<Chip
			active={changes.amend}
			onclick={() => changes.setAmend(!changes.amend)}
			title="Replace the previous commit instead of adding one"
		>
			amend the previous commit
		</Chip>
		{#if changes.amend}
			<span class="note">This rewrites the last commit rather than adding to history.</span>
		{/if}
	</div>
</div>

<style>
	/*
	 * The commit message is what this screen is for, so it is a well rather
	 * than a strip: a sunken surface the two fields sit inside, which is what
	 * says "type here" without a label saying it.
	 *
	 * It must not eat the hunks. `flex: none` plus WebKitGTK's default
	 * textarea height sized this well to the whole column at 100% zoom, and
	 * the diff only appeared after a zoom change forced leftover space. Cap
	 * it, and let it shrink, so the pane below always has a height.
	 */
	.message {
		flex: 0 1 auto;
		min-height: 0;
		max-height: 40%;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin: 8px;
		padding: 8px 10px;
		background: var(--sunken);
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
		box-shadow: none;
		transition: border-color var(--t-fast) var(--ease);
	}

	/* The whole well takes the focus ring when either field inside it has
	   focus, because the well is what a person is aiming at. */
	.message:focus-within {
		border-color: color-mix(in srgb, var(--accent) 55%, var(--soft));
	}

	.subject-row {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	/* The fields themselves are invisible: the well around them is the control.
	   `app.css` gives every input a border and a fill, so both come back off. */
	.subject,
	.body {
		width: 100%;
		background: transparent;
		border: none;
		box-shadow: none;
		padding-inline: 0;
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-ui);
		padding: 2px 0;
	}

	.subject:focus,
	.body:focus {
		outline: none;
		background: transparent;
		box-shadow: none;
	}

	.subject::placeholder,
	.body::placeholder {
		color: var(--placeholder);
	}

	.body {
		min-height: 3.2em;
		max-height: 12em;
		resize: vertical;
		font-size: var(--fs-secondary);
		line-height: var(--lh-ui);
	}

	.count {
		flex: none;
		color: var(--accent);
	}

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	/* A statement, not an alarm: signing being on is the ordinary case for
	   anyone who signs. The warning colour is kept for the case where it is on
	   and cannot work, which is the one worth interrupting for. */
	.signing {
		border-left: 1px solid var(--line);
		padding-left: 6px;
	}

	/* On and unable to work. The palette's amber, which is what the rest of the
	   application now uses for "this needs looking at but nothing is broken". */
	.signing.warn {
		border-left-color: var(--warn);
		color: var(--warn);
	}
</style>
