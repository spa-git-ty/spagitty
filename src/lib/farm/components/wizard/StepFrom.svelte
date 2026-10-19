<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import * as gitApi from '$lib/api';
	import * as issuesApi from '$lib/issues/api';
	import type { Hosted, Issue } from '$lib/issues/types';
	import type { BranchRow } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '../../api';
	import type { WizardState } from '../../wizard';

	/**
	 * Step 2 (FEAT-141): anything to start from. Any mix of issues from the
	 * Issues screen's providers (FEAT-136), written briefs, and one branch in
	 * progress. The planner reads all of it, with the goal.
	 */
	let { wizard = $bindable(), busy }: { wizard: WizardState; busy: boolean } = $props();

	let host = $state<Hosted | null>(null);
	let issues = $state<Issue[]>([]);
	let issuesError = $state('');
	let issuesLoading = $state(false);
	let search = $state('');
	let mdFiles = $state<string[]>([]);
	let briefFilter = $state('');
	let pickingBrief = $state(false);
	let briefError = $state('');
	let branches = $state<BranchRow[]>([]);
	let pickingBranch = $state(false);
	let searchTimer: ReturnType<typeof setTimeout> | null = null;

	const picked = $derived(new Set(wizard.issues.map((i) => i.number)));
	const shownBriefs = $derived(
		mdFiles
			.filter((path) => !wizard.briefs.some((b) => b.path === path))
			.filter((path) => path.toLowerCase().includes(briefFilter.trim().toLowerCase()))
			.slice(0, 8)
	);

	async function readIssues(text: string) {
		if (!host) return;
		issuesLoading = true;
		try {
			const page = await issuesApi.page(host.remote, `is:open ${text}`.trim(), 1);
			issues = page.data.issues;
			issuesError = '';
		} catch (cause) {
			issuesError = api.failure(cause).message;
		} finally {
			issuesLoading = false;
		}
	}

	onMount(() => {
		void (async () => {
			try {
				const hosts = await issuesApi.remotes();
				host = hosts.find((h) => h.remote === 'origin') ?? hosts[0] ?? null;
				if (host) await readIssues('');
			} catch (cause) {
				issuesError = api.failure(cause).message;
			}
		})();
		void Promise.resolve(api.briefs()).then((found) => (mdFiles = found ?? [])).catch(() => {});
		void Promise.resolve(gitApi.branches()).then((rows) => (branches = (rows ?? []).filter((b) => b.kind === 'branch'))).catch(() => {});
		return () => {
			if (searchTimer) clearTimeout(searchTimer);
		};
	});

	function searched(text: string) {
		search = text;
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => void readIssues(text), 300);
	}

	function toggle(issue: Issue) {
		wizard.issues = picked.has(issue.number)
			? wizard.issues.filter((i) => i.number !== issue.number)
			: [...wizard.issues, issue];
	}

	async function addBrief(path: string) {
		try {
			const brief = await api.brief(path);
			if (!wizard.briefs.some((b) => b.path === brief.path)) wizard.briefs = [...wizard.briefs, brief];
			briefError = '';
			pickingBrief = false;
			briefFilter = '';
		} catch (cause) {
			briefError = api.failure(cause).message;
		}
	}

	async function chooseFile() {
		const chosen = await openDialog({ multiple: false, filters: [{ name: 'Markdown', extensions: ['md', 'markdown'] }] });
		if (typeof chosen === 'string') await addBrief(chosen);
	}
</script>

