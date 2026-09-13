// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, press, render } from '../../testing/mount';
import { LANE_COLUMNS_MAX, LANE_COLUMNS_MIN, ROW_PITCH, laneColumnWidth } from '../metrics';
import type { GraphRow, RefChip } from '$lib/types';

/**
 * A stand-in for the graph store. The real one is driven by Tauri events and is
 * tested separately; here the rows are the input and the DOM is the output.
 */
vi.mock('$lib/graph/store.svelte', async () => await import('../../testing/graph-store.svelte'));
// `overlay.wip` reads `repo.counts`, which is what puts the uncommitted-changes
// row on screen at all.
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import { readFileSync } from 'node:fs';
import { calls, control } from '../../testing/graph-store.svelte';
import { control as repoControl } from '../../testing/repo-store.svelte';
import { columns } from './columns.svelte';

const componentSource = readFileSync('src/lib/graph/CommitRows.svelte', 'utf8');
import CommitRows from './CommitRows.svelte';
import LaneCanvas from './LaneCanvas.svelte';

function row(index: number, overrides: Partial<GraphRow> = {}): GraphRow {
	return {
		index,
		id: `${index}`.padStart(40, 'a'),
		short: `${index}`.padStart(7, 'a'),
		summary: `commit ${index}`,
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		initials: 'AL',
		// One row per day, so every row is a landmark unless a test says otherwise.
		time: 1_700_000_000 - index * 86_400,
		lane: 0,
		color: 0,
		signed: false,
		parents: [],
		refs: [],
		edges: [],
		...overrides
	};
}

function chip(name: string, kind: RefChip['kind'] = 'branch', current = false): RefChip {
	return { name, kind, current, local: kind !== 'remote', remotes: [], divergence: null };
}

beforeEach(() => {
	control.reset();
	// The column store is a module singleton: a width or an order set by one
	// test is still set in the next one. Every test here that touches a column
	// used to have to be the first to do so.
	columns.reset();
	control.setRows([row(0), row(1), row(2)]);
	vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => {}, removeItem: () => {} });
});

