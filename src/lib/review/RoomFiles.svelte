<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import { agentRead, readingNow } from '$lib/agents/levels';
	import type { Assignment } from '$lib/agents/types';
	import { room } from './room.svelte';

	/**
	 * The files a pull request touches, each with its viewed tick
	 * (FEAT-091), and which of them hold code written while fixing a merge
	 * conflict (FEAT-092).
	 *
	 * A tick is kept against the file's content, not its name: when the author
	 * changes a ticked file, it comes back unticked, and a file still bold is
	 * one still to read.
	 */

	function name(path: string): string {
		return path.slice(path.lastIndexOf('/') + 1);
	}

	// Conflict fixes are found in the fetched head; the host's patch has no
	// merges to re-do.
	const local = $derived(room.head !== null);
	// What each filter lets through, so choosing one visibly picks (TASK-053).
	const authors = $derived(room.files.filter((file) => room.isAuthors(file.path)).length);
	const fixes = $derived(room.files.filter((file) => room.hasFix(file.path)).length);

	interface Props {
		/** The agent assigned to this pull request, when there is one (2.0). */
		assignment?: Assignment | null;
	}

	let { assignment = null }: Props = $props();

	/** Findings waiting for the person, per file. Read from the drafts, so the
	 * marks and the diff never disagree. */
	const waiting = $derived.by(() => {
		const counts = new Map<string, number>();
		for (const draft of room.drafts) {
			if (draft.agent?.state === 'proposed') counts.set(draft.path, (counts.get(draft.path) ?? 0) + 1);
		}
		return counts;
	});
	const findings = $derived([...waiting.values()].reduce((sum, n) => sum + n, 0));
	const here = $derived(assignment ? readingNow(assignment) : null);
	const read = $derived(assignment ? agentRead(assignment) : new Set<string>());
	const agentName = $derived(assignment?.agent.name ?? 'Agent');
</script>

<aside class="files" aria-label="Touched files">
	{#if local}
		<div class="filters">
			<Chip active={room.filter === 'all'} onclick={() => room.setFilter('all')}>All {room.files.length}</Chip>
			<Chip active={room.filter === 'author'} onclick={() => room.setFilter('author')}>Author {authors}</Chip>
			<Chip active={room.filter === 'conflict'} onclick={() => room.setFilter('conflict')}
				>Conflict fixes {fixes}</Chip
			>
			{#if findings > 0 || room.filter === 'agent'}
				<Chip active={room.filter === 'agent'} onclick={() => room.setFilter('agent')}>
					<span class="agent-mark"><Icon name="agent" size="0.95em" weight={2} /></span>Agent · {findings}
				</Chip>
			{/if}
		</div>
	{/if}
	<ul>
		{#each room.visible as file (file.path)}
			{@const viewed = room.isViewed(file.path)}
			{@const threads = room.openThreadsOn(file.path)}
			<li class:on={room.layout === 'one' && room.selected === file.path}>
				<input
					type="checkbox"
					aria-label="Viewed {name(file.path)}"
					checked={viewed}
					onchange={() => room.toggleViewed(file.path)}
				/>
				<button class="pick" title={file.path} onclick={() => room.select(file.path)}>
					<span class="name mono" class:viewed>{name(file.path)}</span>
					<span class="facts">
						<span class="mono"><span class="add">+{file.added}</span> <span class="del">−{file.removed}</span></span>
						{#if local && room.isAuthors(file.path)}<span class="by"><span class="dot author"></span>author</span>{/if}
						{#if room.hasFix(file.path)}<span class="fix"><span class="dot"></span>conflict fix</span>{/if}
						{#if threads > 0}<span class="threads">{threads} {threads === 1 ? 'thread' : 'threads'}</span>{/if}
						{#if read.has(file.path) && !waiting.get(file.path)}<span class="read" title="{agentName} read this file. Viewed is yours to tick.">agent read</span>{/if}
					</span>
				</button>
				{#if waiting.get(file.path)}
					<span class="found" title="{agentName}’s findings waiting for you">
						<Icon name="agent" size="0.9em" weight={2} />{waiting.get(file.path)}
					</span>
				{/if}
				{#if here === file.path}
					<span class="here" title="{agentName} is reading this file"></span>
				{/if}
			</li>
		{:else}
			<li class="none note">
				{room.filter === 'conflict' ? 'No conflict fixes in this pull request.' : 'No files.'}
			</li>
		{/each}
	</ul>
	{#if local}
		<div class="legend note">
			<span><span class="dot author"></span>Author's own commits</span>
			<span><span class="dot"></span>Written while fixing a merge conflict</span>
			{#if assignment}
				<span class="agent-legend"><Icon name="agent" size="0.9em" weight={2} />{agentName}</span>
			{/if}
			{#if room.conflictsError}
				<span class="error" title={room.conflictsError}>Conflict fixes could not be looked for.</span>
			{:else if room.conflicts?.truncated}
				<span>Only the newest 20 merges were looked at.</span>
			{/if}
		</div>
	{/if}
</aside>

<style>
	/* The agent's marks (2.0): its colour and its hexagon, kept apart from
	   the viewed tick, which only a person sets. */
	.found,
	.agent-mark,
	.agent-legend {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		color: var(--agent);
		font-size: var(--fs-secondary);
		flex: none;
	}

	.agent-legend {
		gap: 6px;
	}

	.here {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex: none;
		border: 2px solid var(--agent);
	}

	.read {
		color: var(--muted);
	}

	.files {
		width: var(--room-files-w);
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		padding: 4px 8px 10px;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 4px 0 12px;
		overflow: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	li {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--r-panel);
	}

	li:hover:not(.none) {
		background: var(--hover);
	}

	li.on {
		background: color-mix(in srgb, var(--accent) 14%, transparent);
	}

	input {
		width: 16px;
		height: 16px;
		margin: 0;
		flex: none;
		accent-color: var(--ok);
	}

	.pick {
		flex: 1;
		min-width: 0;
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.name {
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--ink);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Read: it steps back, so what is left to read stands out. */
	.name.viewed {
		font-weight: 400;
		color: var(--muted);
	}

	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 0 8px;
		align-items: center;
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.add {
		color: var(--ok);
	}

	.del {
		color: var(--danger);
	}

	.fix,
	.by {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}

	.fix {
		color: var(--resolve);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--resolve);
		flex: none;
	}

	.dot.author {
		background: var(--muted);
	}

	.threads {
		color: var(--accent);
	}

	.legend {
		display: flex;
		flex-direction: column;
		gap: 5px;
		padding: 10px 10px 14px;
		font-size: var(--fs-mono);
	}

	.legend > span {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.error {
		color: var(--danger);
	}
</style>
