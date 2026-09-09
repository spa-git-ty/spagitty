<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { remotes } from '$lib/remotes/store.svelte';
	import { removeRemote, renameRemote, retargetRemote } from '$lib/remotes/actions';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import type { Host } from '$lib/types';

	/**
	 * The remotes of the repository that is open (FEAT-049).
	 *
	 * In Settings rather than on a screen of its own because this is
	 * configuration people touch once and then not again for months, and the
	 * rail is for the places work happens. It is the one section here that is
	 * about the *open repository* rather than about Spagitty, which is why it
	 * says so when there is none.
	 *
	 * Every remote shows how many refs it has. Zero means it has never been
	 * fetched, and that number is the difference between a remote somebody just
	 * added and one whose URL stopped working months ago — which is otherwise
	 * invisible until a push fails.
	 */

	const HOSTS: Record<Host, string> = {
		gitHub: 'GitHub',
		gitLab: 'GitLab',
		bitbucket: 'Bitbucket',
		azureDevOps: 'Azure DevOps',
		generic: 'git'
	};

	onMount(() => {
		remotes.load();
		return () => remotes.clear();
	});
</script>

<section class="section" id="remotes">
	<header>
		<h2 class="heading">Remotes</h2>
		{#if repo.info}
			<span class="note">{repo.info.name}</span>
		{/if}
	</header>

	{#if repo.info === null}
		<p class="note">No repository is open. Remotes belong to one.</p>
	{:else}
		{#if remotes.error}
			<p class="note error">{remotes.error}</p>
		{:else if !remotes.loaded}
			<p class="note">Reading…</p>
		{:else if remotes.list.length === 0}
			<p class="note">No remotes.</p>
		{:else}
			<ul class="list">
				{#each remotes.list as remote (remote.name)}
					<li class="remote">
						<div class="what">
							<span class="name">{remote.name}</span>
							<span class="note host">{HOSTS[remote.host]}</span>
							<span class="note refs">
								{remote.refs === 0
									? 'never fetched'
									: `${remote.refs} ${remote.refs === 1 ? 'ref' : 'refs'}`}
							</span>
						</div>
						<div class="url mono note" title={remote.url}>{remote.url}</div>
						{#if remote.pushUrl}
							<div class="url mono note" title={remote.pushUrl}>
								pushes to {remote.pushUrl}
							</div>
						{/if}
						<div class="acts">
							<Chip
								disabled={remotes.busy}
								title="Rename {remote.name}"
								onclick={() => renameRemote(remote)}
							>
								rename
							</Chip>
							<Chip
								disabled={remotes.busy}
								title="Change where {remote.name} points"
								onclick={() => retargetRemote(remote)}
							>
								change URL
							</Chip>
							<Chip
								danger
								disabled={remotes.busy}
								title="Remove {remote.name}"
								onclick={() => removeRemote(remote)}
							>
								remove
							</Chip>
						</div>
					</li>
				{/each}
			</ul>
		{/if}

		<div class="add">
			<input
				class="field name"
				type="text"
				placeholder="name"
				spellcheck="false"
				aria-label="New remote name"
				value={remotes.newName}
				oninput={(event) => remotes.setNewName(event.currentTarget.value)}
			/>
			<input
				class="field url"
				type="text"
				placeholder="https://… or git@…"
				spellcheck="false"
				aria-label="New remote URL"
				value={remotes.newUrl}
				oninput={(event) => remotes.setNewUrl(event.currentTarget.value)}
			/>
			<Btn primary disabled={!remotes.addable} onclick={() => remotes.add()}>Add</Btn>
		</div>

		

		{#if remotes.writeError}
			<p class="note error">{remotes.writeError}</p>
		{/if}
	{/if}
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 640px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	header {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.remote {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px 10px;
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
	}

	.what {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.url {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.acts {
		display: flex;
		gap: 6px;
		padding-top: 4px;
	}

	.add {
		display: flex;
		gap: 6px;
	}

	.field {
		font: inherit;
		font-size: var(--fs-ui);
	}

	.name {
		flex: none;
		width: 120px;
	}

	.url.field,
	.add .url {
		flex: 1;
		min-width: 0;
	}

	p {
		margin: 0;
	}
</style>
