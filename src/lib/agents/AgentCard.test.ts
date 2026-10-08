// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Agent card (2.0), mounted: the sentence, the meter, the timeline, each
 * kind of gate, and every control reaching the engine.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, render } from '../../testing/mount';
import { aFinding, aMerge, aRemote, aReview, aSnapshot, aStep } from '../../testing/agent-fixtures';

vi.mock('./api', () => ({
	control: vi.fn(() => Promise.resolve()),
	transcript: vi.fn(() => Promise.resolve('the whole transcript')),
	forget: vi.fn(() => Promise.resolve())
}));

import * as api from './api';
import { agents } from './store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import AgentCard from './AgentCard.svelte';
import type { Assignment } from './types';

const text = (el: Element | null | undefined) => el?.textContent?.replace(/\s+/g, ' ').trim() ?? '';

function mount(assignment: Assignment, props: Record<string, unknown> = {}) {
	return render(AgentCard, { assignment, ...props });
}

function button(view: ReturnType<typeof mount>, name: string) {
	return view.all('button').find((b) => text(b) === name || b.getAttribute('title') === name)!;
}

beforeEach(() => {
	vi.clearAllMocks();
	agents.reset(aSnapshot());
});

describe('what the card says', () => {
	it('names the agent, its level, the one sentence and the meter', () => {
		const view = mount(aReview());
		expect(view.text()).toContain('Claude Code');
		expect(view.text()).toContain('Step');
		expect(view.get('.sentence').className).toContain('waiting');
		expect(view.text()).toContain('Waiting for you: 2 findings on avatars.rs');
		expect(view.text()).toMatch(/0 \/ 4 · \d+:\d\d/);
	});

	it('shows a remote agent’s tokens against its budget', () => {
		agents.reset(aSnapshot({ remote: [aRemote({ id: 'api-1', tokensPerRun: 40000 })] }));
		const view = mount(
			aReview({ agent: { id: 'api-1', name: 'Anthropic', reach: 'remote', version: null, provider: 'anthropic', model: 'm' }, tokens: { input: 12000, output: 400 } })
		);
		expect(view.text()).toContain('12,400 / 40,000 tokens');
	});

	it('says pausing while a pause waits for the step to end', () => {
		const view = mount(aReview({ state: 'working', pausing: true }));
		expect(view.text()).toContain('Pausing after this step');
	});

	it('shows why it stopped, and offers Resume and forget once it has', () => {
		const resume = vi.fn();
		const failed = aReview({ state: 'failed', reason: 'Codex exited with 1 after 4 of 12 files.' });
		agents.reset(aSnapshot(), [failed]);
		const view = mount(failed, { onresume: resume });
		expect(view.text()).toContain('Codex exited with 1 after 4 of 12 files.');
		click(button(view, 'Resume'));
		expect(resume).toHaveBeenCalled();
		click(button(view, 'forget'));
		expect(api.forget).toHaveBeenCalledWith('/work/spagitty', 'review-1');
	});

	it('says where the person took over', () => {
		const view = mount(aReview({ state: 'stopped', tookOver: 'You took over at file 2 of 4' }));
		expect(view.text()).toContain('You took over at file 2 of 4');
	});
});

describe('the controls', () => {
	it('pause, stop, take over and tell all reach the engine', async () => {
		const view = mount(aReview({ state: 'working' }));
		click(button(view, 'Pause after this step'));
		click(button(view, 'Stop now. Everything proposed stays.'));
		click(button(view, 'Take over'));
		const box = view.get('input[aria-label="Tell the agent"]') as HTMLInputElement;
		box.value = 'look at the migration first';
		box.dispatchEvent(new Event('input', { bubbles: true }));
		box.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
		await vi.waitFor(() => expect(api.control).toHaveBeenCalledTimes(4));
		expect(vi.mocked(api.control).mock.calls.map((c) => c[1])).toEqual([
			{ kind: 'pause' },
			{ kind: 'stop' },
			{ kind: 'takeOver' },
			{ kind: 'tell', text: 'look at the migration first' }
		]);
	});

	it('resumes a paused agent', () => {
		const view = mount(aReview({ state: 'paused' }));
		click(button(view, 'Resume'));
		expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'resume' });
	});

	it('offers the levels in a menu, the ones above the repository’s highest disabled with why', () => {
		const view = mount(aReview({ state: 'working' }));
		click(view.all('button').find((b) => text(b).startsWith('Step'))!);
		const menu = document.querySelector('[role="menu"]')!;
		const items = [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')];
		expect(items.map(text).join(' | ')).toContain('Unattended');
		const unattended = items.find((i) => text(i).includes('Unattended')) as HTMLButtonElement;
		expect(unattended.disabled).toBe(true);
		expect(text(unattended)).toContain('Capped at Sign off here');
		click(items.find((i) => text(i).includes('Sign off'))!);
		expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'level', level: 'signOff' });
	});

	it('follows the agent only when asked', () => {
		const onfollow = vi.fn();
		const view = mount(aReview({ state: 'working' }), { follow: false, onfollow });
		click(button(view, 'Follow the agent'));
		expect(onfollow).toHaveBeenCalledWith(true);
	});

	it('shows the raw output, and the command that started the run', async () => {
		const view = mount(aReview({ steps: [aStep({ command: 'claude -p --permission-mode plan …' })] }));
		click(view.get('.raw-toggle'));
		await vi.waitFor(() => expect(view.text()).toContain('the whole transcript'));
		expect(view.text()).toContain('claude -p --permission-mode plan …');
	});
});

