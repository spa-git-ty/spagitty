<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { relativeTime } from '$lib/format';
	import { search } from '$lib/search/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import type { NotBlamable } from '$lib/types';

	/**
	 * Who last touched each line.
	 *
	 * A refusal is stated rather than rendered as an empty list — an empty list
	 * reads as a file nobody has ever touched, which is a different and much
	 * stranger claim.
	 */
	const REFUSALS: Record<NotBlamable, string> = {
		binary: 'This is a binary file. There are no lines to attribute.',
		tooLarge: 'This file is too large to blame. Read it in an editor instead.',
		notAFile: 'No such file at that revision — it may have been added later, or be a directory.',
		empty: 'This file has no lines.'
	};

	let file = $state('');
	let revision = $state('');

	function submit(event: SubmitEvent) {
		event.preventDefault();
		search.loadBlame(file, revision);
	}

	/** A run of lines from one commit is drawn once, the way git blame reads. */
	const groups = $derived.by(() => {
		const lines = search.blame?.lines ?? [];
		const out: Array<{ commit: string; lines: typeof lines }> = [];
		for (const line of lines) {
			const last = out[out.length - 1];
			if (last && last.commit === line.commit) last.lines.push(line);
			else out.push({ commit: line.commit, lines: [line] });
		}
		return out;
	});
</script>

<section class="strip">
	<form class="ask" onsubmit={submit}>
		<label class="field">
			<span class="note">File</span>
			<input bind:value={file} placeholder="path" />
		</label>
		<label class="field short">
			<span class="note">At</span>
			<input bind:value={revision} placeholder="HEAD" />
		</label>
		<Btn disabled={file.trim() === '' || search.blaming}>Blame</Btn>
	</form>

	<div class="body">
		{#if search.blaming}
			<Loader label="Reading…" />
		{:else if search.blameError}
			<p class="note error state">{search.blameError}</p>
		{:else if search.blame?.refused}
			<p class="note state">{REFUSALS[search.blame.refused]}</p>
		{:else if search.blame}
			<div class="head note">
				{search.blamePath} at
				<span class="mono">{search.blame.revision.slice(0, 7)}</span>
			</div>
			{#each groups as group (group.lines[0].line)}
				<div class="group">
					<div class="who note">
						<span class="mono">{group.lines[0].short}</span>
						{group.lines[0].authorName} · {relativeTime(group.lines[0].time)}
						{#if group.lines[0].sourcePath}
							· was <span class="mono">{group.lines[0].sourcePath}</span>
						{/if}
					</div>
					<div class="summary note" title={group.lines[0].summary}>
						{group.lines[0].summary}
					</div>
					<ol class="lines mono">
						{#each group.lines as line (line.line)}
							<li><span class="number note">{line.line}</span>{line.text}</li>
						{/each}
					</ol>
				</div>
			{/each}
		{/if}
	</div>
</section>

<style>
	.strip {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.ask {
		flex: none;
		display: flex;
		align-items: flex-end;
		gap: 8px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		flex: 1;
	}

	.field.short {
		flex: 0 0 72px;
	}

	.field .note {
		font-size: var(--fs-mono);
		padding-left: 2px;
	}

	.field input {
		width: 100%;
		height: 30px;
		box-sizing: border-box;
		border: 1px solid var(--pane-edge);
		border-radius: 10px;
		padding: 0 10px;
		background: var(--sunken);
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: var(--fs-secondary);
	}

	.field input:focus {
		outline: none;
		border-color: var(--accent);
	}

	.ask :global(button) {
		height: 30px;
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}

	.state {
		margin: 0;
	}

	.head {
		margin-bottom: 8px;
	}

	/* One block per run of lines from the same commit, which is how a blame is
	   actually read: by change, not by line. */
	.group {
		border-left: 2px solid color-mix(in srgb, var(--accent) 45%, transparent);
		padding: 2px 0 2px 10px;
		margin-bottom: 10px;
	}

	.summary {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.lines {
		margin: 4px 0 0;
		padding: 0;
		list-style: none;
		font-size: var(--fs-secondary);
	}

	.lines li {
		display: flex;
		gap: 8px;
		white-space: pre;
		overflow: hidden;
	}

	.number {
		flex: none;
		min-width: 32px;
		text-align: right;
	}
</style>
