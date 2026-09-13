// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * FEAT-081 at the component level: the graph header's drag lifecycle, its
 * compact state, and what hovering a row shows.
 *
 * The geometry and the lifecycle each have their own unit tests
 * (`metrics.test.ts`, `resize-drag.test.ts`, `peek.test.ts`,
 * `highlight.test.ts`). These check the wiring — that the real header hands
 * the pointer to the lifecycle, that the rows read the store the header
 * writes, and that a hover never selects anything.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, render } from '../../testing/mount';
import type { GraphRow, RefChip } from '$lib/types';

vi.mock('$lib/graph/store.svelte', async () => await import('../../testing/graph-store.svelte'));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/api', async (original) => ({
	...(await original<typeof import('$lib/api')>()),
	commitDetail: vi.fn()
}));

import * as api from '$lib/api';
import { calls, control } from '../../testing/graph-store.svelte';
import { columns } from './columns.svelte';
import CommitRows from './CommitRows.svelte';

const commitDetail = vi.mocked(api.commitDetail);

function row(index: number, overrides: Partial<GraphRow> = {}): GraphRow {
	return {
		index,
		id: `${index}`.padStart(40, 'a'),
		short: `${index}`.padStart(7, 'a'),
		summary: `commit ${index}`,
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		initials: 'AL',
		time: 1_700_000_000 - index * 60,
		lane: 0,
		color: 0,
		signed: false,
		parents: [],
		refs: [],
		edges: [],
		...overrides
	};
}

function chip(name: string): RefChip {
	return { name, kind: 'branch', current: false, local: true, remotes: [], divergence: null };
}

/** Frames the test releases by hand. */
let frames: FrameRequestCallback[] = [];
function tick() {
	const due = frames;
	frames = [];
	for (const frame of due) frame(0);
	flushSync();
}

function pointer(target: EventTarget, type: string, clientX: number) {
	target.dispatchEvent(
		new PointerEvent(type, { bubbles: true, cancelable: true, clientX, clientY: 5, pointerId: 1 })
	);
	flushSync();
}

let store: Map<string, string>;

beforeEach(() => {
	control.reset();
	columns.reset();
	frames = [];
	store = new Map();
	vi.stubGlobal('localStorage', {
		getItem: (key: string) => store.get(key) ?? null,
		setItem: vi.fn((key: string, value: string) => store.set(key, value)),
		removeItem: (key: string) => store.delete(key)
	});
	vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
		frames.push(callback);
		return frames.length;
	});
	vi.stubGlobal('cancelAnimationFrame', () => {
		frames = [];
	});
	commitDetail.mockReset();
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

/** The Graph header cell's divider — the one that sizes the graph. */
function graphDivider(view: ReturnType<typeof render>): HTMLElement {
	return view.get('.header-scroll [data-column="graph"] .divider');
}

describe('dragging the graph column', () => {
	it('applies once per frame, and saves once on release', () => {
		control.setRows([row(0), row(1)]);
		const view = render(CommitRows, {});
		columns.open('/repo/a');
		flushSync();
		const setItem = vi.mocked(localStorage.setItem);
		setItem.mockClear();
		const divider = graphDivider(view);

		pointer(divider, 'pointerdown', 200);
		expect(columns.width('graph')).toBe(0);

		for (let x = 199; x >= 120; x--) pointer(divider, 'pointermove', x);
		expect(columns.width('graph')).toBe(0);
		expect(frames).toHaveLength(1);

		tick();
		// The header cell measured 0 under happy-dom, so the width is the
		// travel from there, clamped to the column's minimum.
		expect(columns.width('graph')).toBe(40);
		expect(setItem).not.toHaveBeenCalled();

		pointer(divider, 'pointerup', 120);
		expect(setItem).toHaveBeenCalledTimes(1);
		expect(frames).toHaveLength(0);

		// Header and rows read the same width.
		const cell = view.get('.header-scroll [data-column="graph"]');
		expect(cell.style.width).toBe('40px');
		expect(view.get('.lane-space').style.width).toBe('40px');

		view.destroy();
	});

	it('ends on cancellation and on lost capture, leaving nothing pending', () => {
		control.setRows([row(0)]);
		const view = render(CommitRows, {});
		columns.open('/repo/b');
		const divider = graphDivider(view);
		columns.resize('graph', 200);
		flushSync();

		pointer(divider, 'pointerdown', 0);
		pointer(divider, 'pointermove', 30);
		pointer(divider, 'pointercancel', 0);
		expect(frames).toHaveLength(0);

		// A move after the drag ended changes nothing.
		pointer(divider, 'pointermove', 90);
		tick();
		const afterCancel = columns.width('graph');

		pointer(divider, 'pointerdown', 0);
		pointer(divider, 'pointermove', -10);
		divider.dispatchEvent(new PointerEvent('lostpointercapture', { pointerId: 1 }));
		flushSync();
		expect(frames).toHaveLength(0);
		pointer(divider, 'pointermove', 400);
		tick();
		expect(columns.width('graph')).toBe(afterCancel);

		view.destroy();
	});

	it('does not size the column on a press that never moves', () => {
		control.setRows([row(0)]);
		const view = render(CommitRows, {});
		const divider = graphDivider(view);
		pointer(divider, 'pointerdown', 50);
		pointer(divider, 'pointerup', 50);
		expect(columns.width('graph')).toBe(0);
		view.destroy();
	});

	it('flushes and saves a drag still live when the view is torn down', () => {
		control.setRows([row(0)]);
		const view = render(CommitRows, {});
		columns.open('/repo/c');
		columns.resize('graph', 200);
		flushSync();
		const divider = graphDivider(view);
		const setItem = vi.mocked(localStorage.setItem);
		setItem.mockClear();

		pointer(divider, 'pointerdown', 0);
		pointer(divider, 'pointermove', 25);
		view.destroy();

		expect(frames).toHaveLength(0);
		expect(setItem).toHaveBeenCalledTimes(1);
	});
});

