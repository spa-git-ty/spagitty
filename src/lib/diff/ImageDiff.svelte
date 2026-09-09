<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { BinaryDiff } from '$lib/types';

	interface Props {
		diff: BinaryDiff;
	}

	let { diff }: Props = $props();

	type ImageMode = '2-up' | 'swipe' | 'onion';
	let mode = $state<ImageMode>('2-up');
	let swipePos = $state(50); // percentage 0-100
	let opacity = $state(50); // percentage 0-100

	const oldSrc = $derived(diff.oldBase64 ? `data:${diff.mime};base64,${diff.oldBase64}` : null);
	const newSrc = $derived(diff.newBase64 ? `data:${diff.mime};base64,${diff.newBase64}` : null);

	function formatBytes(bytes: number | null): string {
		if (bytes === null) return '0 B';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
	}

	const deltaBytes = $derived.by(() => {
		if (diff.oldSize === null && diff.newSize !== null) return `+${formatBytes(diff.newSize)}`;
		if (diff.oldSize !== null && diff.newSize === null) return `-${formatBytes(diff.oldSize)}`;
		if (diff.oldSize !== null && diff.newSize !== null) {
			const diffSize = diff.newSize - diff.oldSize;
			if (diffSize === 0) return '0 B';
			return diffSize > 0 ? `+${formatBytes(diffSize)}` : `-${formatBytes(Math.abs(diffSize))}`;
		}
		return '0 B';
	});
</script>

