<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
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
	Pills in the title bar's row (FEAT-082), and absent entirely when nothing is
	open, which is when the row says the program's name instead.
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
					<Loader size="inline" label="Opening" />
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
	 * The tabs are pills in the one row above the pane (FEAT-082). They were a
	 * row of their own, with the open tab drawn as a card standing on the row's
	 * bottom edge — a shape that only makes sense on a boundary, and the
	 * boundary has gone. A pill needs nothing to stand on.
	 *
	 * The open one is lifted: the pane's own material, its edge, and the
	 * ornaments' shadow. No blur: what is behind it is the environment, which
	 * is smooth, and blurring it would cost a pass and change nothing.
	 */
	.tabrow {
		flex: 0 1 auto;
		display: flex;
		align-items: center;
		min-width: 0;
	}

	.tabs {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
		padding: 4px 2px;
		overflow-x: auto;
		scrollbar-width: none;
	}

	.tab {
		flex: none;
		display: flex;
		align-items: center;
		gap: 6px;
		max-width: 200px;
		height: 28px;
		padding: 0 6px 0 12px;
		border: 1px solid transparent;
		border-radius: var(--r-pill);
		color: var(--muted);
		cursor: pointer;
		user-select: none;
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	.tab:hover {
		background: var(--hover);
		color: var(--ink);
	}

	/*
	 * No shadow (BUG-035). The ornaments' shadow is sized for an object that
	 * floats over the pane; on a pill sitting in the title row it spread wider
	 * than the pill and was cut off by the row, which read as a smudge. The
	 * pill says "this one" with its surface and its edge.
	 */
	.tab.active {
		background: var(--surface);
		color: var(--ink);
		font-weight: 550;
		border-color: var(--pane-edge);
		border-top-color: var(--glass-edge);
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
		padding: 3px;
		border-radius: var(--r-pill);
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
		width: 28px;
		height: 28px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--muted);
		line-height: 1;
		border-radius: var(--r-pill);
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
