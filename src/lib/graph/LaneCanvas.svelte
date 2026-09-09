<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { avatars } from '$lib/graph/avatars.svelte';
	import { graph } from '$lib/graph/store.svelte';
	import { drawLanes } from '$lib/graph/lanes';
	import { forgetPortraits } from '$lib/graph/portrait';
	import { LANE_COLOR_COUNT, LANE_SPAN, laneColorVar } from '$lib/metrics';
	import { scale } from '$lib/scale.svelte';
	import { theme } from '$lib/theme.svelte';

	interface Props {
		scrollTop: number;
		first: number;
		last: number;
		/** CSS pixel size of the lane column. */
		width: number;
		height: number;
		/** Lane columns the column is currently sized for. */
		columns: number;
		/** Horizontal room the lanes share, once the column has been sized. */
		span?: number;
		/** Rows to keep at full strength; null when nothing is dimmed. */
		highlight?: Set<number> | null;
		/** Rows carrying stashes, and how many each has. */
		stashes?: Map<number, number>;
	}

	let {
		scrollTop,
		first,
		last,
		width,
		height,
		columns,
		span = LANE_SPAN,
		highlight = null,
		stashes
	}: Props = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);

	/**
	 * Lane colors come from the stylesheet, so switching theme repaints without
	 * any color literal living in TypeScript. Re-resolved whenever the theme
	 * changes; `theme.revision` is read here purely to create that dependency.
	 *
	 * It used to be `theme.id`, which is `family-mode` and named the family as
	 * well as the mode — enough while every palette was one of sixteen written
	 * down in `themes.ts`. A followed desktop palette has no family: every one
	 * of them is `omarchy-dark`, so switching between two dark desktop themes
	 * changed nothing `id` could see and the graph kept the previous theme's
	 * lane colours and its whole cache of portraits (FEAT-080). `revision`
	 * counts changes to the palette's *values*, which is the question a cache
	 * actually has.
	 */
	function resolveColors(el: HTMLElement): { lanes: string[]; nodeRing: string } {
		void theme.revision;
		const styles = getComputedStyle(el);
		const lanes: string[] = [];
		for (let i = 0; i < LANE_COLOR_COUNT; i++) {
			lanes.push(styles.getPropertyValue(laneColorVar(i)).trim() || '#888');
		}
		return {
			lanes,
			// The graph column's own fill: a portrait is ringed in the colour
			// behind it, so the lane line stops at the head rather than running
			// visibly into it.
			nodeRing: styles.getPropertyValue('--graph-bg').trim() || '#888'
		};
	}

	/**
	 * Portraits are rendered with resolved colours, so a theme change makes
	 * every cached face wrong. Dropping them here — beside the one other thing
	 * that reads the palette — keeps the invalidation next to what it
	 * invalidates.
	 */
	$effect(() => {
		void theme.revision;
		forgetPortraits();
	});

	$effect(() => {
		const el = canvas;
		if (!el || width <= 0 || height <= 0) return;

		// Track everything the drawing depends on.
		void graph.version;
		void scrollTop;
		void first;
		void last;
		void columns;
		void span;
		void theme.revision;
		void highlight;
		void stashes;
		// A picture arriving is a repaint: the node it belongs to was drawn
		// with a generated face and now has a real one (FEAT-079).
		void avatars.version;
		// Row pitch and lane spacing both move with these, so a zoom is a
		// repaint even when nothing scrolled.
		const pitch = scale.pitch;
		const zoom = scale.zoom;

		const dpr = window.devicePixelRatio || 1;
		const pixelWidth = Math.round(width * dpr);
		const pixelHeight = Math.round(height * dpr);
		if (el.width !== pixelWidth) el.width = pixelWidth;
		if (el.height !== pixelHeight) el.height = pixelHeight;

		const ctx = el.getContext('2d');
		if (!ctx) return;

		ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

		const { lanes, nodeRing } = resolveColors(el);
		drawLanes({
			ctx,
			width,
			height,
			scrollTop,
			first,
			last,
			row: (index) => graph.row(index),
			colors: lanes,
			nodeRing,
			columns,
			span,
			pitch,
			zoom,
			highlight,
			stashes,
			// Reads what has arrived; never asks. The rows ask — see the note
			// on `picture` in `lanes.ts`.
			picture: (commit) => avatars.drawable(commit.authorEmail ?? '', commit.authorName)
		});
	});
</script>

<!--
	Purely presentational: the rows underneath own the clicks, so this must not
	swallow them.
-->
<canvas
	bind:this={canvas}
	class="lanes"
	style="width: {width}px; height: {height}px"
	aria-hidden="true"
></canvas>

<style>
	/*
	 * Placed by its slot, not by a constant. The wrapper in `CommitRows` is laid
	 * out as part of the same flex row the cells are, so this only has to fill
	 * it — see BUG-003 for what positioning it by hand cost.
	 */
	.lanes {
		position: absolute;
		top: 0;
		left: 0;
		pointer-events: none;
		display: block;
	}
</style>
