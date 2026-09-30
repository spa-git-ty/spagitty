<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import { commandLog, line, transcript } from '$lib/commandlog/store.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import type { ExecutedCommand } from '$lib/types';

	/**
	 * What Spagitty ran, as a drawer along the bottom of the shell.
	 *
	 * A drawer rather than a toast: a command worth showing is a command worth
	 * reading twice, and `Notice` deliberately holds one message at a time.
	 * Mounted by the shell, because an operation started on the Graph can finish
	 * after the user has navigated to Changes.
	 */

	const entries = $derived([...commandLog.entries].reverse());

	function failure(entry: ExecutedCommand): string | null {
		return entry.outcome.kind === 'failed' ? entry.outcome.stderr : null;
	}

	function status(entry: ExecutedCommand): string {
		switch (entry.outcome.kind) {
			case 'ok':
				return `${entry.durationMs} ms`;
			case 'started':
				return 'running';
			case 'failed':
				return entry.outcome.code === null ? 'killed' : `exit ${entry.outcome.code}`;
		}
	}

	function at(entry: ExecutedCommand): string {
		return new Date(entry.atMs).toLocaleTimeString();
	}

	async function copy(text: string, said: string): Promise<void> {
		try {
			await navigator.clipboard.writeText(text);
			notice.ok(said);
		} catch (error) {
			// A webview can refuse clipboard access outright, and a copy button
			// that silently does nothing is worse than one that says why.
			notice.failed('Could not copy', error);
		}
	}
</script>

{#if commandLog.open}
	<section class="drawer" aria-label="Git commands">
		<header class="bar">
			<h2 class="title">Git commands</h2>
			<span class="note count">{entries.length}</span>
			<div class="spacer"></div>
			<Btn
				disabled={entries.length === 0}
				onclick={() => copy(transcript(commandLog.entries), 'Copied the command log')}
			>
				Copy all
			</Btn>
			<Btn disabled={entries.length === 0} onclick={() => commandLog.clear()}>Clear</Btn>
			<button class="close" aria-label="Close" onclick={() => commandLog.hide()}>×</button>
		</header>

		<ol class="list">
			{#each entries as entry (entry.seq)}
				<li class="row" class:failed={entry.outcome.kind === 'failed'}>
					<span class="note when">{at(entry)}</span>
					<code class="line">{line(entry)}</code>
					<span class="note state">{status(entry)}</span>
					<button
						class="copy"
						aria-label="Copy this command"
						onclick={() => copy(line(entry), 'Copied the command')}
					>
						Copy
					</button>
					{#if failure(entry)}
						<pre class="note stderr">{failure(entry)}</pre>
					{/if}
				</li>
			{:else}
				<li class="empty note">
					Nothing has run yet.
				</li>
			{/each}
		</ol>

		<!--
			Said plainly rather than left to be inferred: an empty-looking log
			next to a screen full of history would otherwise read as Spagitty
			hiding what it did, when in fact reading history never runs `git` at
			all.
		-->
	</section>
{/if}

<style>
	.drawer {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 50;
		display: flex;
		flex-direction: column;
		max-height: min(46vh, 420px);
		background-color: var(--glass-thick);
		backdrop-filter: var(--blur-thick);
		-webkit-backdrop-filter: var(--blur-thick);
		/* Only the top edge is on screen — the panel is anchored to the bottom
		   and runs off both sides — so the lit edge is the whole edge (TASK-024). */
		border-top: 1px solid var(--glass-edge);
		border-radius: var(--r-floating) var(--r-floating) 0 0;
		animation: rise-in var(--t-enter-liquid) var(--spring-liquid);
		/* It slides up over whatever screen is open, so it takes the floating
		   shadow — cast upward, which is the direction it came from. */
		box-shadow: 0 -10px 30px color-mix(in srgb, var(--umbra) 14%, transparent);
	}

	.bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		background: color-mix(in srgb, var(--panel) 40%, transparent);
		border-bottom: 1px solid color-mix(in srgb, var(--line) 60%, transparent);
	}

	.title {
		font-size: var(--fs-secondary);
	}

	.spacer {
		flex: 1;
	}

	.close {
		line-height: 1;
		font-size: var(--fs-title);
		color: var(--muted);
	}

	.close:hover {
		color: var(--ink);
		background: var(--hover);
		border-radius: var(--r-field);
	}

	.list {
		flex: 1;
		overflow-y: auto;
		margin: 0;
		padding: 4px 0;
		list-style: none;
	}

	.row {
		display: grid;
		grid-template-columns: auto 1fr auto auto;
		align-items: baseline;
		gap: 8px;
		padding: 3px 10px;
	}

	.row:hover {
		background: var(--hover);
	}

	/* The command itself is the thing to select and paste, so it never wraps
	   into something that pastes differently from what ran. */
	.line {
		font-family: var(--font-mono);
		font-size: var(--fs-secondary);
		white-space: pre;
		overflow-x: auto;
		user-select: text;
	}

	.row.failed .line {
		color: var(--danger);
	}

	.stderr {
		grid-column: 2 / -1;
		margin: 2px 0 0;
		white-space: pre-wrap;
		word-break: break-word;
		user-select: text;
	}

	.copy {
		font-size: var(--fs-secondary);
		color: var(--muted);
		opacity: 0;
	}

	.row:hover .copy,
	.copy:focus-visible {
		opacity: 1;
	}

	.copy:hover {
		color: var(--ink);
	}

	.empty {
		padding: 10px;
	}

</style>
