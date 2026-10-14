<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { page } from '$app/state';
	import * as api from '$lib/api';
	import BranchMenu from '$lib/branches/BranchMenu.svelte';
	import type { BranchOption } from '$lib/branches/pick';
	import { branches } from '$lib/branches/store.svelte';
	import { targetLabel } from '$lib/investigate/chart';
	import EventDetail from '$lib/investigate/EventDetail.svelte';
	import EventList from '$lib/investigate/EventList.svelte';
	import LifeCard from '$lib/investigate/LifeCard.svelte';
	import { investigate, RANGES, type Range } from '$lib/investigate/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import type { LifeTarget } from '$lib/types';
	import Chip from '$lib/ui/Chip.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';

	/**
	 * Investigate (1O, FEAT-132): the life of a file, a folder, a branch or a
	 * line. When it was born and by whom, how it grew, when it was renamed,
	 * moved or rewritten, and who owns it today.
	 *
	 * It was File history, off the rail and about one file. It is on the rail
	 * now, and `/history` comes here with the path it was given.
	 *
	 * The target is in the address — `?kind=line&path=…&line=…&rev=…` — so
	 * Files' *Investigate line* and a bookmark arrive the same way.
	 */
	type Kind = LifeTarget['kind'];
	const KINDS: { id: Kind; label: string }[] = [
		{ id: 'file', label: 'File' },
		{ id: 'folder', label: 'Folder' },
		{ id: 'branch', label: 'Branch' },
		{ id: 'line', label: 'Line' }
	];

	const info = $derived(repo.info);
	const target = $derived(investigate.target);

	/** What the header shows: the kind chosen and what is typed, before either is a target. */
	let kind = $state<Kind>('file');
	let typed = $state('');
	let lineTyped = $state('1');
	let field = $state<HTMLInputElement | null>(null);

	/** The header follows the target, however the target was set. */
	$effect(() => {
		const shown = target;
		if (!shown) return;
		untrack(() => {
			kind = shown.kind;
			if (shown.kind !== 'branch') typed = shown.path;
			if (shown.kind === 'line') lineTyped = String(shown.line);
		});
	});

	$effect(() => {
		const id = info?.head.id ?? '';
		untrack(() => investigate.setHead(id));
	});

	/**
	 * A target asked for in the address, or the branch that is checked out.
	 *
	 * The address is applied when it changes and when another repository is
	 * opened, and not again. The repository's `info` is a new object after
	 * every refresh, which the watcher makes after any commit, and applying
	 * the address each time would put back the target it named over one the
	 * person has since chosen on the page.
	 */
	let appliedAddress: string | null = null;
	let appliedRepository: string | null = null;
	$effect(() => {
		const asked = page.url.searchParams;
		const here = info;
		if (!here) return;
		untrack(() => {
			const address = asked.toString();
			if (address === appliedAddress && here.path === appliedRepository && investigate.target) return;
			appliedAddress = address;
			appliedRepository = here.path;
			const wanted = asked.get('kind') as Kind | null;
			const path = asked.get('path');
			const rev = asked.get('rev') ?? '';
			if (wanted === 'branch' && asked.get('name')) {
				investigate.show({ kind: 'branch', name: asked.get('name') as string }, rev);
			} else if (path !== null && (wanted === 'file' || wanted === 'folder' || wanted === null)) {
				investigate.show({ kind: wanted ?? 'file', path }, rev);
			} else if (path !== null && wanted === 'line') {
				investigate.show({ kind: 'line', path, line: Math.max(1, Number(asked.get('line')) || 1) }, rev);
			} else if (!investigate.target && here.head.branch) {
				investigate.show({ kind: 'branch', name: here.head.branch });
			}
		});
	});

	/** Turn what is typed into a target. Nothing typed is not one. */
	function commit() {
		suggestions = [];
		const path = typed.trim().replace(/^\/+/, '');
		if (kind === 'branch') return;
		if (kind === 'folder') {
			investigate.show({ kind: 'folder', path: path.replace(/\/+$/, '') }, investigate.rev);
			return;
		}
		if (path === '') return;
		if (kind === 'file') investigate.show({ kind: 'file', path }, investigate.rev);
		else investigate.show({ kind: 'line', path, line: Math.max(1, Number(lineTyped) || 1) }, investigate.rev);
	}

	/**
	 * Another kind of the same thing, where there is one: a file's line, the
	 * folder a file is in, the branch that is checked out. Otherwise the field
	 * waits to be typed in.
	 */
	async function choose(next: Kind) {
		if (next === kind) return;
		const was = target;
		kind = next;
		suggestions = [];
		if (next === 'branch') {
			const name = info?.head.branch;
			if (name) investigate.show({ kind: 'branch', name });
			return;
		}
		const path = was && was.kind !== 'branch' ? was.path : '';
		if (next === 'folder') {
			typed = was?.kind === 'folder' ? path : path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
			if (was && was.kind !== 'branch') commit();
		} else if (was && (was.kind === 'file' || was.kind === 'line')) {
			typed = path;
			commit();
		} else {
			typed = '';
		}
		await tick();
		if (typed === '') field?.focus();
	}

	let suggestions = $state<string[]>([]);
	let at = $state(-1);
	let timer: ReturnType<typeof setTimeout> | null = null;
	let asked = 0;

	/** Paths at the revision that hold what is typed. A folder is offered the folders they are in. */
	function suggest() {
		if (timer) clearTimeout(timer);
		const words = typed.trim();
		at = -1;
		if (words === '') {
			suggestions = [];
			return;
		}
		timer = setTimeout(async () => {
			const mine = ++asked;
			try {
				const found = await api.filesFind(investigate.rev, words);
				if (mine !== asked) return;
				if (kind !== 'folder') {
					suggestions = found.paths.slice(0, 12);
					return;
				}
				const wanted = words.toLowerCase().split(/\s+/);
				const folders = new Set<string>();
				for (const path of found.paths) {
					const parts = path.split('/').slice(0, -1);
					for (let depth = 1; depth <= parts.length; depth++) {
						const folder = parts.slice(0, depth).join('/');
						if (wanted.every((word) => folder.toLowerCase().includes(word))) folders.add(folder);
					}
				}
				suggestions = [...folders].sort((a, b) => a.length - b.length || a.localeCompare(b)).slice(0, 12);
			} catch {
				if (mine === asked) suggestions = [];
			}
		}, 140);
	}

	function take(path: string) {
		typed = path;
		commit();
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			if (at >= 0 && suggestions[at]) take(suggestions[at]);
			else commit();
		} else if (event.key === 'ArrowDown' && suggestions.length > 0) {
			event.preventDefault();
			at = (at + 1) % suggestions.length;
		} else if (event.key === 'ArrowUp' && suggestions.length > 0) {
			event.preventDefault();
			at = (at - 1 + suggestions.length) % suggestions.length;
		} else if (event.key === 'Escape' && suggestions.length > 0) {
			event.preventDefault();
			suggestions = [];
		}
	}

	let branchMenu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);
	let rangeMenu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	const BRANCHES: BranchOption[] = $derived([
		...branches.rows
			.filter((row) => row.kind === 'branch')
			.map((row) => ({ name: row.name, group: 'local' as const })),
		...branches.rows
			.filter((row) => row.kind === 'remote')
			.map((row) => ({ name: row.name, group: 'remote' as const }))
	]);

	function below(event: MouseEvent) {
		const anchor = event.currentTarget as HTMLElement;
		const box = anchor.getBoundingClientRect();
		return { x: box.left, y: box.bottom + 4, anchor };
	}

	function openBranches(event: MouseEvent) {
		branchMenu = branchMenu ? null : below(event);
		if (!branches.loaded) void branches.load();
	}

	const RANGE_ITEMS: MenuItem[] = $derived(
		RANGES.map((option) => ({
			id: option.id,
			label: option.label,
			note: option.id === investigate.range ? '✓' : undefined,
			run: () => investigate.setRange(option.id as Range)
		}))
	);
	const rangeLabel = $derived(RANGES.find((option) => option.id === investigate.range)?.label ?? '');
