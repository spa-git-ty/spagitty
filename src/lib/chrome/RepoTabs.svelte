<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	import * as api from '$lib/api';
	import { clone } from '$lib/clone/store.svelte';
	import { graph } from '$lib/graph/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { notice } from '$lib/ui/notice.svelte';
	import { workspace } from '$lib/workspace.svelte';
	import type { RepoSummary } from '$lib/types';
	import { worktrees } from '$lib/worktrees/store.svelte';
	import { worktreeModal } from '$lib/worktrees/modal.svelte';

	/**
	 * The repositories open as tabs.
	 *
	 * A tab is a place to go back to, not a live session — Spagitty's backend
	 * holds one repository at a time, and switching re-opens the one clicked and
	 * restores the screen and the selection it was left on. `workspace.svelte.ts`
	 * carries the reasoning; the honest consequence is that a switch costs a
	 * fresh walk, which is why the tab shows its repository as loading rather
	 * than pretending the graph is ready.
	 */

	const tabs = $derived(workspace.tabs);
	let menu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);
	let recents = $state<RepoSummary[]>([]);

	/** Where the current repository is right now, for remembering on the way out. */
	function here() {
		return { route: page.url.pathname, selected: graph.selectedId };
	}

	async function switchTo(path: string): Promise<void> {
		if (workspace.isActive(path)) return;

		const leaving = repo.info?.path;
		if (leaving) workspace.remember(leaving, here());

		const place = workspace.placeOf(path);
		const opened = await repo.open(path);
		if (!opened) return;

		// The route first, then the selection: the graph store holds a wanted id
		// across a walk it cannot see the end of, so it can be handed over before
		// the rows that contain it have arrived.
		if (place?.route && place.route !== page.url.pathname) await goto(place.route);
		if (place?.selected) graph.want(place.selected);
	}

	function closeTab(event: MouseEvent, path: string): void {
		event.stopPropagation();

		const wasActive = workspace.isActive(path);
		if (wasActive && repo.info?.path === path) workspace.remember(path, here());

		const next = workspace.close(path);
		if (!wasActive) return;

		// The tab that was showing has gone. Either something else takes its
		// place, or there is nothing left and the repository itself closes
		// (BUG-019).
		//
		// This used to be `if (wasActive && next)` with no other branch, so
		// closing the last tab took the strip away and left the repository open
		// behind it: the toolbar still naming it, the rail still counting its
		// branches, the graph still full of its commits, and no tab anywhere to
		// close a second time.
		if (next) void switchTo(next);
		else void repo.close();
	}

	async function openMenu(event: MouseEvent): Promise<void> {
		// Clicking the control again closes it — see BUG-018 and the `anchor`
		// note on `Menu`.
		if (menu) {
			menu = null;
			return;
		}
		const button = event.currentTarget as HTMLElement;
		const box = button.getBoundingClientRect();
		menu = { x: box.left, y: box.bottom + 2, anchor: button };

		try {
			recents = await api.recentRepos();
		} catch (error) {
			// The menu still offers Open and Clone; only the recent list is lost.
			notice.failed('Could not read the repository list', error);
			recents = [];
		}
	}

	const menuItems = $derived.by((): MenuItem[] => {
		const items: MenuItem[] = [
			{ id: 'open', label: 'Open repository…', run: () => void repo.choose() },
			{ id: 'clone', label: 'Clone…', run: () => clone.show() },
			// Off the rail since TASK-045; this is where it is reached from.
			{ id: 'all', label: 'All repositories', run: () => void goto('/repos') }
		];

		if (repo.info) {
			items.push({
				id: 'worktrees',
				label: 'Worktrees…',
				run: () => {
					void worktrees.fetch();
					worktreeModal.showManager();
				}
			});
		}

		// Only the ones not already open: a menu offering to open the tab you are
		// looking at teaches nothing.
		const unopened = recents.filter((entry) => !tabs.some((tab) => tab.path === entry.path));
		if (unopened.length > 0) {
			items.push({ heading: 'Recent' });
			for (const entry of unopened.slice(0, 8)) {
				items.push({
					id: entry.path,
					label: entry.name,
					note: entry.branch ?? undefined,
					disabled: !entry.present,
					reason: entry.present ? undefined : 'missing from disk',
					run: () => void switchTo(entry.path)
				});
			}
		}

		return items;
	});
