<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import RefTabs from '$lib/branches/RefTabs.svelte';
	import { onMount, untrack } from 'svelte';
	import * as api from '$lib/api';
	import { relativeTime } from '$lib/format';
	import { checkoutTag, deleteTag, editMessage } from '$lib/tags/actions';
	import { tags } from '$lib/tags/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * Every tag, and what each one names (FEAT-051).
	 *
	 * Creating and deleting were already possible from the graph's context
	 * menu, which meant doing either required already looking at the commit it
	 * was about. This screen answers the question people actually have — what
	 * versions are there — and is the only place an annotated tag's message can
	 * be read at all.
	 *
	 * Annotated and lightweight are marked rather than flattened. The difference
	 * decides whether a message can exist, and a row that just showed nothing
	 * would be answering "no message" to a question about the wrong thing.
	 */

	let generation: number | null = null;
	$effect(() => {
		const current = repo.generation;
		if (repo.info === null) return;
		if (generation === current) return;

		generation = current;
		untrack(() => {
			tags.clear();
			tags.load();
		});
	});

	onMount(() => {
		if (api.inTauri() && repo.info) tags.load();
		return () => tags.clear();
	});

	const rows = $derived(tags.filtered);
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<RefTabs />
			{#if tags.loaded}
				<span class="note">
					{tags.list.length}
					{tags.list.length === 1 ? 'tag' : 'tags'}
				</span>
			{/if}
		</div>
		<div class="right">
			{#if tags.loading}<span class="note">Reading…</span>{/if}
			<input
				class="field"
				type="text"
				placeholder="Filter by name or message"
				spellcheck="false"
				aria-label="Filter tags"
				value={tags.query}
				oninput={(event) => tags.setQuery(event.currentTarget.value)}
			/>
			<Btn disabled={tags.busy} onclick={() => tags.load()}>Refresh</Btn>
		</div>
	</header>

	<div class="body">
		{#if repo.info === null}
			<div class="empty"><p class="note">No repository open.</p></div>
		{:else if tags.error}
			<div class="empty"><p class="note error">{tags.error}</p></div>
		{:else if !tags.loaded}
			<div class="empty"><span class="note">Reading…</span></div>
		{:else if rows.length === 0}
			<div class="empty">
				<p class="note">
					{tags.list.length === 0
						? 'This repository has no tags yet.'
						: 'No tag matches that filter.'}
				</p>
			</div>
		{:else}
			<div class="table" role="table" aria-label="Tags">
				{#each rows as tag (tag.name)}
					<div class="row" class:lightweight={!tag.annotated} role="row">
						<div class="what" role="cell">
							<span class="name">{tag.name}</span>
							<span class="note kind" title={tag.annotated
								? 'An object of its own, with a message and a tagger'
								: 'A ref pointing straight at the commit — it can carry no message'}>
								{tag.annotated ? 'annotated' : 'lightweight'}
							</span>
							<span class="mono muted target" title={tag.target}>{tag.targetShort}</span>
						</div>

						<div class="says" role="cell">
							{#if tag.annotated && tag.message}
								<span class="message" title={tag.message}>{tag.message}</span>
							{:else}
								<span class="note summary" title={tag.summary}>{tag.summary}</span>
							{/if}
						</div>

						<span class="note when" role="cell" title={tag.taggerName}>
							{relativeTime(tag.time)}
						</span>

						<span class="acts" role="cell">
							<Chip
								disabled={tags.busy}
								title="Check out {tag.name} with no branch attached"
								onclick={() => checkoutTag(tag)}
							>
								check out
							</Chip>
							{#if tag.annotated}
								<Chip
									disabled={tags.busy}
									title="Rewrite the message — the tag is recreated at the same commit"
									onclick={() => editMessage(tag)}
								>
									edit message
								</Chip>
							{:else}
								<Chip title="A lightweight tag has no message to edit">edit message</Chip>
							{/if}
							<Chip
								danger
								disabled={tags.busy}
								title="Delete {tag.name} locally"
								onclick={() => deleteTag(tag)}
							>
								delete
							</Chip>
						</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#if repo.info !== null}
		<div class="create">
			<span class="note">New tag</span>
			<input
				class="field name"
				type="text"
				placeholder="name"
				spellcheck="false"
				aria-label="New tag name"
				value={tags.newName}
				oninput={(event) => tags.setNewName(event.currentTarget.value)}
			/>
			<span class="note">at</span>
			<input
				class="field target"
				type="text"
				placeholder={repo.info.head.branch ?? 'HEAD'}
				spellcheck="false"
				aria-label="Commit to tag"
				value={tags.newTarget}
				oninput={(event) => tags.setNewTarget(event.currentTarget.value)}
			/>
			<input
				class="field message"
				type="text"
				placeholder="message — leave empty for a lightweight tag"
				aria-label="Tag message"
				value={tags.newMessage}
				oninput={(event) => tags.setNewMessage(event.currentTarget.value)}
			/>
			<Btn primary disabled={!tags.creatable} onclick={() => tags.create()}>Create</Btn>
		</div>
	{/if}

	<footer class="foot">
		{#if tags.writeError}
			<span class="note error">{tags.writeError}</span>
		{:else if tags.hidden > 0}
			<span class="note">{tags.hidden} hidden by the filter</span>
		{:else}
			<span class="note">
				Newest first. A lightweight tag is dated by the commit it points at, which is
				the only date it has.
			</span>
		{/if}
	</footer>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head,
	.foot,
	.create {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.head {
		justify-content: space-between;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.field {
		font: inherit;
		font-size: var(--fs-ui);
	}

	.right .field {
		width: 200px;
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}

	.row {
		display: grid;
		grid-template-columns: minmax(0, 260px) minmax(0, 1fr) 90px auto;
		align-items: center;
		gap: 10px;
		padding: 4px 12px;
		border-bottom: 1px solid var(--soft);
		min-height: 30px;
	}

	/* A lightweight tag carries nothing but a position, so it is dimmed rather
	   than drawn with a dashed rule. */
	.row.lightweight {
		color: var(--muted);
	}

	.row:hover {
		background: var(--hover);
	}

	.what,
	.says {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	.name {
		color: var(--accent);
		flex: none;
	}

	.kind,
	.target {
		flex: none;
	}

	.message,
	.summary,
	.when {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.acts {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
	}

	.create {
		padding: 8px 12px;
		border-top: 1px solid var(--soft);
	}

	.create .name {
		width: 140px;
	}

	.create .target {
		width: 140px;
	}

	.create .message {
		flex: 1;
		min-width: 0;
	}

	.empty {
		padding: 14px 12px;
	}

	.foot {
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-top: 1px solid color-mix(in srgb, var(--line) 55%, transparent);
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	p {
		margin: 0;
	}
</style>
