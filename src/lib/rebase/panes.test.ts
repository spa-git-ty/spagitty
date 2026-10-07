// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, press, render } from '../../testing/mount';
import type { PreviewRow, RebasePreview, RebaseTodo, TodoRow } from '$lib/types';

vi.mock('$lib/api', () => ({
	rebaseTodo: vi.fn(),
	rebasePreview: vi.fn()
}));

import * as api from '$lib/api';
import RebaseHistory from './RebaseHistory.svelte';
import { rebase } from './store.svelte';
import RebasePlan from './RebasePlan.svelte';

const rebaseTodo = vi.mocked(api.rebaseTodo);
const rebasePreview = vi.mocked(api.rebasePreview);

function todoRow(n: number): TodoRow {
	return {
		id: `${n}`.repeat(40),
		short: `${n}`.repeat(7),
		summary: `Commit ${n}`,
		authorName: 'Ada Lovelace',
		time: 1_700_000_000 + n,
		paths: [`file${n}.txt`]
	};
}

function todoOf(count = 3): RebaseTodo {
	return {
		upstream: 'f'.repeat(40),
		upstreamShort: 'fffffff',
		rows: Array.from({ length: count }, (_, n) => todoRow(n + 1)),
		truncated: false
	};
}

function previewRow(n: number, overrides: Partial<PreviewRow> = {}): PreviewRow {
	return {
		id: `${n}`.repeat(40),
		short: `${n}`.repeat(7),
		summary: `Commit ${n}`,
		absorbed: [],
		reworded: false,
		mayConflict: false,
		...overrides
	};
}

function previewOf(overrides: Partial<RebasePreview> = {}): RebasePreview {
	return {
		rows: [],
		dropped: [],
		refusal: null,
		emptiesTheBranch: false,
		...overrides
	};
}

async function planned() {
	rebase.upstream = 'main';
	await rebase.load();
}

beforeEach(() => {
	vi.clearAllMocks();
	rebase.clear();
	rebaseTodo.mockResolvedValue(todoOf());
	rebasePreview.mockResolvedValue(previewOf());
});