describe('CommitRows', () => {
	it('is a listbox of options', () => {
		const view = render(CommitRows, {});

		expect(view.get('.scroller').getAttribute('role')).toBe('listbox');
		expect(view.all('[role="option"]')).toHaveLength(3);

		view.destroy();
	});

	/**
	 * FEAT-019. `S` rather than a tick: the tick already means "the branch you
	 * are on" on a RefChip, and one mark cannot mean two things on one screen.
	 */
	it('marks a signed commit, and only a signed one', () => {
		control.setRows([row(0, { signed: true }), row(1, { signed: false })]);
		const view = render(CommitRows, {});

		const marks = view.all('.signed');
		expect(marks).toHaveLength(1);
		expect(marks[0].textContent?.trim()).toBe('S');

		view.destroy();
	});

	it('does not claim a signature it has not verified', () => {
		// The whole reason this is cheap enough to put on every row: it reads a
		// header. Saying "verified" would be a claim nothing here checked.
		control.setRows([row(0, { signed: true })]);
		const view = render(CommitRows, {});

		const title = view.get('.signed').getAttribute('title') ?? '';
		expect(title).toContain('does not verify');
		expect(view.text()).not.toContain('verified');

		view.destroy();
	});

	it('shows no edge shadow when there is nothing under it', () => {
		// happy-dom lays nothing out, so `scrollWidth` is 0 and the table cannot
		// overflow. That is the case worth pinning anyway: the affordance must
		// be absent when it would be a lie.
		const view = render(CommitRows, {});

		for (const edge of view.all('.edge')) {
			expect(edge.classList.contains('showing')).toBe(false);
		}

		view.destroy();
	});

	it('puts an edge on each side, and lets clicks through both', () => {
		// They sit over the rows, so anything they cover has to stay reachable.
		const view = render(CommitRows, {});

		const edges = view.all('.edge');
		expect(edges).toHaveLength(2);
		expect(view.all('.edge.left')).toHaveLength(1);
		expect(view.all('.edge.right')).toHaveLength(1);
		for (const edge of edges) {
			expect(edge.getAttribute('aria-hidden')).toBe('true');
		}

		view.destroy();
	});

	it('sizes the scroll area to the whole history, not to the rows drawn', () => {
		control.setRows(Array.from({ length: 5000 }, (_, i) => row(i)));
		const view = render(CommitRows, {});

		expect(view.get('.sizer').style.height).toBe(`${5000 * ROW_PITCH}px`);
		// Virtualized: only a screenful exists as DOM nodes.
		expect(view.all('.row').length).toBeLessThan(50);

		view.destroy();
	});

	it('positions each row at its own index, not at its position in the batch', () => {
		const view = render(CommitRows, {});
		const rows = view.all('.row');

		expect(rows[1].style.transform).toBe(`translateY(${ROW_PITCH}px)`);
		expect(rows[2].style.transform).toBe(`translateY(${2 * ROW_PITCH}px)`);

		view.destroy();
	});

	it('selects a row on click and opens it on double-click', () => {
		const onopen = vi.fn();
		const view = render(CommitRows, { onopen });

		view.all('.row')[1].dispatchEvent(new MouseEvent('click', { bubbles: true }));
		flushSync();
		expect(calls.selected).toEqual([1]);

		view.all('.row')[1].dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
		flushSync();
		expect(onopen).toHaveBeenCalledWith(row(1).id);

		view.destroy();
	});

	it('marks the selected option for assistive technology as well as visually', () => {
		control.setSelected(1);
		const view = render(CommitRows, {});

		const selected = view.all('.row')[1];
		expect(selected.classList.contains('selected')).toBe(true);
		expect(selected.getAttribute('aria-selected')).toBe('true');
		expect(view.get('.scroller').getAttribute('aria-activedescendant')).toBe('commit-1');

		view.destroy();
	});

	it('moves the selection with the arrow keys', () => {
		const view = render(CommitRows, {});
		const scroller = view.get('.scroller');

		press(scroller, 'ArrowDown');
		expect(calls.selected.at(-1)).toBe(0);

		press(scroller, 'ArrowDown');
		expect(calls.selected.at(-1)).toBe(1);

		press(scroller, 'ArrowUp');
		expect(calls.selected.at(-1)).toBe(0);

		view.destroy();
	});

	it('stops at either end rather than wrapping', () => {
		const view = render(CommitRows, {});
		const scroller = view.get('.scroller');

		press(scroller, 'ArrowUp');
		expect(calls.selected.at(-1)).toBe(0);

		press(scroller, 'End');
		expect(calls.selected.at(-1)).toBe(2);
		press(scroller, 'ArrowDown');
		expect(calls.selected.at(-1)).toBe(2);

		press(scroller, 'Home');
		expect(calls.selected.at(-1)).toBe(0);

		view.destroy();
	});

	it('opens the selected commit with Enter', () => {
		const onopen = vi.fn();
		control.setSelected(2);
		const view = render(CommitRows, { onopen });

		press(view.get('.scroller'), 'Enter');
		expect(onopen).toHaveBeenCalledWith(row(2).id);

		view.destroy();
	});

	it('does nothing on Enter with no selection', () => {
		const onopen = vi.fn();
		const view = render(CommitRows, { onopen });

		press(view.get('.scroller'), 'Enter');
		expect(onopen).not.toHaveBeenCalled();

		view.destroy();
	});

	it('leaves keys it does not handle alone', () => {
		const view = render(CommitRows, {});
		const event = press(view.get('.scroller'), 'x');

		expect(event.defaultPrevented).toBe(false);
		expect(calls.selected).toEqual([]);

		view.destroy();
	});

	it('asks the store for more history as the viewport reaches the end', () => {
		const view = render(CommitRows, {});
		expect(calls.ensured.length).toBeGreaterThan(0);
		view.destroy();
	});

	it('scrolls the selection back into view when it moves off screen', () => {
		control.setRows(Array.from({ length: 100 }, (_, i) => row(i)));
		const view = render(CommitRows, {});
		const scroller = view.get('.scroller');
		// happy-dom lays nothing out, so the viewport has to be declared.
		Object.defineProperty(scroller, 'clientHeight', { value: 10 * ROW_PITCH });
		scroller.scrollTop = 0;

		press(scroller, 'End');
		expect(scroller.scrollTop).toBeGreaterThan(0);

		press(scroller, 'Home');
		expect(scroller.scrollTop).toBe(0);

		view.destroy();
	});

	it('shows at most two ref chips and counts the rest', () => {
		control.setRows([
			row(0, { refs: [chip('main', 'branch', true), chip('origin/main', 'remote'), chip('v1')] })
		]);
		const view = render(CommitRows, {});

		expect(view.all('.refs .ref')).toHaveLength(2);
		const more = view.get('.more');
		expect(more.textContent?.trim()).toBe('+1');
		// The overflow still names every ref, so nothing becomes unreachable.
		expect(more.getAttribute('title')).toBe('main, origin/main, v1');

		view.destroy();
	});

	it('shows no overflow marker when everything fits', () => {
		control.setRows([row(0, { refs: [chip('main', 'branch', true)] })]);
		const view = render(CommitRows, {});
		expect(view.find('.more')).toBeNull();
		view.destroy();
	});

	it('joins a labelled row to its node, and only a labelled one', () => {
		control.setRows([row(0, { refs: [chip('main', 'branch', true)] }), row(1)]);
		const view = render(CommitRows, {});
		flushSync();

		const labelled = view.get('#commit-0');
		const bare = view.get('#commit-1');
		expect(labelled.querySelectorAll('.ref-lead')).toHaveLength(2);
		expect(bare.querySelector('.ref-lead')).toBeNull();

		const graphLead = labelled.querySelector('.graph-lead') as HTMLElement;
		expect(graphLead.style.width).not.toBe('');
		expect(graphLead.style.width).not.toBe('0px');

		expect(componentSource).toMatch(/\.refs \{[^}]*justify-content: flex-start/);

		view.destroy();
	});

	it('does not rule the graph column into a table', () => {
		expect(componentSource).not.toMatch(/--graph-line/);
		expect(componentSource).toMatch(/\.lane-band \{[^}]*background: var\(--graph-bg\)/);
		expect(componentSource).not.toMatch(/\.lane-band \{[^}]*box-shadow/);
	});

	it('labels only the first row of each day', () => {
		const day = 86_400;
		const base = 1_700_000_000;
		control.setRows([
			row(0, { time: base }),
			row(1, { time: base - 60 }), // same day
			row(2, { time: base - day }) // the day before
		]);
		const view = render(CommitRows, {});
		const times = view.all('.row').map((r) => r.querySelector('.when'));

		expect(times[0]).not.toBeNull();
		expect(times[1]).toBeNull();
		expect(times[2]).not.toBeNull();

		view.destroy();
	});

	/**
	 * FEAT-031 request 4. `CommitRows` has always taken an `onwip` prop and
	 * `src/routes/+page.svelte` never passed one, so the row was dead: clicking
	 * "Uncommitted changes" did nothing at all.
	 */
	it('opens the working copy when the uncommitted-changes row is clicked', () => {
		const onwip = vi.fn();
		repoControl.setCounts({
			commits: 3,
			branches: 1,
			working: 3,
			staged: 2,
			conflicts: 0,
			stashes: 0,
			tags: 0,
			submodules: 0
		});
		const view = render(CommitRows, { onwip });

		const row = view.get('.wip');
		expect(row.tagName).toBe('BUTTON');
		expect(row.getAttribute('title')).toBe('Open the working copy');

		row.dispatchEvent(new MouseEvent('click', { bubbles: true }));
		flushSync();

		expect(onwip).toHaveBeenCalledTimes(1);
		view.destroy();
	});

	/** The graph screen must actually pass the handler, which is what was missing. */
	it('is given a handler by the graph screen', () => {
		const page = readFileSync('src/routes/+page.svelte', 'utf8');
		expect(page).toMatch(/<CommitRows[^>]*onwip=/);
	});

	it('starts at the design lane width', () => {
		const view = render(CommitRows, {});
		expect(view.get('.lane-space').style.width).toBe(`${laneColumnWidth(LANE_COLUMNS_MIN)}px`);
		view.destroy();
	});

	it('widens the lane column immediately for a deeper history', () => {
		control.setRows([row(0), row(1, { lane: 8, edges: [{ from: 0, to: 8, color: 1 }] })]);
		const view = render(CommitRows, {});

		expect(view.get('.lane-space').style.width).toBe(`${laneColumnWidth(9)}px`);
		view.destroy();
	});

	/**
	 * FEAT-035. Past the cap the column stops growing and the pitch gives
	 * instead, so a busy history cannot push the message column off the screen.
	 * Asserted at the component because this is the property the author asked
	 * for — `metrics.test.ts` proves the geometry, this proves it is wired.
	 */
	it('stops widening the lane column at the cap, however deep the history', () => {
		const atCap = `${laneColumnWidth(LANE_COLUMNS_MAX)}px`;

		for (const deepest of [LANE_COLUMNS_MAX, 20, 60, 200]) {
			control.setRows([
				row(0),
				row(1, { lane: deepest - 1, edges: [{ from: 0, to: deepest - 1, color: 1 }] })
			]);
			const view = render(CommitRows, {});

			expect(
				view.get('.lane-space').style.width,
				`${deepest} lanes widened the column past the cap`
			).toBe(atCap);

			view.destroy();
		}
	});

	it('does not narrow the lane column the instant the history does', () => {
		// Shrinking on sight would make the message column jump left and right
		// under the reader's eyes while scrolling.
		vi.useFakeTimers();
		control.setRows([row(0, { lane: 8, edges: [{ from: 0, to: 8, color: 1 }] })]);
		const view = render(CommitRows, {});
		const wide = view.get('.lane-space').style.width;
		expect(wide).toBe(`${laneColumnWidth(9)}px`);

		control.setRows([row(0)]);
		flushSync();
		expect(view.get('.lane-space').style.width).toBe(wide);

		vi.advanceTimersByTime(500);
		flushSync();
		expect(view.get('.lane-space').style.width).toBe(`${laneColumnWidth(LANE_COLUMNS_MIN)}px`);

		view.destroy();
		vi.useRealTimers();
	});
});