<section class="card block issues" class:lit={wizard.issues.length > 0}>
	<div class="block-head">
		<div class="name">
			<span class="tile issues-tile"><Icon name="issues" size="1.1em" /></span>
			<strong>Issues</strong>
			{#if wizard.issues.length}<span class="count">{wizard.issues.length} picked</span>{/if}
		</div>
		{#if host}
			<label class="search">
				<Icon name="search" size="0.9em" />
				<input type="search" placeholder="Search issues" aria-label="Search issues" value={search} oninput={(e) => searched(e.currentTarget.value)} />
			</label>
		{/if}
	</div>
	{#if !host && !issuesError}
		<p class="muted">No hosted remote with issues. Connect an account in Settings › Accounts.</p>
	{:else if issuesError}
		<p class="error">{issuesError}</p>
	{/if}
	{#each [...wizard.issues.filter((i) => !issues.some((x) => x.number === i.number)), ...issues].slice(0, 12) as issue (issue.number)}
		<label class="issue">
			<input type="checkbox" checked={picked.has(issue.number)} disabled={busy} onchange={() => toggle(issue)} />
			<span class="mono muted">#{issue.number}</span>
			<span class="ellipsis">{issue.title}</span>
			{#if issue.labels[0]}<span class="label">{issue.labels[0].name}</span>{:else}<span></span>{/if}
		</label>
	{:else}
		{#if host && !issuesLoading && !issuesError}<p class="muted">No open issues{search ? ' match' : ''}.</p>{/if}
	{/each}
</section>

<section class="card block">
	<div class="block-head">
		<div class="name">
			<span class="tile brief-tile"><Icon name="files" size="1.1em" /></span>
			<strong>Brief</strong>
			{#each wizard.briefs as brief (brief.path)}
				<span class="mono file">{brief.path} <span class="muted">{Math.max(1, Math.round(brief.bytes / 1024))} KB</span></span>
			{/each}
		</div>
		<div class="actions">
			{#if wizard.briefs.length}
				<Btn disabled={busy} onclick={() => (wizard.briefs = wizard.briefs.slice(0, -1))}>Remove</Btn>
			{/if}
			<Btn disabled={busy} onclick={() => (pickingBrief = !pickingBrief)}>{wizard.briefs.length ? 'Add another' : 'Add a brief'}</Btn>
		</div>
	</div>
	{#if pickingBrief}
		<div class="picker">
			<input placeholder="Markdown files in this repository" aria-label="Find a brief" bind:value={briefFilter} />
			{#each shownBriefs as path (path)}
				<button class="pick mono" onclick={() => addBrief(path)}>{path}</button>
			{:else}
				<p class="muted">No Markdown files{briefFilter ? ' match' : ''}.</p>
			{/each}
			<Btn onclick={chooseFile}>Choose a file…</Btn>
		</div>
	{/if}
	{#if briefError}<p class="error">{briefError}</p>{/if}
</section>

<section class="block dashed" class:card={!!wizard.branch}>
	<div class="block-head">
		<div class="name">
			<span class="tile branch-tile"><Icon name="branch" size="1.1em" /></span>
			<span class="stack">
				<strong>Branch in progress</strong>
				<span class="muted">{wizard.branch ? `Tasks start from ${wizard.branch} and merge back into it` : 'The farm finishes it, or splits it into tasks'}</span>
			</span>
		</div>
		{#if wizard.branch}
			<Btn disabled={busy} onclick={() => (wizard.branch = null)}>Remove</Btn>
		{:else}
			<Btn disabled={busy || !branches.length} onclick={() => (pickingBranch = !pickingBranch)}>Pick a branch</Btn>
		{/if}
	</div>
	{#if pickingBranch && !wizard.branch}
		<select aria-label="Branch in progress" onchange={(e) => { wizard.branch = e.currentTarget.value || null; pickingBranch = false; }}>
			<option value="">Choose a branch</option>
			{#each branches as branch (branch.name)}
				<option value={branch.name}>{branch.name}{branch.current ? ' (checked out)' : ''}</option>
			{/each}
		</select>
	{/if}
</section>

<style>
	.block {
		padding: 16px 18px;
		border-radius: var(--r-floating);
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.issues.lit {
		border-color: color-mix(in srgb, var(--lane-5) 40%, transparent);
	}

	.dashed {
		border: 1.5px dashed var(--soft);
	}

	.block-head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}

	.name {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.name strong {
		font-size: calc(var(--fs-ui) * 1.05);
	}

	.stack {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.tile {
		width: 34px;
		height: 34px;
		display: grid;
		place-items: center;
		border-radius: 10px;
		color: var(--tone);
		background: color-mix(in srgb, var(--tone) 14%, transparent);
	}

	.issues-tile {
		--tone: var(--lane-5);
	}

	.brief-tile {
		--tone: var(--warn);
	}

	.branch-tile {
		--tone: var(--ok);
	}

	.count {
		color: var(--lane-5);
		font-size: var(--fs-secondary);
	}

	/*
	 * farm.css stacks every label in the farm (`.farm-journey label` is a
	 * column) and gives every input a border and padding. Both won here, so
	 * the icon sat on a bar of its own with a second box under it (BUG-093).
	 * The direction is said outright, and the input's rule is made more
	 * specific than the shared one.
	 */
	.search {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 8px;
		height: 36px;
		width: 240px;
		max-width: 100%;
		padding: 0 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
		background: var(--sunken);
		color: var(--muted);
	}

	.search input[type='search'] {
		flex: 1;
		height: 100%;
		border-radius: 0;
		box-shadow: none;
		min-width: 0;
		border: 0;
		padding: 0;
		background: transparent;
		color: var(--ink);
		outline: none;
		font-size: var(--fs-secondary);
	}

	.issue {
		display: grid;
		grid-template-columns: 22px 54px minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
		padding: 10px 4px;
		border-top: 1px solid color-mix(in srgb, var(--line) 45%, transparent);
		cursor: pointer;
	}

	.label {
		padding: 3px 9px;
		border-radius: var(--r-pill);
		border: 1px solid var(--soft);
		color: var(--lane-5);
		font-size: var(--fs-mono);
	}

	.ellipsis {
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.file {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 5px 10px;
		border-radius: 8px;
		background: var(--soft);
		font-size: var(--fs-mono);
	}

	.actions {
		display: flex;
		gap: 8px;
	}

	.picker {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 6px;
	}

	.picker input {
		width: 100%;
		box-sizing: border-box;
	}

	.pick {
		padding: 4px 8px;
		border: 0;
		border-radius: var(--r-button);
		background: none;
		color: var(--ink);
		font-size: var(--fs-secondary);
		text-align: left;
		cursor: pointer;
	}

	.pick:hover {
		background: var(--hover);
	}

	.error {
		margin: 0;
		color: var(--danger);
	}

	p {
		margin: 0;
	}
</style>