describe('RebasePlan', () => {
	it('draws one row per commit, in plan order, with its summary and short id', async () => {
		await planned();
		const view = render(RebasePlan, {});

		const rows = view.all('.row');
		expect(rows).toHaveLength(3);
		expect(rows[0].textContent).toContain('Commit 1');
		expect(rows[0].textContent).toContain('1111111');
		view.destroy();
	});

	it('offers every action, with the current one marked', async () => {
		await planned();
		const view = render(RebasePlan, {});

		const chips = view.all('.row')[0].querySelectorAll('.action');
		expect([...chips].map((chip) => chip.textContent?.trim())).toEqual([
			'Pick',
			'Squash',
			'Reword',
			'Drop'
		]);
		expect(
			view.all('.row')[0].querySelector('.action[aria-pressed="true"]')?.textContent?.trim()
		).toBe('Pick');
		view.destroy();
	});

	it('each action chip says what it would do', async () => {
		await planned();
		const view = render(RebasePlan, {});

		const chips = [...view.all('.row')[0].querySelectorAll('.action')] as HTMLElement[];
		expect(chips[1].title).toContain('above');
		expect(chips[3].title).toContain('out of the result');
		view.destroy();
	});

	it('clicking an action sets it', async () => {
		await planned();
		const view = render(RebasePlan, {});

		click([...view.all('.row')[0].querySelectorAll('.action')][3] as HTMLElement);
		await Promise.resolve();

		expect(rebase.plan[0].action).toBe('drop');
		view.destroy();
	});

	it('a dropped commit stays visible and reads as spent', async () => {
		await planned();
		await rebase.setAction(todoRow(2).id, 'drop');
		const view = render(RebasePlan, {});

		expect(view.all('.row')).toHaveLength(3);
		expect(view.all('.row.dropped')).toHaveLength(1);
		view.destroy();
	});

	it('moves a row with the keyboard, so reordering does not need a pointer', async () => {
		// Drag alone is untestable headlessly and unusable for some people.
		await planned();
		const view = render(RebasePlan, {});

		press(view.all('.row')[0].querySelector('.what') as HTMLElement, 'ArrowDown', {
			altKey: true
		});
		await Promise.resolve();

		expect(rebase.plan.map((entry) => entry.id)).toEqual([
			todoRow(2).id,
			todoRow(1).id,
			todoRow(3).id
		]);
		view.destroy();
	});

	it('a plain arrow key does not move anything', async () => {
		await planned();
		const view = render(RebasePlan, {});

		press(view.all('.row')[0].querySelector('.what') as HTMLElement, 'ArrowDown');
		await Promise.resolve();

		expect(rebase.plan.map((entry) => entry.id)).toEqual(
			todoOf().rows.map((row) => row.id)
		);
		view.destroy();
	});

	it('marks the focused row', async () => {
		await planned();
		const view = render(RebasePlan, {});

		expect(view.all('.row.focused')).toHaveLength(1);
		expect(view.all('.row.focused')[0].textContent).toContain('Commit 1');
		view.destroy();
	});

	it('a drag moves the row it was dropped onto', async () => {
		await planned();
		const view = render(RebasePlan, {});
		const rows = view.all('.row');

		fire(rows[0], 'dragstart');
		fire(rows[2], 'dragover');
		fire(rows[2], 'drop');
		await Promise.resolve();

		expect(rebase.plan.map((entry) => entry.id)).toEqual([
			todoRow(2).id,
			todoRow(3).id,
			todoRow(1).id
		]);
		view.destroy();
	});

	it('dropping a row on itself changes nothing', async () => {
		await planned();
		const view = render(RebasePlan, {});
		const rows = view.all('.row');

		fire(rows[1], 'dragstart');
		fire(rows[1], 'drop');
		await Promise.resolve();

		expect(rebase.plan.map((entry) => entry.id)).toEqual(
			todoOf().rows.map((row) => row.id)
		);
		view.destroy();
	});

	it('a drop with nothing being dragged is ignored', async () => {
		await planned();
		const view = render(RebasePlan, {});

		fire(view.all('.row')[1], 'drop');
		await Promise.resolve();

		expect(rebase.plan.map((entry) => entry.id)).toEqual(
			todoOf().rows.map((row) => row.id)
		);
		view.destroy();
	});

	it('ending a drag without dropping leaves the plan alone', async () => {
		await planned();
		const view = render(RebasePlan, {});
		const rows = view.all('.row');

		fire(rows[0], 'dragstart');
		fire(rows[0], 'dragend');
		fire(rows[2], 'drop');
		await Promise.resolve();

		expect(rebase.plan.map((entry) => entry.id)).toEqual(
			todoOf().rows.map((row) => row.id)
		);
		view.destroy();
	});

	it('the handle says both ways to reorder', async () => {
		await planned();
		const view = render(RebasePlan, {});

		expect(view.get('.grip').title).toContain('Drag');
		expect(view.get('.grip').title).toContain('Alt+');
		view.destroy();
	});
	it("takes a reword's new message on the row, and keeps it across a change of mind", async () => {
		await planned();
		const id = todoRow(1).id;
		await rebase.setAction(id, 'reword');
		const view = render(RebasePlan, {});

		const input = view.get('.reword') as HTMLInputElement;
		expect(input.placeholder).toBe('Commit 1');
		input.value = 'A better message';
		fire(input, 'input');
		expect(rebase.plan[0].message).toBe('A better message');

		await rebase.setAction(id, 'pick');
		await rebase.setAction(id, 'reword');
		expect(rebase.messageOf(id)).toBe('A better message');
		view.destroy();
	});

	it('says a squash folds into the one above, and when there is none', async () => {
		await planned();
		await rebase.setAction(todoRow(1).id, 'squash');
		await rebase.setAction(todoRow(2).id, 'squash');
		const view = render(RebasePlan, {});

		const rows = view.all('.row');
		expect(rows[0].textContent).toContain('nothing above to fold into');
		expect(rows[1].textContent).toContain('folds into the one above');
		view.destroy();
	});

	it('marks the rows the preview thinks may conflict', async () => {
		rebasePreview.mockResolvedValue(previewOf({ rows: [previewRow(2, { mayConflict: true })] }));
		await planned();
		const view = render(RebasePlan, {});

		expect(view.all('.row')[1].textContent).toContain('may conflict');
		expect(view.all('.row')[0].textContent).not.toContain('may conflict');
		view.destroy();
	});

	it('locks every control while git owns the branch', async () => {
		await planned();
		const view = render(RebasePlan, { locked: true });

		click(view.all('.row')[0].querySelectorAll('.action')[3] as HTMLElement);
		await Promise.resolve();

		expect(rebase.plan[0].action).toBe('pick');
		expect(view.all('.row')[0].getAttribute('draggable')).toBe('false');
		view.destroy();
	});
});

describe('RebaseHistory', () => {
	const props = (overrides = {}) => ({
		rows: [previewRow(1), previewRow(2)],
		dropped: [],
		onto: 'main',
		ontoShort: 'fffffff',
		branch: 'topic',
		...overrides
	});

	it('draws the branch it lands on, then one node per new commit, and names the branch', () => {
		const view = render(RebaseHistory, props());

		expect(view.text()).toContain('onto main · fffffff');
		expect(view.text()).toContain('1111111');
		expect(view.text()).toContain('2222222');
		expect(view.text()).toContain('topic');
		view.destroy();
	});

	it('shows how many commits a folded one holds, and rings the risky ones', () => {
		const view = render(
			RebaseHistory,
			props({ rows: [previewRow(1, { absorbed: ['a', 'b'] }), previewRow(2, { mayConflict: true })] })
		);

		expect(view.get('.count').textContent).toBe('3');
		expect(view.all('.risk')).toHaveLength(1);
		view.destroy();
	});

	it('sums up the oldest past eight rather than drawing them all', () => {
		const rows = Array.from({ length: 11 }, (_, n) => previewRow(n + 1));
		const view = render(RebaseHistory, props({ rows }));

		expect(view.text()).toContain('+ 3 earlier');
		view.destroy();
	});

	it('lists what the plan drops', () => {
		const view = render(
			RebaseHistory,
			props({ dropped: [{ short: 'abcdef1', summary: 'gone' }, { short: 'abcdef2', summary: 'also' }] })
		);

		expect(view.text()).toContain('2 dropped');
		expect(view.text()).toContain('abcdef1');
		view.destroy();
	});
});