describe('the column bed', () => {
	// BUG-016: the graph's band was painted by a cell inside a row, so on a
	// repository shorter than the window it stopped at the last commit and the
	// table looked cut off. The bed paints it for the whole height, under the
	// rows.

	it('gives every shown column a slot, in the same order as a row', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		const slots = [...view.get('.bed-scroll').children];

		expect(slots).toHaveLength(2);
		// Default order is refs, graph, then the pinned list pane.
		expect((slots[0] as HTMLElement).style.width).toBe(`${columns.shown[0].width}px`);
		expect((slots[1] as HTMLElement).classList.contains('lane-band')).toBe(true);

		view.destroy();
	});

	it('paints the band with the class the row graph cell also wears', () => {
		// One declaration for both, so the band cannot come out one colour where
		// there are commits and another where there are none.
		control.setRows([row(0)]);
		const view = render(CommitRows, {});
		flushSync();

		expect(view.get('.lane-space').classList.contains('lane-band')).toBe(true);
		expect(view.get('.bed .lane-band')).toBeTruthy();

		view.destroy();
	});

	it('follows a column being resized, exactly as the rows do', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		columns.resize('refs', 120);
		flushSync();

		expect((view.get('.bed-scroll').children[0] as HTMLElement).style.width).toBe('120px');

		view.destroy();
	});

	it('follows a reorder, so the band never lands on the wrong column', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		columns.reorder(1, 0);
		flushSync();

		const slots = [...view.get('.bed-scroll').children];
		expect((slots[0] as HTMLElement).classList.contains('lane-band')).toBe(true);

		view.destroy();
	});

	it('is there when the repository has no commits at all', () => {
		// The empty case is the whole point: nothing draws the columns but this.
		control.setRows([]);
		const view = render(CommitRows, {});
		flushSync();

		expect(view.get('.bed-scroll').children).toHaveLength(2);
		expect(view.get('.bed-frozen')).toBeTruthy();
		expect(view.all('.lane-space')).toHaveLength(0);

		view.destroy();
	});

	it('is sized by the scroller rather than by the rows', () => {
		// `.sizer` is `rows x pitch` tall; the bed must not be, or it would stop
		// at the last commit again by another route.
		control.setRows([row(0)]);
		const view = render(CommitRows, {});
		flushSync();

		const bed = view.get('.bed');
		expect(bed.parentElement?.classList.contains('rows')).toBe(true);
		expect(bed.closest('.sizer')).toBeNull();
		expect(bed.closest('.scroller')).toBeNull();

		view.destroy();
	});
});