describe('the gates', () => {
	it('a file’s findings: go to them, accept both, go on, or redo with a note', async () => {
		const ongo = vi.fn();
		const onaccept = vi.fn();
		const view = mount(aReview(), { ongo, onaccept });
		expect(view.get('.gate').textContent).toContain('2 findings');
		click(button(view, 'Go to them'));
		expect(ongo).toHaveBeenCalled();
		click(button(view, 'Accept both'));
		await vi.waitFor(() => expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'continue' }));
		expect(onaccept.mock.calls[0][0]).toHaveLength(2);

		vi.spyOn(dialog, 'prompt').mockResolvedValue('the offline case matters most');
		click(button(view, 'Redo with a note'));
		await vi.waitFor(() =>
			expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'redo', note: 'the offline case matters most' })
		);
	});

	it('the plan: drop a file from it, then go on', () => {
		const review = aReview({
			steps: [aStep(), aStep({ index: 1, kind: { kind: 'plan' }, label: 'Plan · 2 files', state: 'waiting', gate: 'plan', endedAt: null })],
			proposals: [
				{
					id: 'p1-0',
					step: 1,
					body: { kind: 'plan', files: [{ path: 'a.rs', why: '' }, { path: 'b.rs', why: '' }], lookFor: 'races' },
					sure: true,
					state: 'proposed',
					decidedBy: null,
					why: null,
					stale: false
				}
			]
		});
		const view = mount(review);
		expect(view.get('.gate').textContent).toContain('races');
		click(view.all('button').find((b) => b.getAttribute('title') === 'Leave a.rs out')!);
		expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'plan', files: ['b.rs'] });
		click(button(view, 'Go on'));
		expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'continue' });
	});

	it('the last act at Sign off is the person’s, filled in', () => {
		const onlast = vi.fn();
		const view = mount(
			aReview({ steps: [aStep({ kind: { kind: 'last' }, label: 'Finish review', state: 'waiting', gate: 'send', endedAt: null })], proposals: [] }),
			{ onlast }
		);
		click(button(view, 'Finish review'));
		expect(onlast).toHaveBeenCalled();
	});

	it('failing checks show their output, and can run again', () => {
		const view = mount(
			aMerge({
				state: 'waiting',
				steps: [
					aStep({
						kind: { kind: 'checks' },
						label: 'Checks · 1 failed',
						state: 'waiting',
						gate: 'checks',
						endedAt: null,
						checks: [{ command: 'bun test', passed: false, output: 'expected 2, got 3', durationMs: 10 }]
					})
				]
			})
		);
		expect(view.get('.gate').textContent).toContain('bun test failed');
		expect(view.get('.gate').textContent).toContain('expected 2, got 3');
		click(button(view, 'Run again'));
		expect(api.control).toHaveBeenCalledWith('merge-1', { kind: 'redo', note: '' });
	});

	it('a conflict says the agent’s one sentence on why', () => {
		const merge = aMerge({
			state: 'waiting',
			steps: [aStep({ index: 1, kind: { kind: 'conflict', path: 'a', region: 0 }, state: 'waiting', gate: 'conflict', endedAt: null })],
			proposals: [{ ...aMerge().proposals[0], step: 1, state: 'proposed' }]
		});
		const view = mount(merge, { names: { a: 'main', b: 'feat' }, ongo: vi.fn(), onaccept: vi.fn() });
		expect(view.get('.gate').textContent).toContain('Keeps pinned from main and dragging from feat/tab-drag.');
		expect(button(view, 'Go to it')).toBeDefined();
		expect(button(view, 'Accept')).toBeDefined();
	});

	it('a moved head offers Carry on or Stop', () => {
		const view = mount(aReview({ steps: [aStep()], reason: 'The pull request changed since the agent started.' }));
		expect(view.text()).toContain('The pull request changed since the agent started.');
		click(button(view, 'Carry on'));
		expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'carryOn' });
	});

	it('a gate with nothing to decide just goes on', () => {
		const view = mount(aReview({ proposals: [aFinding({ state: 'accepted' })] }));
		expect(view.get('.gate').textContent).toContain('Nothing to decide here.');
		click(button(view, 'Go on'));
		expect(api.control).toHaveBeenCalledWith('review-1', { kind: 'continue' });
	});
});