</script>

<div class="screen">
	{#if !info}
		<EmptyState message="Open a repository to investigate it" />
	{:else}
		<header class="head">
			<span class="title">Investigate</span>
			<div class="kinds" role="radiogroup" aria-label="What to investigate">
				{#each KINDS as option (option.id)}
					<button
						role="radio"
						aria-checked={kind === option.id}
						class:on={kind === option.id}
						onclick={() => choose(option.id)}>{option.label}</button
					>
				{/each}
			</div>

			{#if kind === 'branch'}
				<button class="target mono" aria-haspopup="listbox" aria-expanded={branchMenu !== null} onclick={openBranches}>
					<span class="name">{target?.kind === 'branch' ? target.name : 'Choose a branch'}</span>
					<span class="muted" aria-hidden="true">▾</span>
				</button>
			{:else}
				<div class="typed">
					<label class="target mono">
						<input
							bind:this={field}
							bind:value={typed}
							type="text"
							aria-label={kind === 'folder' ? 'Folder' : 'File'}
							placeholder={kind === 'folder' ? 'path/to/folder' : 'path/to/file'}
							spellcheck="false"
							autocomplete="off"
							oninput={suggest}
							{onkeydown}
							onblur={() => setTimeout(() => (suggestions = []), 120)}
						/>
						{#if kind === 'line'}
							<span class="muted" aria-hidden="true">:</span>
							<input
								class="number"
								bind:value={lineTyped}
								type="text"
								inputmode="numeric"
								aria-label="Line"
								onkeydown={(event) => {
									if (event.key === 'Enter') commit();
								}}
							/>
						{/if}
					</label>
					{#if suggestions.length > 0}
						<ul class="suggestions floating" role="listbox" aria-label="Paths">
							{#each suggestions as path, index (path)}
								<li>
									<button
										role="option"
										aria-selected={index === at}
										class="mono"
										class:at={index === at}
										tabindex="-1"
										onmousedown={(event) => event.preventDefault()}
										onclick={() => take(path)}>{path}</button
									>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/if}

			<span class="gap"></span>
			{#if kind === 'file'}
				<Chip
					active={investigate.follow}
					onclick={() => investigate.setFollow(!investigate.follow)}
					title="Go back past a rename to the name the file had before"
				>
					Follow renames
				</Chip>
			{/if}
			<button
				class="target range"
				aria-haspopup="menu"
				aria-expanded={rangeMenu !== null}
				onclick={(event) => (rangeMenu = rangeMenu ? null : below(event))}
			>
				{rangeLabel}<span class="muted" aria-hidden="true">▾</span>
			</button>
		</header>

		{#if !target}
			<EmptyState message="Choose a file, a folder, a branch or a line" />
		{:else}
			<div class="body" aria-label="Life of {targetLabel(target)}">
				<LifeCard />
				<div class="lower">
					<EventList />
					<EventDetail />
				</div>
			</div>
		{/if}
	{/if}
</div>

{#if branchMenu}
	<BranchMenu
		x={branchMenu.x}
		y={branchMenu.y}
		anchor={branchMenu.anchor}
		label="Investigate a branch"
		options={BRANCHES}
		value={target?.kind === 'branch' ? target.name : null}
		current={info?.head.branch ?? null}
		loading={!branches.loaded}
		onpick={(name) => investigate.show({ kind: 'branch', name })}
		onclose={() => (branchMenu = null)}
	/>
{/if}

{#if rangeMenu}
	<Menu
		x={rangeMenu.x}
		y={rangeMenu.y}
		anchor={rangeMenu.anchor}
		label="How far back"
		items={RANGE_ITEMS}
		onclose={() => (rangeMenu = null)}
	/>
{/if}

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
		gap: 10px;
		padding: 12px 16px 10px;
	}

	.title {
		margin-right: 6px;
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.kinds {
		display: flex;
		gap: 2px;
		padding: 3px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-pill);
		background: var(--sunken);
	}

	.kinds button {
		height: 28px;
		padding: 0 14px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.kinds button.on {
		background: var(--surface-2);
		box-shadow: var(--shadow-1);
		color: var(--ink);
		font-weight: 600;
	}

	.target {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 30px;
		padding: 0 12px;
		border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
		border-radius: var(--r-button);
		background-color: var(--surface-veil);
		box-shadow: var(--shadow-1);
	}

	.target:focus-within {
		border-color: var(--accent);
	}

	.target input {
		width: min(46vw, 420px);
		padding: 0;
		border: none;
		background: none;
		box-shadow: none;
		outline: none;
		font: inherit;
	}

	.target input.number {
		width: 5ch;
	}

	.range {
		font-size: var(--fs-secondary);
		font-weight: 550;
	}

	.typed {
		position: relative;
	}

	.suggestions {
		position: absolute;
		z-index: 20;
		top: calc(100% + 4px);
		left: 0;
		min-width: 100%;
		max-width: 70vw;
		margin: 0;
		padding: 5px;
		list-style: none;
	}

	.suggestions button {
		width: 100%;
		padding: 5px 8px;
		border-radius: var(--r-row);
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.suggestions button:hover,
	.suggestions button.at {
		background: var(--selection);
	}

	.gap {
		flex: 1;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 0 12px 12px;
		overflow: hidden;
	}

	.lower {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
	}
</style>