describe('the lane canvas layer', () => {
	// BUG-003: the canvas was positioned at the design's 186px gutter, so any
	// column width the user actually chose left it painting over the messages.
	// These assert it is laid out *with* the columns instead.

	it('puts the canvas after exactly the columns that precede the graph', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		const layer = view.get('.lane-layer');
		const slots = [...layer.children];
		const graphSlot = view.get('.lane-slot');

		// Default order is refs, graph, then the pinned list pane: one spacer,
		// then the canvas. The message column is no longer a gap in this layer
		// — the graph slides under it instead.
		expect(slots.indexOf(graphSlot)).toBe(1);
		expect(slots).toHaveLength(2);
		expect((slots[0] as HTMLElement).style.width).toBe(`${columns.shown[0].width}px`);

		view.destroy();
	});

	it('follows the Branch/Tag column when it is resized', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		columns.resize('refs', 120);
		flushSync();

		const spacer = view.get('.lane-layer').children[0] as HTMLElement;
		expect(spacer.style.width).toBe('120px');
		expect(spacer.style.width).not.toBe('186px');

		view.destroy();
	});

	it('has nothing in front of it when the graph is the first column', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		columns.reorder(1, 0);
		flushSync();

		const slots = [...view.get('.lane-layer').children];
		expect(slots.indexOf(view.get('.lane-slot'))).toBe(0);

		view.destroy();
	});

	it('does not stripe alternate rows', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		for (const option of view.all('[role="option"]')) {
			expect(option.classList.contains('stripe')).toBe(false);
		}
		expect(componentSource).not.toMatch(/class:stripe/);
		expect(componentSource).not.toMatch(/\.row\.stripe/);

		view.destroy();
	});

	it('clips the canvas to its own column, so a wrong size cannot reach a neighbour', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		const slot = view.get('.lane-slot');
		expect(slot.style.width).toBe(`${laneColumnWidth(LANE_COLUMNS_MIN)}px`);
		// The rule lives in the component's stylesheet, which the test
		// environment does not apply — assert it is declared rather than
		// computed, the same way the Btn regression does.
		expect(componentSource).toContain('.lane-slot');
		expect(componentSource).toMatch(/\.lane-slot \{[^}]*overflow: hidden/);

		view.destroy();
	});
});