<div class="image-diff" aria-label="Image diff visualizer">
	<header class="controls">
		<div class="mode-tabs" role="tablist" aria-label="Comparison mode">
			<button
				type="button"
				class="tab-btn"
				class:active={mode === '2-up'}
				onclick={() => (mode = '2-up')}
			>2-up (Side-by-side)</button>
			<button
				type="button"
				class="tab-btn"
				class:active={mode === 'swipe'}
				disabled={!oldSrc || !newSrc}
				onclick={() => (mode = 'swipe')}
			>Swipe</button>
			<button
				type="button"
				class="tab-btn"
				class:active={mode === 'onion'}
				disabled={!oldSrc || !newSrc}
				onclick={() => (mode = 'onion')}
			>Onion Skin</button>
		</div>

		<div class="meta-stats">
			<span class="mime-badge">{diff.mime}</span>
			<span class="size-badge">
				{formatBytes(diff.oldSize)} → {formatBytes(diff.newSize)} ({deltaBytes})
			</span>
		</div>
	</header>

	<div class="canvas-area">
		{#if mode === '2-up'}
			<div class="two-up-layout">
				<div class="side-frame">
					<div class="frame-label">Before ({formatBytes(diff.oldSize)})</div>
					<div class="checkerboard frame-box">
						{#if oldSrc}
							<img src={oldSrc} alt="Previous revision" class="img-preview" />
						{:else}
							<span class="missing-note">File was added</span>
						{/if}
					</div>
				</div>

				<div class="side-frame">
					<div class="frame-label">After ({formatBytes(diff.newSize)})</div>
					<div class="checkerboard frame-box">
						{#if newSrc}
							<img src={newSrc} alt="New revision" class="img-preview" />
						{:else}
							<span class="missing-note">File was deleted</span>
						{/if}
					</div>
				</div>
			</div>
		{:else if mode === 'swipe'}
			<div class="slider-wrapper">
				<div class="checkerboard slider-canvas">
					{#if newSrc}
						<img src={newSrc} alt="New revision" class="img-base" />
					{/if}
					{#if oldSrc}
						<div class="clip-layer" style="clip-path: inset(0 {100 - swipePos}% 0 0);">
							<img src={oldSrc} alt="Previous revision" class="img-overlay" />
						</div>
					{/if}
					<div class="divider-line" style="left: {swipePos}%;">
						<div class="divider-handle">↔</div>
					</div>
				</div>
				<div class="slider-control">
					<span class="slider-label">Before</span>
					<input
						type="range"
						min="0"
						max="100"
						bind:value={swipePos}
						class="range-input"
						aria-label="Swipe position"
					/>
					<span class="slider-label">After</span>
				</div>
			</div>
		{:else if mode === 'onion'}
			<div class="slider-wrapper">
				<div class="checkerboard onion-canvas">
					{#if oldSrc}
						<img src={oldSrc} alt="Previous revision" class="img-base" />
					{/if}
					{#if newSrc}
						<img
							src={newSrc}
							alt="New revision"
							class="img-overlay"
							style="opacity: {opacity / 100};"
						/>
					{/if}
				</div>
				<div class="slider-control">
					<span class="slider-label">Before (0%)</span>
					<input
						type="range"
						min="0"
						max="100"
						bind:value={opacity}
						class="range-input"
						aria-label="Opacity blend"
					/>
					<span class="slider-label">After (100%)</span>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.image-diff {
		display: flex;
		flex-direction: column;
		flex: 1;
		height: 100%;
		background: var(--bg);
		overflow: hidden;
	}

	.controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 16px;
		background: var(--surface);
		border-bottom: 1px solid var(--line);
		gap: 12px;
		flex-wrap: wrap;
	}

	.mode-tabs {
		display: flex;
		background: var(--sunken);
		border: 1px solid var(--line);
		border-radius: var(--r-field, 6px);
		overflow: hidden;
	}

	.tab-btn {
		background: transparent;
		border: none;
		padding: 6px 12px;
		font-size: var(--fs-secondary);
		color: var(--muted);
		cursor: pointer;
		transition: background 0.1s, color 0.1s;
	}

	.tab-btn:not(:last-child) {
		border-right: 1px solid var(--line);
	}

	.tab-btn.active {
		background: var(--hover);
		color: var(--accent);
		font-weight: 500;
	}

	.tab-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.meta-stats {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: var(--fs-secondary);
	}

	.mime-badge {
		background: var(--soft);
		color: var(--muted);
		padding: 2px 6px;
		border-radius: 4px;
	}

	.size-badge {
		color: var(--ink);
		font-family: var(--font-mono);
	}

	.canvas-area {
		flex: 1;
		padding: 16px;
		overflow: auto;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	/*
	 * What transparency looks like.
	 *
	 * The convention is two neutral squares, and the only thing that matters is
	 * that they differ enough to read as a pattern and little enough not to
	 * compete with the image on top of them. It was two fixed near-blacks, so a
	 * PNG with an alpha channel was checked against a dark grid on every theme,
	 * including the light ones where the grid was the darkest thing on screen.
	 *
	 * Derived from the well the image sits in rather than added to the palette:
	 * a checkerboard is not a colour anybody chooses per theme, and eight
	 * hand-picked pairs is eight chances for one to drift — the argument
	 * `--graph-bg` already makes in `app.css`.
	 */
	.checkerboard {
		background-color: var(--sunken);
		background-image:
			linear-gradient(45deg, var(--soft) 25%, transparent 25%),
			linear-gradient(-45deg, var(--soft) 25%, transparent 25%),
			linear-gradient(45deg, transparent 75%, var(--soft) 75%),
			linear-gradient(-45deg, transparent 75%, var(--soft) 75%);
		background-size: 16px 16px;
		background-position: 0 0, 0 8px, 8px -8px, -8px 0px;
	}

	.two-up-layout {
		display: flex;
		gap: 16px;
		width: 100%;
		height: 100%;
		align-items: stretch;
	}

	.side-frame {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.frame-label {
		font-size: var(--fs-secondary);
		font-weight: 500;
		color: var(--muted);
	}

	.frame-box {
		flex: 1;
		border: 1px solid var(--line);
		border-radius: var(--r-field, 6px);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		min-height: 250px;
		padding: 12px;
	}

	.img-preview {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}

	.missing-note {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.slider-wrapper {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		max-width: 90%;
	}

	.slider-canvas,
	.onion-canvas {
		position: relative;
		border: 1px solid var(--line);
		border-radius: var(--r-field, 6px);
		overflow: hidden;
		min-width: 300px;
		min-height: 250px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.img-base {
		max-width: 600px;
		max-height: 450px;
		display: block;
		object-fit: contain;
	}

	.clip-layer {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.img-overlay {
		max-width: 600px;
		max-height: 450px;
		display: block;
		object-fit: contain;
	}

	.onion-canvas .img-overlay {
		position: absolute;
		inset: 0;
		margin: auto;
	}

	.divider-line {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--accent);
		z-index: 10;
	}

	.divider-handle {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		background: var(--accent);
		color: var(--on-accent);
		font-size: var(--fs-mono);
		font-weight: bold;
		padding: 2px 6px;
		border-radius: 10px;
	}

	.slider-control {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		max-width: 400px;
	}

	.slider-label {
		font-size: var(--fs-secondary);
		color: var(--muted);
		white-space: nowrap;
	}

	.range-input {
		flex: 1;
		accent-color: var(--accent);
	}
</style>