describe('the compact graph header', () => {
	it('shows the icon, keeps its name and its divider, when too narrow for the word', () => {
		control.setRows([row(0)]);
		columns.resize('graph', 60);
		const view = render(CommitRows, {});

		const cell = view.get('.header-scroll [data-column="graph"]');
		expect(cell.classList.contains('compact')).toBe(true);
		expect(cell.querySelector('svg')).toBeTruthy();
		expect(cell.textContent?.trim()).toBe('');
		expect(cell.getAttribute('aria-label')).toBe('Graph');
		expect(cell.querySelector('.divider')).toBeTruthy();

		view.destroy();
	});

	it('shows the word at an ordinary width', () => {
		control.setRows([row(0)]);
		const view = render(CommitRows, {});
		const cell = view.get('.header-scroll [data-column="graph"]');
		expect(cell.classList.contains('compact')).toBe(false);
		expect(cell.textContent).toContain('Graph');
		view.destroy();
	});
});

describe('hovering a row', () => {
	const tip = row(0, { id: 'tip', refs: [chip('feature')], parents: ['mid'] });
	const mid = row(1, { id: 'mid', parents: ['base'] });
	const base = row(2, { id: 'base' });

	it('names a bare commit’s branch faintly, on that row only, and dims nothing', () => {
		control.setRows([tip, mid, base]);
		const view = render(CommitRows, {});
		const options = view.all('[role="option"]');

		options[2].dispatchEvent(new PointerEvent('pointerenter'));
		flushSync();

		const context = view.all('.context-branch');
		expect(context).toHaveLength(1);
		expect(context[0].textContent?.trim()).toBe('feature');
		expect(options[2].contains(context[0])).toBe(true);
		expect(view.all('.row.dim')).toHaveLength(0);

		// A row with its own chip gets no caption.
		options[2].dispatchEvent(new PointerEvent('pointerleave'));
		options[0].dispatchEvent(new PointerEvent('pointerenter'));
		flushSync();
		expect(view.all('.context-branch')).toHaveLength(0);

		view.destroy();
	});

	it('shows the full message after a rest, without selecting anything', async () => {
		vi.useFakeTimers();
		control.setRows([tip, mid, base]);
		commitDetail.mockResolvedValue({
			id: 'mid',
			short: 'mid',
			summary: 'commit 1',
			body: 'The body.\n\nSigned-off-by: Ada <ada@example.com>'
		} as Awaited<ReturnType<typeof api.commitDetail>>);
		const view = render(CommitRows, {});
		const message = view.all('.cell.message')[1];

		message.dispatchEvent(new PointerEvent('pointerenter', { clientX: 10, clientY: 10 }));
		await vi.advanceTimersByTimeAsync(1000);
		flushSync();

		const shown = view.find('.peek') ?? document.querySelector<HTMLElement>('.peek');
		expect(shown?.textContent).toBe(
			'commit 1\n\nThe body.\n\nSigned-off-by: Ada <ada@example.com>'
		);
		expect(calls.selected).toEqual([]);
		expect(message.querySelector('.summary')?.hasAttribute('title')).toBe(false);

		message.dispatchEvent(new PointerEvent('pointerleave'));
		flushSync();
		expect(document.querySelector('.peek')).toBeNull();

		view.destroy();
	});
});
