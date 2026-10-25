<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { createSearch, speedSearch } from '$lib/ui/speedsearch.svelte';
	import { terminal } from './store.svelte';

	/**
	 * The multiplexer's sessions on Spagitty's socket, and the windows of the
	 * one shown (FEAT-130). Only there when there is a multiplexer: with none,
	 * a terminal has no sessions to list, and the screen does not say so.
	 *
	 * *Follow repository* is what makes the terminal the open repository's:
	 * switching repository tabs switches to that repository's session, and
	 * makes it if it is not there.
	 */
	interface Props {
		/** Where a session is made, when attaching makes one. */
		cwd: string | null;
	}

	let { cwd }: Props = $props();

	const shell = $derived(
		(terminal.info?.shell ?? '').split(/[\\/]/).pop()?.replace(/\.exe$/i, '') ?? ''
	);
	const plural = (count: number, one: string) => (count === 1 ? one : `${one}s`);

	/**
	 * Typing narrows the sessions only once the panel has been clicked in
	 * (FEAT-158): on this screen letters are the shell's.
	 */
	const search = createSearch();
	const sessions = $derived(search.filter(terminal.sessions, (session) => session.name));
</script>

<aside
	class="panel"
	aria-label="Sessions"
	use:speedSearch={{
		search,
		label: 'sessions',
		primary: false,
		count: () => [sessions.length, terminal.sessions.length],
		onenter: () => sessions[0] && terminal.attach(sessions[0].name, cwd ?? '')
	}}
>
	<span class="note name">Sessions</span>
	<div class="list">
		{#each sessions as session (session.name)}
			<button
				class="session"
				class:on={session.name === terminal.current}
				aria-pressed={session.name === terminal.current}
				onclick={() => terminal.attach(session.name, cwd ?? '')}
			>
				<span class="mark"><Icon name="terminal" size={15} /></span>
				<span class="what">
					<span class="mono title">{session.name}</span>
					<span class="note small">
						{session.windows}
						{plural(session.windows, 'window')} · {session.attached ? 'attached' : 'detached'}
					</span>
				</span>
			</button>
		{:else}
			<span class="note small none">No session yet</span>
		{/each}
	</div>

	{#if terminal.windows.length > 0}
		<div class="hr" aria-hidden="true"></div>
		<span class="note name">Windows</span>
		<div class="list">
			{#each terminal.windows as window (window.index)}
				<button
					class="window mono"
					class:on={window.active}
					aria-pressed={window.active}
					onclick={() => terminal.act({ kind: 'selectWindow', index: window.index })}
				>
					<span>{window.index}</span><span class="muted" aria-hidden="true">·</span><span class="title"
						>{window.name}</span
					>
				</button>
			{/each}
		</div>
	{/if}

	<span class="grow"></span>
	<div class="follow">
		<span title="Switching repository switches to that repository's session">Follow repository</span>
		<Chip active={terminal.follow} onclick={() => terminal.setFollow(!terminal.follow)}>
			{terminal.follow ? 'on' : 'off'}
		</Chip>
	</div>
	{#if terminal.info?.mux}
		<span class="note small mono">{shell} · {terminal.info.mux.name}</span>
	{/if}
</aside>

<style>
	.panel {
		flex: 1 1 220px;
		max-width: 250px;
		min-width: 180px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: auto;
	}

	.name {
		padding: 2px 6px;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.session {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		border-radius: var(--r-button);
		text-align: left;
	}

	.session:hover,
	.window:hover {
		background: var(--hover);
	}

	.session.on,
	.window.on {
		background: var(--surface-2);
		box-shadow: var(--shadow-1);
	}

	.mark {
		flex: none;
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: var(--r-field);
		background: var(--sunken);
		color: var(--muted);
	}

	.what {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.title {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.session .title {
		font-weight: 600;
	}

	.window {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		border-radius: var(--r-field);
		text-align: left;
	}

	.none {
		padding: 6px;
	}

	.hr {
		margin: 6px 4px;
	}

	.grow {
		flex: 1;
	}

	.follow {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 0 4px;
		font-size: var(--fs-secondary);
	}
</style>
