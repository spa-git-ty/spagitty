<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import Sheet from '$lib/ui/Sheet.svelte';
	import { hooks } from './store.svelte';

	/**
	 * A commit's hooks, as they run (FEAT-107): the same glass window as every
	 * dialog, the hooks it runs, a pill that says running, passed or failed,
	 * and everything git and the hooks print, a line at a time, following the
	 * end while it grows. It cannot be closed while git is still running.
	 */
	const run = $derived(hooks.run);
	let log = $state<HTMLPreElement | null>(null);
	let following = true;

	$effect(() => {
		void run?.lines.length;
		if (log && following) log.scrollTop = log.scrollHeight;
	});

	function scrolled() {
		if (!log) return;
		following = log.scrollTop + log.clientHeight >= log.scrollHeight - 8;
	}

	function onkeydown(event: KeyboardEvent) {
		if ((event.key === 'Escape' || event.key === 'Enter') && run?.status !== 'running') {
			event.preventDefault();
			hooks.close();
		}
	}

	const label = $derived(
		run?.status === 'running' ? 'Running' : run?.status === 'passed' ? 'Passed · committed' : 'Failed · nothing committed'
	);
</script>

{#if run}
	<Sheet labelledby="hook-run-title" {onkeydown}>
			<header class="head">
				<span class="tile tone-{run.status}" aria-hidden="true">
					<Icon name={run.status === 'failed' ? 'conflict' : run.status === 'passed' ? 'check' : 'terminal'} size="1.1em" weight={2} />
				</span>
				<div class="titles">
					<h2 class="title" id="hook-run-title">Hooks</h2>
					<span class="names">
						{#each run.names as name (name)}<span class="name mono">{name}</span>{/each}
					</span>
				</div>
				<span class="status tone-{run.status}">
					{#if run.status === 'running'}<Loader size="inline" label="Running" />{:else}<span class="pip" aria-hidden="true"></span>{/if}
					{label}
				</span>
			</header>

			<pre class="log mono" bind:this={log} onscroll={scrolled} aria-live="polite">{#if run.lines.length === 0}<span class="quiet">{run.status === 'running' ? 'Waiting for the first line…' : 'The hooks printed nothing.'}</span>{:else}{run.lines.join('\n')}{/if}</pre>

			{#if run.status === 'failed' && run.error && run.lines.length === 0}
				<p class="note error">{run.error}</p>
			{/if}

			<div class="actions">
				<Btn primary disabled={run.status === 'running'} onclick={() => hooks.close()}>
					{run.status === 'running' ? 'Running…' : 'Close'}
				</Btn>
			</div>
	</Sheet>
{/if}

<style>
	.head {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.tile {
		--tone: var(--accent);
		width: 34px;
		height: 34px;
		border-radius: 11px;
		display: grid;
		place-items: center;
		flex: none;
		color: var(--tone);
		background: color-mix(in srgb, var(--tone) 16%, transparent);
	}

	.titles {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.names {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.name {
		font-size: var(--fs-mono);
		padding: 1px 8px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
		color: var(--muted);
	}

	.status {
		--tone: var(--accent);
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--tone) 45%, transparent);
		color: var(--tone);
		font-size: var(--fs-secondary);
		white-space: nowrap;
		flex: none;
	}

	.tone-passed {
		--tone: var(--ok);
	}

	.tone-failed {
		--tone: var(--danger);
	}

	.pip {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--tone);
	}

	.log {
		margin: 0;
		min-height: 180px;
		max-height: 52vh;
		overflow: auto;
		padding: 12px 14px;
		border-radius: 12px;
		background: var(--sunken);
		border: 1px solid var(--pane-edge);
		font-size: var(--fs-mono);
		line-height: 1.55;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.quiet {
		color: var(--muted);
	}

	.error {
		margin: 0;
		color: var(--danger);
	}

	.actions {
		display: flex;
		justify-content: flex-end;
	}
</style>
