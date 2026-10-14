<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { graph } from '$lib/graph/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import SessionsPanel from '$lib/terminal/SessionsPanel.svelte';
	import { REFRESH_MS, terminal } from '$lib/terminal/store.svelte';
	import { makeXterm } from '$lib/terminal/xterm';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';

	/**
	 * Terminal (1T, FEAT-130): a real terminal on the rail, so nobody leaves
	 * Spagitty for one.
	 *
	 * It runs the shell inside a multiplexer when one can be run — tmux, or
	 * psmux on Windows, on a socket of Spagitty's own — with one session for
	 * each repository, and it follows the repository that is open. With no
	 * multiplexer each tab is a plain terminal, and nothing says one is
	 * missing.
	 *
	 * While the terminal has the focus every key is the shell's, except
	 * `Ctrl+Shift+P` for the palette and `Ctrl+Shift+`` to leave.
	 */
	const info = $derived(repo.info);
	const mux = $derived(terminal.info?.mux ?? null);
	const key = $derived(terminal.current);
	const element = $derived(terminal.element(key));

	let pane = $state<HTMLDivElement | null>(null);

	terminal.use(makeXterm);
	terminal.onOpen((link) => {
		if (link.kind === 'commit') {
			graph.want(link.commit);
			void goto('/');
		} else {
			const query = new URLSearchParams({ path: link.path, line: String(link.line), rev: 'HEAD' });
			void goto(`/files?${query}`);
		}
	});

	/** This repository's terminal: on arrival, and when another is opened. */
	$effect(() => {
		const path = info?.path ?? null;
		void repo.generation;
		untrack(() => {
			terminal.forgetLinks();
			void terminal.enter(path);
		});
	});

	/** The terminal shown is put in the page; the one that was is taken out, and kept. */
	$effect(() => {
		const host = pane;
		const shown = element;
		if (!host) return;
		if (shown) {
			if (shown.parentElement !== host) host.replaceChildren(shown);
			untrack(() => {
				terminal.fit(key);
				terminal.focus(key);
			});
		} else {
			host.replaceChildren();
		}
	});

	onMount(() => {
		// The sessions and windows change under us: another terminal, a `tmux
		// new-window` typed in this one. Read while the screen is shown, and
		// not at all when it is not.
		const timer = setInterval(() => void terminal.refresh(), REFRESH_MS);
		const sized =
			typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(() => terminal.fit(terminal.current));
		if (pane) sized?.observe(pane);
		return () => {
			clearInterval(timer);
			sized?.disconnect();
		};
	});

	const cwd = $derived(info?.path ?? null);
</script>

<div class="screen">
	<header class="head">
		<span class="title">Terminal</span>
		<div class="tabs" role="tablist" aria-label="Terminals">
			{#each terminal.tabs as tab (tab.key)}
				<span class="tab" class:on={tab.key === key}>
					<button
						role="tab"
						class="mono"
						aria-selected={tab.key === key}
						title={tab.detached ? `${tab.title} — detached` : tab.title}
						onclick={() => terminal.select(tab.key, cwd)}>{tab.title}</button
					>
					{#if !terminal.multiplexed}
						<button class="shut" aria-label="Close {tab.title}" title="Close" onclick={() => terminal.close(tab.key)}>
							<Icon name="close" size={10} weight={2} />
						</button>
					{/if}
				</span>
			{/each}
			<button
				class="tab add"
				aria-label={terminal.multiplexed ? 'New window' : 'New terminal'}
				title={terminal.multiplexed ? 'New window' : 'New terminal'}
				onclick={() => terminal.add(cwd)}>+</button
			>
		</div>

		<span class="gap"></span>
		{#if mux}
			<span class="pill mono" title="{mux.path} · {mux.name} {mux.version}">
				<span class="dot" aria-hidden="true"></span>
				{mux.name}
				{mux.version}
				{#if mux.bundled}<span class="muted">bundled</span>{/if}
			</span>
			<button class="tool" aria-label="Split right" title="Split right" onclick={() => terminal.act({ kind: 'splitRight' })}>
				<Icon name="split-right" size={16} />
			</button>
			<button class="tool" aria-label="Split down" title="Split down" onclick={() => terminal.act({ kind: 'splitDown' })}>
				<Icon name="split-down" size={16} />
			</button>
			<Btn
				disabled={!terminal.has(key)}
				title="Let go of this session. It carries on, and can be attached to again"
				onclick={() => terminal.act({ kind: 'detach' })}>Detach</Btn
			>
		{/if}
	</header>

	<div class="body">
		{#if terminal.multiplexed}
			<SessionsPanel {cwd} />
		{/if}

		<section class="view" aria-label={key ? `Terminal ${key}` : 'Terminal'}>
			<div class="pane" bind:this={pane}></div>
			{#if terminal.failure}
				<p class="said note error">{terminal.failure}</p>
			{:else if !element && key && terminal.multiplexed}
				<div class="said">
					<span class="note"><span class="mono">{key}</span> is detached</span>
					<Btn primary quiet onclick={() => terminal.attach(key, cwd ?? '')}>Attach</Btn>
				</div>
			{:else if !element && terminal.info}
				<div class="said">
					<Btn primary quiet onclick={() => terminal.add(cwd)}>New terminal</Btn>
				</div>
			{/if}
		</section>
	</div>
</div>

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
		gap: 8px;
		padding: 12px 16px 10px;
	}

	.title {
		margin-right: 6px;
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.tabs {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}

	.tab {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		background-color: var(--surface-veil);
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.tab > button:first-child {
		padding: 3px 10px;
		border-radius: var(--r-pill);
	}

	.tab.on {
		border-color: color-mix(in srgb, var(--accent) 62%, transparent);
		background-color: color-mix(in srgb, var(--accent) 14%, var(--surface));
		color: var(--accent);
	}

	.tab.add {
		padding: 3px 10px;
		color: var(--muted);
	}

	.tab.add:hover,
	.tab:not(.on):hover {
		background-color: var(--hover);
	}

	.shut {
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		margin-right: 4px;
		border-radius: 50%;
		color: var(--muted);
	}

	.shut:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.gap {
		flex: 1;
	}

	.pill {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--ok);
	}

	.tool {
		display: grid;
		place-items: center;
		width: 34px;
		height: 30px;
		border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
		border-radius: var(--r-button);
		background-color: var(--surface-veil);
		box-shadow: var(--shadow-1);
	}

	.tool:hover {
		background-color: var(--hover);
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 12px 12px;
	}

	.view {
		position: relative;
		flex: 999 1 560px;
		min-width: 0;
		min-height: 0;
		display: flex;
		background: var(--bg);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.pane {
		flex: 1;
		min-width: 0;
		min-height: 0;
		padding: 10px 6px 6px 12px;
	}

	.said {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		margin: 0;
		text-align: center;
	}

	.error {
		color: var(--danger);
	}
</style>
