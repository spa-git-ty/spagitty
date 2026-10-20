<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack as untrackOf } from 'svelte';
	import BranchField from '$lib/branches/BranchField.svelte';
	import { optionsOf } from '$lib/branches/pick';
	import { branches } from '$lib/branches/store.svelte';
	import FileGlyph from '$lib/files/FileGlyph.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import ScreenHead from '$lib/ui/ScreenHead.svelte';
	import type { FilesEntry } from '$lib/types';
	import { ignoreLine, untrack } from './store.svelte';

	/**
	 * Untrack (FEAT-155): stop tracking files and folders on a branch, in a
	 * commit, and push it.
	 *
	 * Two panes, as Files and Hooks have: on the left the branch and what is
	 * tracked on it, ticked to choose; on the right what will happen — the
	 * paths, the `.gitignore` lines, the commit's message, whether it is
	 * pushed — and the one button that does it. Nothing is written until it
	 * is pressed, and every line of what it writes is on screen before then.
	 */

	interface Row {
		entry: FilesEntry;
		depth: number;
	}

	/** Only a local branch can take a commit. */
	const options = $derived(optionsOf(branches.rows.filter((row) => row.kind === 'branch')));
	const here = $derived(repo.info?.head.branch ?? null);
	const checkedOut = $derived(untrack.branch !== '' && untrack.branch === here);

	/** The open folders, flattened to the rows they show. */
	const rows = $derived.by(() => {
		const out: Row[] = [];
		const walk = (dir: string, depth: number) => {
			const held = untrack.folder(dir);
			if (!Array.isArray(held)) return;
			for (const entry of held) {
				if (entry.kind === 'submodule') continue;
				out.push({ entry, depth });
				if (entry.kind === 'folder' && untrack.open.includes(entry.path)) walk(entry.path, depth + 1);
			}
		};
		walk('', 0);
		return out;
	});
	const root = $derived(untrack.folder(''));

	/** Start on the branch checked out here, and again when the repository changes. */
	let generation: number | null = null;
	$effect(() => {
		const current = repo.generation;
		if (repo.info === null || generation === current) return;
		generation = current;
		untrackOf(() => {
			if (branches.rows.length === 0) void branches.load();
			void untrack.choose(repo.info?.head.branch ?? '');
		});
	});

	let timer: ReturnType<typeof setTimeout> | null = null;
	function type(text: string) {
		if (timer) clearTimeout(timer);
		timer = setTimeout(() => void untrack.find(text), 140);
	}

	const action = $derived(untrack.push ? 'Untrack, commit and push' : 'Untrack and commit');
</script>

