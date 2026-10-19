<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { page } from '$app/state';
	import BranchMenu from '$lib/branches/BranchMenu.svelte';
	import type { BranchOption } from '$lib/branches/pick';
	import { branches } from '$lib/branches/store.svelte';
	import FileTree from '$lib/files/FileTree.svelte';
	import FileView from '$lib/files/FileView.svelte';
	import { files, revisionLabel } from '$lib/files/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import { tags } from '$lib/tags/store.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';

	/**
	 * Files (1U, FEAT-131): any file at any branch, tag or commit, coloured by
	 * tree-sitter, with blame beside every line.
	 *
	 * It reads git's objects and never the working copy, so it cannot write
	 * anything. *View only* in the header says so once; there is no edit
	 * button to disable and no *Open in editor*, because Spagitty is not an
	 * editor.
	 *
	 * Another screen opens a file here with `/files?path=…&rev=…&line=…`, at
	 * the revision that screen was showing, or a folder with `?dir=…&rev=…`.
	 */
	const info = $derived(repo.info);

	/** This repository's Files: on arrival, and when another one is opened. */
	$effect(() => {
		const path = info?.path ?? null;
		void repo.generation;
		if (!path) return;
		untrack(() => void files.enter(path, info?.head.branch ?? null));
	});

	/** A branch moved under the screen: a commit, a pull, a switch. */
	$effect(() => {
		void info?.head.id;
		untrack(() => {
			if (files.repo && files.repo === info?.path) void files.refresh();
		});
	});

	/** A file asked for by another screen, at the revision it was showing. */
	$effect(() => {
		const asked = page.url.searchParams;
		const path = asked.get('path');
		const dir = asked.get('dir');
		if (!info) return;
		// A folder asked for at a revision: the tree, opened there.
		if (!path && dir !== null) {
			const at = asked.get('rev');
			untrack(() => {
				void files
					.enter(info.path, info.head.branch ?? null)
					.then(() => (at ? files.setRevision(at) : undefined))
					.then(() => reveal(dir));
			});
			return;
		}
		if (!path) return;
		const rev = asked.get('rev') ?? undefined;
		const line = Number(asked.get('line'));
		untrack(() => {
			void files
				.enter(info.path, info.head.branch ?? null)
				.then(() => files.openFile(path, { rev, line: line > 0 ? line : undefined }));
		});
	});

	let menu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	const OPTIONS: BranchOption[] = $derived([
		...branches.rows
			.filter((row) => row.kind === 'branch')
			.map((row) => ({ name: row.name, group: 'local' as const })),
		...branches.rows
			.filter((row) => row.kind === 'remote')
			.map((row) => ({ name: row.name, group: 'remote' as const })),
		...tags.list.map((tag) => ({ name: tag.name, group: 'tag' as const }))
	]);

	function openMenu(event: MouseEvent) {
		if (menu) {
			menu = null;
			return;
		}
		const anchor = event.currentTarget as HTMLElement;
		const box = anchor.getBoundingClientRect();
		menu = { x: box.left, y: box.bottom + 4, anchor };
		if (!branches.loaded) void branches.load();
		if (!tags.loaded) void tags.load();
	}

	/** Open the tree at a folder, and bring its row into view. */
	async function reveal(dir: string) {
		if (files.found) await files.findFiles('');
		files.reveal(dir);
		await tick();
		const row = document.querySelector(`.tree [data-path="${CSS.escape(dir)}"]`);
		(row as HTMLElement | null)?.scrollIntoView?.({ block: 'nearest' });
		(row as HTMLElement | null)?.focus?.();
	}
</script>

<div class="screen">
	{#if !info}
		<EmptyState message="Open a repository to read its files" />
	{:else}
		<header class="head">
			<span class="title">Files</span>
			<button
				class="revision mono"
				aria-haspopup="listbox"
				aria-expanded={menu !== null}
				title="Read at another branch, tag or commit"
				onclick={openMenu}
			>
				<Icon name="branch" size={14} weight={1.9} />
				<span class="name">{revisionLabel(files.rev)}</span>
				<span class="muted" aria-hidden="true">▾</span>
			</button>
			{#if files.resolved}
				<span class="mono muted" title={files.resolved.id}>@ {files.resolved.short}</span>
			{/if}
			{#if files.revisionError}<span class="note error">{files.revisionError}</span>{/if}
			<span class="gap"></span>
			<div class="switches" role="group" aria-label="View">
				<Chip active={files.blameOn} onclick={() => files.toggleBlame()} title="Who wrote each line (b)">
					Blame
				</Chip>
				<Chip active={files.wrap} onclick={() => files.toggleWrap()} title="Wrap long lines">
					Wrap
				</Chip>
			</div>
			<span class="only" title="Spagitty never writes files here">
				<Icon name="review" size={12} weight={2.2} />View only
			</span>
		</header>

		<div class="body">
			<FileTree />
			<Splitter panel="filesTree" label="Resize the tree" />
			<FileView onreveal={reveal} />
		</div>
	{/if}
</div>

{#if menu}
	<BranchMenu
		x={menu.x}
		y={menu.y}
		anchor={menu.anchor}
		label="Read at"
		options={OPTIONS}
		value={files.rev}
		current={info?.head.branch ?? null}
		loading={!branches.loaded}
		onpick={(name) => void files.setRevision(name)}
		oncommit={(revision) => void files.setRevision(revision)}
		onclose={() => (menu = null)}
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
		gap: 8px;
		padding: 12px 16px 10px;
	}

	.title {
		margin-right: 6px;
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.revision {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		max-width: 320px;
		padding: 5px 12px;
		border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
		border-radius: var(--r-button);
		background-color: var(--surface-veil);
		box-shadow: var(--shadow-1);
		font-weight: 550;
	}

	.revision:hover {
		background-color: var(--hover);
	}

	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.gap {
		flex: 1;
	}

	.switches {
		display: flex;
		gap: 6px;
	}

	/* Said once, quietly. It is a fact about the screen, not a control. */
	.only {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		color: var(--muted);
		white-space: nowrap;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 12px 12px;
	}

	/* On the gap, not beside it: the divider takes one gap's room back. */
	.body :global(.splitter.filesTree) {
		margin-inline: -8.5px;
	}

	.error {
		color: var(--danger);
	}
</style>
