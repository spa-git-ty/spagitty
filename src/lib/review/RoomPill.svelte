<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import { room } from './room.svelte';

	/**
	 * The review controls (FEAT-091): a second glass pill floating at the
	 * bottom of the diff column, the way the toolbar floats under the pane.
	 *
	 * The only blurred surface the room adds — WebKitGTK paints blur in
	 * software, and a diff row is never blurred.
	 */

	const file = $derived(room.selectedFile);
	const name = $derived(file ? file.path.slice(file.path.lastIndexOf('/') + 1) : '');
</script>

<div class="pill ornament" role="toolbar" aria-label="Review controls">
	<button class="tool icon" aria-label="Previous file" onclick={() => room.step(-1)}>
		<Icon name="chevron-left" size="1.1em" weight={1.9} />
	</button>
	<span class="where">
		{#if room.layout === 'one'}
			<span class="name mono" title={file?.path}>{name}</span>
			<span class="note">{room.position + 1} of {room.files.length}</span>
		{:else}
			<span class="name mono">All files</span>
			<span class="note">{room.files.length} {room.files.length === 1 ? 'file' : 'files'}</span>
		{/if}
	</span>
	<button class="tool icon" aria-label="Next file" onclick={() => room.step(1)}>
		<Icon name="chevron-right" size="1.1em" weight={1.9} />
	</button>
	<span class="vr"></span>
	<span class="segments">
		<button class="tool" aria-pressed={room.scope === 'changes'} onclick={() => room.setScope('changes')}>
			Changes
		</button>
		<button
			class="tool"
			aria-pressed={room.scope === 'whole'}
			disabled={!room.canShowWhole}
			title={room.canShowWhole ? undefined : 'Needs the pull request fetched'}
			onclick={() => room.setScope('whole')}
		>
			Whole file
		</button>
	</span>
	<span class="segments">
		<button class="tool" aria-pressed={room.layout === 'one'} onclick={() => room.setLayout('one')}>One</button>
		<button class="tool" aria-pressed={room.layout === 'all'} onclick={() => room.setLayout('all')}>All</button>
	</span>
	<span class="vr"></span>
	<button
		class="tool icon toggle"
		aria-pressed={room.ruler}
		aria-label="Focus ruler"
		title="Focus ruler"
		onclick={() => room.toggleRuler()}
	>
		<Icon name="ruler" size="1.1em" weight={1.9} />
	</button>
	<button
		class="tool icon toggle aa"
		aria-pressed={room.readingSet}
		aria-label="Reading font"
		title="Reading font"
		onclick={() => room.toggleReading()}
	>
		Aa
	</button>
	<span class="vr"></span>
	<button class="tool next" disabled={!file} onclick={() => room.viewedNext()}>
		<Icon name="check" size="1em" weight={2.4} />Viewed, next
	</button>
</div>

<style>
	.pill {
		position: absolute;
		left: 50%;
		bottom: 14px;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 6px;
		height: 52px;
		padding: 0 9px;
		border-radius: var(--r-pill);
		white-space: nowrap;
		max-width: calc(100% - 16px);
		z-index: 2;
	}

	.tool {
		display: flex;
		align-items: center;
		gap: 7px;
		height: 34px;
		padding: 0 12px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		color: var(--ink);
		white-space: nowrap;
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	.tool:hover:not(:disabled) {
		background: var(--hover);
	}

	.tool:disabled {
		opacity: 0.4;
	}

	.tool.icon {
		width: 32px;
		padding: 0;
		justify-content: center;
	}

	.where {
		display: flex;
		flex-direction: column;
		line-height: 1.15;
		min-width: 0;
		max-width: 150px;
	}

	.name {
		font-size: var(--fs-mono);
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.where .note {
		font-size: var(--fs-mono);
	}

	/* Two choices in a sunken track; the chosen one raised out of it. */
	.segments {
		display: flex;
		padding: 2px;
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
		border: 1px solid var(--pane-edge);
	}

	.segments .tool {
		height: 28px;
		color: var(--muted);
	}

	.segments .tool[aria-pressed='true'] {
		background: var(--surface-2);
		color: var(--ink);
		font-weight: 600;
		box-shadow: var(--shadow-1);
	}

	.toggle {
		color: var(--muted);
	}

	.toggle[aria-pressed='true'] {
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
	}

	.aa {
		font-weight: 700;
	}

	.next {
		background: var(--ok);
		color: var(--on-accent);
		font-weight: 600;
	}

	.next:hover:not(:disabled) {
		background: color-mix(in srgb, var(--ok) 86%, var(--ink));
	}
</style>