describe('the pinned commit list', () => {
	/**
	 * Sideways scrolling used to translate the whole table, so the subject
	 * line ran away under the eye whenever somebody panned to see more of the
	 * graph. The list pane stays; the graph slides under it.
	 */
	it('keeps the message column sticky, with a seam shadow', () => {
		control.setRows([row(0), row(1), row(2)]);
		const view = render(CommitRows, {});
		flushSync();

		const message = view.get('.message');
		expect(message.classList.contains('frozen')).toBe(true);
		expect(message.style.position).toBe('sticky');
		expect(view.get('.header-frozen')).toBeTruthy();
		expect(view.get('.list-shadow')).toBeTruthy();
		expect(view.get('.bed-frozen')).toBeTruthy();

		view.destroy();
	});
});

describe('LaneCanvas', () => {
	/** happy-dom has no canvas; this is the smallest context the drawing uses. */
	function stubCanvas() {
		const seen: string[] = [];
		const ctx = new Proxy(
			{ canvas: {} },
			{
				get(target: Record<string, unknown>, key: string) {
					if (key === 'canvas') return target.canvas;
					return (...args: unknown[]) => {
						seen.push(key);
						void args;
					};
				},
				set: () => true
			}
		);
		HTMLCanvasElement.prototype.getContext = (() =>
			ctx) as unknown as HTMLCanvasElement['getContext'];
		vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue: () => '#123456' }));
		return seen;
	}

	it('sizes its backing store for the device pixel ratio', () => {
		stubCanvas();
		vi.stubGlobal('devicePixelRatio', 2);

		const view = render(LaneCanvas, {
			scrollTop: 0,
			first: 0,
			last: 2,
			width: 150,
			height: 400,
			columns: LANE_COLUMNS_MIN
		});

		const canvas = view.get('canvas') as HTMLCanvasElement;
		expect(canvas.width).toBe(300);
		expect(canvas.height).toBe(800);
		// CSS size stays in CSS pixels, or the column would be twice as wide.
		expect(canvas.style.width).toBe('150px');

		view.destroy();
	});

	it('draws the lanes', () => {
		const seen = stubCanvas();
		vi.stubGlobal('devicePixelRatio', 1);

		const view = render(LaneCanvas, {
			scrollTop: 0,
			first: 0,
			last: 2,
			width: 150,
			height: 400,
			columns: LANE_COLUMNS_MIN
		});

		expect(seen).toContain('clearRect');
		expect(seen).toContain('arc');

		view.destroy();
	});

	it('draws nothing before it has been given a size', () => {
		const seen = stubCanvas();

		const view = render(LaneCanvas, {
			scrollTop: 0,
			first: 0,
			last: 2,
			width: 150,
			height: 0,
			columns: LANE_COLUMNS_MIN
		});

		expect(seen).toHaveLength(0);
		view.destroy();
	});

	it('does not swallow clicks meant for the rows underneath', () => {
		stubCanvas();
		const view = render(LaneCanvas, {
			scrollTop: 0,
			first: 0,
			last: 0,
			width: 150,
			height: 100,
			columns: LANE_COLUMNS_MIN
		});

		expect(view.get('canvas').getAttribute('aria-hidden')).toBe('true');
		view.destroy();
	});
});