</script>

<!--
	The tabs are a row of their own, below the title bar (FEAT-044). They were a
	passenger in that bar, squeezed by the program name and the window controls
	in the one row that has to survive a narrow window — and they are a workspace
	control, not a window control.

	The row is absent entirely when nothing is open: a band of chrome across the
	window with nothing in it makes an empty application look broken.
-->
{#if tabs.length > 0}
<div class="tabrow">
	<div class="tabs" role="tablist" aria-label="Open repositories">
		{#each tabs as tab (tab.path)}
			<div
				class="tab"
				class:active={workspace.isActive(tab.path)}
				role="tab"
				tabindex={workspace.isActive(tab.path) ? 0 : -1}
				aria-selected={workspace.isActive(tab.path)}
				title={tab.path}
				onclick={() => void switchTo(tab.path)}
				onkeydown={(event) => {
					if (event.key === 'Enter' || event.key === ' ') {
						event.preventDefault();
						void switchTo(tab.path);
					}
				}}
			>
				<span class="label">{tab.name}</span>
				{#if workspace.isActive(tab.path) && repo.busy}
					<span class="note" aria-label="Opening">…</span>
				{/if}
				<button
					class="close"
					title="Close this tab — the repository stays in your list"
					aria-label="Close {tab.name}"
					onclick={(event) => closeTab(event, tab.path)}
				>
					<Icon name="close" size="0.8em" weight={2} />
				</button>
			</div>
		{/each}

		<button
			class="add"
			title="Open, clone or reopen a repository"
			aria-label="Add a repository"
			onclick={openMenu}
		>
			<Icon name="plus" size="1em" weight={1.9} />
		</button>
	</div>
</div>
{/if}

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		anchor={menu.anchor}
		items={menuItems}
		label="Repositories"
		onclose={() => (menu = null)}
	/>
{/if}

<style>
	/*
		The row itself. Its tabs sit on its bottom edge, because a tab's shape —
		rounded at the top, square at the bottom, an accent underline on the
		active one — is drawn to sit *on* a boundary. That boundary used to be
		the title bar's bottom border; now it is this row's.
	*/
	.tabrow {
		flex: none;
		height: var(--tabs-h);
		display: flex;
		align-items: stretch;
		padding: 4px 10px 0;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid color-mix(in srgb, var(--line) 60%, transparent);
	}

	.tabs {
		display: flex;
		align-items: stretch;
		gap: 2px;
		min-width: 0;
		overflow-x: auto;
		scrollbar-width: none;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 6px;
		max-width: 190px;
		padding: 0 6px 0 10px;
		border-radius: var(--r-field) var(--r-field) 0 0;
		color: var(--muted);
		cursor: pointer;
		user-select: none;
	}

	.tab {
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	.tab:hover {
		background: var(--hover);
		color: var(--ink);
	}

	/*
	 * The open one is a card standing on the row's bottom edge: the ground
	 * colour so it reads as continuous with the screen below, a hairline up
	 * each side, an accent bar along the bottom, and the light catching its top
	 * edge. Before, it was the same rectangle with an underline.
	 */
	.tab.active {
		background: var(--bg);
		color: var(--ink);
		font-weight: 550;
		border: 1px solid var(--line);
		border-bottom: none;
		/* The accent underline is a line, not a shadow: it stays. */
		box-shadow: inset 0 -2px 0 var(--accent);
	}

	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
	}

	/* Visible on the active tab and on hover only: a row of close buttons reads
	   as a row of things to dismiss rather than a row of repositories. */
	.close {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		line-height: 1;
		color: var(--muted);
		padding: 2px;
		border-radius: var(--r-field);
		opacity: 0;
	}

	.tab:hover .close,
	.tab.active .close,
	.close:focus-visible {
		opacity: 1;
	}

	.close:hover {
		color: var(--danger);
		background: var(--danger-soft);
	}

	.add {
		flex: none;
		width: 26px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--muted);
		line-height: 1;
		border-radius: var(--r-field);
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease),
			transform var(--t-fast) var(--spring);
	}

	.add:active {
		transform: scale(0.9);
	}

	.add:hover {
		color: var(--accent);
		background: var(--accent-soft);
	}
</style>
