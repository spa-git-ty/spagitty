<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import SideBadge from '$lib/merger/SideBadge.svelte';

	/**
	 * The resolver's controls (FEAT-102): a glass pill floating at the foot of
	 * the conflict column, as Review's does under its diff. Previous and next
	 * conflict across every file, the Base strip, a side for the current one,
	 * and the next one still unresolved. The only blurred surface the resolver
	 * adds; no code row is ever blurred.
	 */
	interface Props {
		position: number;
		total: number;
		file: string;
		showBase: boolean;
		/** Every conflict is resolved. */
		done: boolean;
		/** Side buttons are offered: a region with text on both sides. */
		canTake: boolean;
		names: { a: string; b: string };
		onstep: (by: number) => void;
		onbase: () => void;
		ontake: (side: 'a' | 'b') => void;
		onnext: () => void;
	}

	let { position, total, file, showBase, done, canTake, names, onstep, onbase, ontake, onnext }: Props = $props();
</script>

<div class="pill ornament" role="toolbar" aria-label="Conflict controls">
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
	<span class="vr"></span>
	<button class="tool" aria-pressed={showBase} title="Show how it looked before either branch" onclick={onbase}>
		<Icon name="base" size="1em" weight={1.9} />Base
	</button>
	<button class="tool muted" disabled={!canTake} aria-label="Take {names.a} here" onclick={() => ontake('a')}>
		<SideBadge side="a" />Take
	</button>
	<button class="tool muted" disabled={!canTake} aria-label="Take {names.b} here" onclick={() => ontake('b')}>
		<SideBadge side="b" />Take
	</button>
	<span class="vr"></span>
	<button class="tool next" class:done onclick={onnext}>
		<Icon name="check" size="1em" weight={2.4} />{done ? 'All resolved' : 'Next unresolved'}
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
		min-width: 0;
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

	.next.done {
		background: none;
		color: var(--ok);
	}
</style>
