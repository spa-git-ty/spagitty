<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import SideBadge from '$lib/merger/SideBadge.svelte';

	/**
	 * The resolver's controls (FEAT-102): a glass pill floating at the foot of
	 * the conflict column, as Review's does under its diff. Previous and next
	 * conflict across every file, the Base strip, whether the three columns
	 * scroll as one (FEAT-128), a side for the current one, and the next one
	 * still unresolved. The only blurred surface the resolver adds; no code row
	 * is ever blurred.
	 *
	 * Once nothing is left there is no next one to go to, so the last place
	 * says *All resolved* and is not a button (BUG-079).
	 */
	interface Props {
		position: number;
		total: number;
		file: string;
		showBase: boolean;
		/** The three columns of a conflict scroll together. */
		linked: boolean;
		/** Every conflict is resolved. */
		done: boolean;
		/** Side buttons are offered: a region with text on both sides. */
		canTake: boolean;
		names: { a: string; b: string };
		onstep: (by: number) => void;
		onbase: () => void;
		onlink: () => void;
		ontake: (side: 'a' | 'b') => void;
		onnext: () => void;
	}

	let { position, total, file, showBase, linked, done, canTake, names, onstep, onbase, onlink, ontake, onnext }: Props =
		$props();
</script>

<div class="pill ornament" role="toolbar" aria-label="Conflict controls">
	<span class="group">
		<button class="tool icon" aria-label="Previous conflict" onclick={() => onstep(-1)}>
			<Icon name="chevron-left" size="1.1em" weight={1.9} />
		</button>
		<span class="where">
			<span class="position">Conflict {position} of {total}</span>
			<span class="note mono file" title={file}>{file}</span>
		</span>
		<button class="tool icon" aria-label="Next conflict" onclick={() => onstep(1)}>
			<Icon name="chevron-right" size="1.1em" weight={1.9} />
		</button>
	</span>
	<span class="vr"></span>
	<span class="group">
		<button class="tool" aria-pressed={showBase} title="Show how it looked before either branch" onclick={onbase}>
			<Icon name="base" size="1em" weight={1.9} />Base
		</button>
		<button
			class="tool"
			aria-pressed={linked}
			title={linked
				? 'The three columns of a conflict scroll as one. Press to scroll each alone.'
				: 'Each column of a conflict scrolls alone. Press to scroll them as one.'}
			onclick={onlink}
		>
			<Icon name="link" size="1em" weight={1.9} />Scroll together
		</button>
	</span>
	<span class="vr"></span>
	<span class="group">
		<button class="tool muted" disabled={!canTake} aria-label="Take {names.a} here" onclick={() => ontake('a')}>
			<SideBadge side="a" />Take
		</button>
		<button class="tool muted" disabled={!canTake} aria-label="Take {names.b} here" onclick={() => ontake('b')}>
			<SideBadge side="b" />Take
		</button>
	</span>
	<span class="vr"></span>
	{#if done}
		<span class="state" role="status">
			<Icon name="check" size="1em" weight={2.4} />All resolved
		</span>
	{:else}
		<button class="tool next" onclick={onnext}>
			<Icon name="check" size="1em" weight={2.4} />Next unresolved
		</button>
	{/if}
</div>

<style>
	/*
	 * Centred by its margins, between the column's two edges. `left: 50%` with
	 * a transform would leave it only half the column to be laid out in, and a
	 * pill wider than that squeezed its own labels (BUG-080).
	 */
	.pill {
		position: absolute;
		left: 8px;
		right: 8px;
		bottom: 14px;
		width: fit-content;
		max-width: calc(100% - 16px);
		margin-inline: auto;
		box-sizing: border-box;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: center;
		gap: 4px 6px;
		min-height: 52px;
		padding: 9px;
		border-radius: 26px;
		white-space: nowrap;
		z-index: 2;
	}

	.group {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.tool {
		display: flex;
		align-items: center;
		gap: 7px;
		height: 34px;
		padding: 0 12px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	.tool:hover:not(:disabled) {
		background: var(--hover);
	}

	.tool:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.tool.icon {
		width: 32px;
		padding: 0;
		justify-content: center;
	}

	.tool.muted {
		color: var(--muted);
	}

	.tool[aria-pressed='true'] {
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
	}

	.where {
		display: flex;
		flex-direction: column;
		line-height: 1.15;
		flex: none;
		max-width: 170px;
	}

	.position {
		font-size: var(--fs-mono);
		font-weight: 600;
	}

	.file {
		font-size: calc(var(--fs-mono) * 0.92);
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.vr {
		width: 1px;
		height: 22px;
		background: var(--soft);
		flex: none;
	}

	.next {
		background: var(--ok);
		color: var(--bg);
		font-weight: 600;
	}

	.next:hover:not(:disabled) {
		background: color-mix(in srgb, var(--ok) 85%, var(--ink));
	}

	/* Said, not offered: no fill, no hover, nothing to press. */
	.state {
		display: flex;
		align-items: center;
		gap: 7px;
		height: 34px;
		padding: 0 12px;
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--ok);
		cursor: default;
		user-select: none;
	}
</style>
