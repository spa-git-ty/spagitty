<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts" generics="T">
	import { untrack, type Snippet } from 'svelte';

	/**
	 * A scrolling column that draws only the rows near the viewport
	 * (FEAT-091).
	 *
	 * The graph's list can assume every row is one pitch tall. A diff cannot:
	 * a long line wraps, a thread is as tall as what was said in it, and the
	 * reading settings change a line's height. So each row drawn is measured,
	 * and a row not drawn yet is assumed to be its estimate until it is.
	 *
	 * When a row above what is being read changes height — measured for the
	 * first time, or wrapping differently — the column is moved by the same
	 * amount, so the line under the reader's eye stays there. Browsers that
	 * anchor scrolling themselves are told not to, or the two would fight.
	 *
	 * Rows are told apart by `key`, so a measurement belongs to its row
	 * rather than to a position, and survives rows being added above it.
	 */

	interface Props {
		items: T[];
		key: (item: T) => string;
		/** A row's height before it has been measured, in pixels. */
		estimate: (item: T) => number;
		row: Snippet<[T, number]>;
		/** How far past each edge of the viewport rows are drawn, in pixels. */
		overscan?: number;
		/** Space under the last row, for whatever floats over the bottom. */
		tail?: number;
		label?: string;
	}

	let { items, key, estimate, row, overscan = 600, tail = 0, label }: Props = $props();

	/**
	 * Drawn when the viewport has no height to go by — before layout, or in a
	 * test with no layout at all.
	 */
	const BLIND = 400;

	let viewport = $state<HTMLDivElement | null>(null);
	let scrollTop = $state(0);
	let height = $state(0);
	const measured = new Map<string, number>();
	/** Bumped when a measurement changes, which is what re-lays the column. */
	let version = $state(0);

	const keys = $derived(items.map(key));
	const indexOfKey = $derived(new Map(keys.map((k, i) => [k, i])));

	/** `offsets[i]` is the top of row `i`; the last entry is the total. */
	const offsets = $derived.by(() => {
		void version;
		const out = new Float64Array(items.length + 1);
		for (let i = 0; i < items.length; i++) {
			out[i + 1] = out[i] + (measured.get(keys[i]) ?? estimate(items[i]));
		}
		return out;
	});

	/** The first row whose bottom is below `y`. */
	function rowAt(y: number): number {
		let low = 0;
		let high = items.length;
		while (low < high) {
			const mid = (low + high) >> 1;
			if (offsets[mid + 1] <= y) low = mid + 1;
			else high = mid;
		}
		return low;
	}

	const range = $derived.by((): [number, number] => {
		if (height <= 0) return [0, Math.min(items.length, BLIND)];
		const start = rowAt(Math.max(0, scrollTop - overscan));
		const end = Math.min(items.length, rowAt(scrollTop + height + overscan) + 1);
		return [start, end];
	});

	const shown = $derived(items.slice(range[0], range[1]));
	const above = $derived(offsets[range[0]]);
	const below = $derived(offsets[items.length] - offsets[range[1]]);

	let observer: ResizeObserver | null = null;

	function sizeOf(entry: ResizeObserverEntry): number {
		const box = entry.borderBoxSize?.[0];
		return box ? box.blockSize : (entry.target as HTMLElement).getBoundingClientRect().height;
	}

	$effect(() => {
		if (!viewport || typeof ResizeObserver === 'undefined') return;
		const element = viewport;
		height = element.clientHeight;
		const sized = new ResizeObserver(() => {
			height = element.clientHeight;
		});
		sized.observe(element);

		observer = new ResizeObserver((entries) => {
			let shift = 0;
			let changed = false;
			const top = element.scrollTop;
			for (const entry of entries) {
				const k = (entry.target as HTMLElement).dataset.key;
				if (k === undefined) continue;
				const index = indexOfKey.get(k);
				if (index === undefined) continue;
				const next = sizeOf(entry);
				const before = measured.get(k) ?? estimate(items[index]);
				if (Math.abs(next - before) < 0.5) {
					measured.set(k, next);
					continue;
				}
				// A row wholly above the top edge moves what is being read.
				if (offsets[index] + before <= top) shift += next - before;
				measured.set(k, next);
				changed = true;
			}
			if (!changed) return;
			version += 1;
			if (shift !== 0) element.scrollTop = top + shift;
		});
		// Rows drawn before there was an observer to give them to.
		for (const drawn of element.querySelectorAll<HTMLElement>(':scope > .row')) observer.observe(drawn);
		return () => {
			sized.disconnect();
			observer?.disconnect();
			observer = null;
		};
	});

	/** Measure a drawn row for as long as it is drawn. */
	function measure(element: HTMLElement) {
		observer?.observe(element);
		return {
			destroy() {
				observer?.unobserve(element);
			}
		};
	}

	function onscroll() {
		if (viewport) scrollTop = viewport.scrollTop;
	}

	/**
	 * Bring row `index` into view. `nearest` moves only when it is not
	 * already in view; `start` and `center` always move.
	 *
	 * Rows between here and there may not have been measured, so the first
	 * jump lands by estimate and a second, a frame later, by measurement.
	 *
	 * Reads nothing reactively: called from an effect, it would otherwise make
	 * that effect depend on every row measured, and run it again as the reader
	 * scrolls (BUG-047).
	 */
	export function scrollToIndex(index: number, align: 'start' | 'center' | 'nearest' = 'nearest'): void {
		const go = () => {
			const element = viewport;
			if (!element || index < 0 || index >= items.length) return;
			const top = offsets[index];
			const bottom = offsets[index + 1];
			const view = element.clientHeight || height;
			let target = element.scrollTop;
			if (align === 'start') target = top;
			else if (align === 'center') target = top - (view - (bottom - top)) / 2;
			else if (top < element.scrollTop) target = top;
			else if (bottom > element.scrollTop + view - tail) target = bottom - view + tail;
			target = Math.max(0, target);
			if (Math.abs(target - element.scrollTop) >= 1) {
				element.scrollTop = target;
				scrollTop = target;
			}
		};
		untrack(go);
		if (typeof requestAnimationFrame !== 'undefined') requestAnimationFrame(go);
	}

	/** The first row at least partly in view. */
	export function firstVisible(): number {
		return rowAt(scrollTop);
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	class="viewport"
	bind:this={viewport}
	{onscroll}
	role="region"
	aria-label={label}
	tabindex="0"
>
	<div style:height="{above}px"></div>
	{#each shown as item, i (keys[range[0] + i])}
		<div class="row" data-key={keys[range[0] + i]} use:measure>
			{@render row(item, range[0] + i)}
		</div>
	{/each}
	<div style:height="{below + tail}px"></div>
</div>

<style>
	.viewport {
		flex: 1;
		min-height: 0;
		overflow: auto;
		overflow-anchor: none;
		outline: none;
	}

	/* Margins collapse between rows and would not be measured; a row's
	   spacing is its own padding. */
	.row {
		display: flow-root;
	}
</style>