<div class="screen">
	<ScreenHead title="Untrack">
		{#snippet detail()}
			<span class="note">Stop tracking files on a branch. They stay on disk.</span>
		{/snippet}
	</ScreenHead>

	{#if repo.info === null}
		<div class="empty"><p class="note">No repository open.</p></div>
	{:else}
		<div class="body">
			<section class="pane pick" aria-label="What to untrack">
				<label class="label" for="untrack-branch">Branch</label>
				<BranchField
					id="untrack-branch"
					value={untrack.branch}
					{options}
					current={here}
					label="Branch to untrack on"
					onpick={(name) => void untrack.choose(name)}
				/>
				{#if untrack.branch !== ''}
					<p class="note small">
						{#if checkedOut}
							Checked out here. The commit is made in place, with your hooks.
						{:else}
							Not checked out here. The commit is made beside your working copy, which does not change.
						{/if}
					</p>
				{/if}

				<label class="find">
					<Icon name="search" size={14} weight={2} />
					<input
						type="text"
						aria-label="Find a file on the branch"
						placeholder="Find a file"
						spellcheck="false"
						autocomplete="off"
						disabled={!untrack.revision}
						value={untrack.finding}
						oninput={(event) => type(event.currentTarget.value)}
					/>
				</label>

				<div class="rows" role="group" aria-label="Tracked on {untrack.branch || 'the branch'}">
					{#if untrack.found}
						{#each untrack.found as path (path)}
							{@const picked = untrack.isChosen(path)}
							<label class="row" class:on={picked}>
								<input
									type="checkbox"
									checked={picked}
									disabled={untrack.isCovered(path)}
									onchange={() => untrack.toggle({ path, folder: false })}
								/>
								<FileGlyph {path} />
								<span class="name">{path}</span>
							</label>
						{:else}
							<span class="note said">No file by that name</span>
						{/each}
					{:else if untrack.branch === ''}
						<span class="note said">Choose a branch.</span>
					{:else if root === 'loading' || root === undefined}
						<Loader size="inline" label="Reading…" />
					{:else if !Array.isArray(root)}
						<span class="note said error">{root.error}</span>
					{:else}
						{#each rows as { entry, depth } (entry.path)}
							{@const folder = entry.kind === 'folder'}
							{@const opened = folder && untrack.open.includes(entry.path)}
							{@const picked = untrack.isChosen(entry.path)}
							<div class="row" class:on={picked} style:padding-left="{6 + depth * 14}px">
								<button
									type="button"
									class="arrow"
									aria-label={folder ? `${opened ? 'Close' : 'Open'} ${entry.name}` : undefined}
									aria-expanded={folder ? opened : undefined}
									tabindex={folder ? 0 : -1}
									disabled={!folder}
									onclick={() => untrack.toggleFolder(entry.path)}>{folder ? (opened ? '▾' : '▸') : ''}</button
								>
								<label class="choose">
									<input
										type="checkbox"
										checked={picked}
										disabled={untrack.isCovered(entry.path)}
										onchange={() => untrack.toggle({ path: entry.path, folder })}
									/>
									<FileGlyph path={entry.path} {folder} open={opened} />
									<span class="name">{entry.name}</span>
								</label>
							</div>
						{/each}
					{/if}
				</div>
			</section>

			<section class="pane plan" aria-label="Untrack">
				<span class="label">Untracking</span>
				{#if untrack.chosen.length === 0}
					<p class="note">Tick the files and folders to stop tracking on {untrack.branch || 'the branch'}.</p>
				{:else}
					<ul class="chosen">
						{#each untrack.chosen as target (target.path)}
							<li>
								<FileGlyph path={target.path} folder={target.folder} />
								<span class="mono path">{target.path}{target.folder ? '/' : ''}</span>
								<button
									type="button"
									class="drop"
									aria-label="Keep tracking {target.path}"
									title="Keep tracking it"
									onclick={() => untrack.remove(target.path)}
								>
									<Icon name="close" size={11} weight={2} />
								</button>
							</li>
						{/each}
					</ul>
				{/if}

				<label class="switch-line">
					<input
						type="checkbox"
						role="switch"
						checked={untrack.ignore}
						onchange={(event) => untrack.setIgnore(event.currentTarget.checked)}
					/>
					<span>
						<span class="strong">Add to .gitignore</span>
						<span class="note small">Otherwise the next <span class="mono">git add .</span> tracks them again.</span>
					</span>
				</label>
				{#if untrack.ignore && untrack.chosen.length > 0}
					<pre class="lines mono" aria-label="Lines added to .gitignore">{untrack.chosen.map(ignoreLine).join('\n')}</pre>
				{/if}

				<label class="label" for="untrack-message">Commit message</label>
				<textarea
					id="untrack-message"
					class="message mono"
					rows="4"
					spellcheck="false"
					value={untrack.message}
					oninput={(event) => untrack.setMessage(event.currentTarget.value)}
				></textarea>

				<label class="switch-line">
					<input
						type="checkbox"
						role="switch"
						checked={untrack.push}
						onchange={(event) => untrack.setPush(event.currentTarget.checked)}
					/>
					<span>
						<span class="strong">Push after committing</span>
						<span class="note small">To the branch's remote, creating the branch there if it is not yet.</span>
					</span>
				</label>

				{#if untrack.error}<p class="note error" role="alert">{untrack.error}</p>{/if}

				<div class="go">
					<Btn
						primary
						disabled={untrack.busy || untrack.branch === '' || untrack.chosen.length === 0 || untrack.message.trim() === ''}
						onclick={() => void untrack.run()}
					>
						{untrack.busy ? 'Committing…' : action}
					</Btn>
				</div>

				{#if untrack.done}
					<p class="note made" role="status">
						Committed <span class="mono">{untrack.done.commit.slice(0, 7)}</span> on
						<span class="mono">{untrack.done.branch}</span>{#if !untrack.done.hooksRan}, without hooks{/if}.
						{#if untrack.pushed}Pushing to <span class="mono">{untrack.done.remote}</span>.{:else if untrack.push && !untrack.done.remote}There is no remote to push it to.{/if}
					</p>
				{/if}
			</section>
		</div>
	{/if}
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.empty {
		flex: 1;
		display: grid;
		place-items: center;
	}

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 12px;
	}

	.pane {
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 14px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		background: var(--surface);
	}

	.pick {
		flex: 1 1 0;
		min-width: 260px;
	}

	.plan {
		flex: 1 1 0;
		min-width: 300px;
		max-width: 560px;
		overflow: auto;
	}

	.label {
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-size: var(--fs-secondary);
	}

	.note {
		margin: 0;
		color: var(--muted);
		font-size: var(--fs-secondary);
		line-height: 1.45;
	}

	.small {
		font-size: calc(var(--fs-secondary) * 0.92);
	}

	.strong {
		display: block;
		color: var(--ink);
		font-weight: 600;
		font-size: var(--fs-secondary);
	}

	.error {
		color: var(--danger);
	}

	.find {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		height: 32px;
		padding: 0 10px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-button);
		background: var(--sunken);
		color: var(--muted);
	}

	.find:focus-within {
		border-color: var(--accent);
	}

	.find input {
		flex: 1;
		min-width: 0;
		padding: 0;
		border: none;
		background: none;
		box-shadow: none;
		outline: none;
		font-size: var(--fs-secondary);
	}

	.rows {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.row {
		flex: none;
		display: flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 8px 0 6px;
		border-radius: var(--r-field);
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.row:hover {
		background: var(--hover);
	}

	.row.on {
		background: color-mix(in srgb, var(--accent) 14%, transparent);
	}

	.arrow {
		flex: none;
		width: 14px;
		color: var(--muted);
	}

	.choose {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		cursor: pointer;
	}

	input[type='checkbox'] {
		accent-color: var(--accent);
	}

	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.said {
		padding: 6px;
	}

	.chosen {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.chosen li {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 6px 4px 8px;
		border-radius: var(--r-field);
		background: var(--sunken);
	}

	.path {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-mono);
	}

	.drop {
		flex: none;
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		border-radius: var(--r-row);
		color: var(--muted);
	}

	.drop:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.switch-line {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		margin-top: 6px;
		cursor: pointer;
	}

	.switch-line input {
		margin-top: 3px;
	}

	.lines {
		margin: 0;
		padding: 8px 10px;
		border-radius: var(--r-field);
		background: var(--sunken);
		color: var(--muted);
		font-size: var(--fs-mono);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.message {
		width: 100%;
		box-sizing: border-box;
		resize: vertical;
		font-size: var(--fs-mono);
	}

	.go {
		display: flex;
		justify-content: flex-end;
		margin-top: 4px;
	}

	.made {
		color: var(--ok);
	}

	/* One above the other when the window is narrow. */
	@media (max-width: 900px) {
		.body {
			flex-direction: column;
			overflow: auto;
		}

		.plan {
			max-width: none;
		}
	}
</style>
