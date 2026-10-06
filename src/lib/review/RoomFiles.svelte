<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { room } from './room.svelte';

	/**
	 * The files a pull request touches, each with its viewed tick
	 * (FEAT-091).
	 *
	 * A tick is kept against the file's content, not its name: when the author
	 * changes a ticked file, it comes back unticked, and a file still bold is
	 * one still to read.
	 */

	function name(path: string): string {
		return path.slice(path.lastIndexOf('/') + 1);
	}
</script>

<aside class="files" aria-label="Touched files">
	<ul>
		{#each room.files as file (file.path)}
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
						{#if threads > 0}<span class="threads">{threads} {threads === 1 ? 'thread' : 'threads'}</span>{/if}
					</span>
				</button>
			</li>
		{/each}
	</ul>
</aside>

<style>
	.files {
		width: 262px;
		flex: none;
		display: flex;
		flex-direction: column;
		min-height: 0;
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

	li:hover {
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
		gap: 8px;
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

	.threads {
		color: var(--accent);
	}
</style>
