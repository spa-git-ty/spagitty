// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Starting a farm one step at a time (FEAT-141): what each step allows, what
 * the start button says, what the planner is given and what *Your farm* shows.
 */

import { describe, expect, it } from 'vitest';
import type { Issue } from '$lib/issues/types';
import type { PastFarm, Setup } from './types';
import {
	back,
	canContinue,
	initial,
	next,
	pastLine,
	plannerGets,
	rememberedLine,
	rulesLine,
	rulesOf,
	skipToStart,
	skippable,
	soFar,
	sourcesLine,
	sourcesOf,
	startLabel,
	startingPoint,
	wayDetail,
	wayOf,
	ways,
	type WizardState
} from './wizard';

const issue = (number: number, title = `issue ${number}`): Issue =>
	({ id: String(number), number, title, body: 'body', state: 'open', author: 'a', created: '', updated: '', url: `https://host/${number}`, replies: 0, labels: [{ name: 'feature', color: '' }], assignees: [], milestone: null, locked: false, branchSuggestion: '' }) as Issue;

const setup: Setup = {
	agents: ['claude'],
	autonomy: 'semiAuto',
	permissions: { writeFiles: true, runCommands: true, network: false, commit: true, push: false, merge: false, deleteBranch: false },
	maxParallel: 3,
	maxAttempts: 3,
	verification: ['cargo test --workspace', 'bun run check'],
	supplemental: { mode: 'off', provider: 'p', threshold: 'medium', maxRepairs: 2, revision: 1 },
	workspace: 'worktree',
	fromTitle: 'GitLab support for pull requests',
	fromMs: new Date(2026, 9, 7).getTime()
};

describe('the steps', () => {
	it('asks only for the goal', () => {
		const empty = initial();
		expect(canContinue(empty)).toBe(false);
		expect(next(empty).step).toBe(1);
		expect(skipToStart(empty).step).toBe(1);
		const goal = { ...empty, title: '  Gitea support  ' };
		expect(next(goal).step).toBe(2);
		expect(skipToStart(goal).step).toBe(4);
	});

	it('goes forward and back one step, and no further than the ends', () => {
		let state: WizardState = { ...initial('Gitea'), step: 1 };
		state = next(next(next(state)));
		expect(state.step).toBe(4);
		expect(next(state).step).toBe(4);
		state = back(back(back(state)));
		expect(state.step).toBe(1);
		expect(back(state).step).toBe(1);
	});

	it('counts step 3 as seen once it is reached, and only Start from has a Skip', () => {
		const state = next(next(initial('Gitea')));
		expect(state.step).toBe(3);
		expect(state.crewSeen).toBe(true);
		expect([1, 2, 3, 4].filter((s) => skippable(s as 1 | 2 | 3 | 4))).toEqual([2]);
	});
});

describe('step 4', () => {
	it('offers one task per issue only when issues were picked', () => {
		expect(ways(initial('g'))).toEqual(['plan', 'empty']);
		const picked = { ...initial('g'), issues: [issue(142), issue(147), issue(151)] };
		expect(ways(picked)).toEqual(['plan', 'perIssue', 'empty']);
		// A chosen way that stops being offered falls back to planning.
		expect(wayOf({ ...initial('g'), way: 'perIssue' })).toBe('plan');
	});

	it('says what the button will do', () => {
		expect(startLabel('plan', 'Claude Code', 0)).toBe('Plan it with Claude Code');
		expect(startLabel('perIssue', null, 3)).toBe('Make 3 tasks');
		expect(startLabel('perIssue', null, 1)).toBe('Make 1 task');
		expect(startLabel('empty', null, 0)).toBe('Open an empty plan');
		expect(wayDetail('perIssue', [issue(142), issue(147), issue(151)])).toBe(
			'3 tasks, straight from #142, #147 and #151. Edit them before they run.'
		);
	});

	it('lists exactly what goes into the planning prompt, in its order', () => {
		const state: WizardState = {
			...initial('Gitea', 'Same Host trait'),
			issues: [issue(142), issue(147)],
			briefs: [{ path: 'docs/specs/gitea.md', text: '# Gitea', bytes: 4000 }],
			branch: 'feature/gitea'
		};
		expect(plannerGets(state, true).map((c) => c.label)).toEqual([
			'Goal and notes',
			'#142',
			'#147',
			'gitea.md',
			'feature/gitea',
			'AGENTS.md',
			'a map of the repository'
		]);
		expect(plannerGets(initial('Gitea'), false).map((c) => c.label)).toEqual(['The goal', 'a map of the repository']);
	});

	it('creates the goal with every source the person picked', () => {
		const state: WizardState = {
			...initial('Gitea'),
			issues: [issue(142, 'Gitea: list and open pull requests')],
			briefs: [{ path: 'docs/specs/gitea.md', text: '# Gitea', bytes: 4 }],
			branch: 'feature/gitea'
		};
		expect(sourcesOf(state)).toEqual([
			{ kind: 'issue', number: 142, title: 'Gitea: list and open pull requests', body: 'body', labels: ['feature'], url: 'https://host/142' },
			{ kind: 'brief', path: 'docs/specs/gitea.md', text: '# Gitea' },
			{ kind: 'branch', name: 'feature/gitea' }
		]);
		expect(sourcesLine(state)).toBe('1 issue · 1 brief · feature/gitea');
		expect(sourcesLine(initial('g'))).toBeNull();
	});
});

describe('Your farm', () => {
	it('dims and tags the remembered crew and rules until step 3', () => {
		const rules = rulesOf(setup);
		const at1 = soFar(initial('Gitea'), { crew: 4, rules, remembered: true, planner: 'Claude Code' });
		expect(at1.map((r) => [r.label, r.value, r.dim, r.lastTime])).toEqual([
			['Goal', 'Gitea', false, false],
			['Start from', 'Nothing yet · optional', true, false],
			['Crew', '4 agents', true, true],
			['Rules', 'Semi-automatic · 3 at once · 2 checks', true, true],
			['Tasks', '—', true, false]
		]);
		const at4 = soFar(next(next(next(initial('Gitea')))), { crew: 4, rules, remembered: true, planner: 'Claude Code' });
		expect(at4.find((r) => r.label === 'Crew')?.lastTime).toBe(false);
		expect(at4.find((r) => r.label === 'Tasks')?.value).toBe('Planned by Claude Code');
	});

	it('words the rules in one line', () => {
		expect(rulesLine({ autonomy: 'auto', maxParallel: 1, verification: [] })).toBe('Automatic · 1 at once · no checks');
	});
});

describe('what was remembered and what came before', () => {
	it('names the farm the crew and rules came from', () => {
		expect(rememberedLine(setup)).toBe('From “GitLab support for pull requests”, Oct 7. Change anything, or go on.');
		expect(rememberedLine(null)).toBe('These are the defaults. Change anything, or go on.');
	});

	it('offers to resume a stopped farm and to start from a finished one', () => {
		const past = (status: PastFarm['status']): PastFarm => ({
			id: 'f',
			goal: { id: 'g', title: 'Hooks tab, step by step', description: 'notes', constraints: [], createdMs: 0 },
			status,
			merged: 4,
			total: 7,
			updatedMs: 0,
			setup
		});
		expect(pastLine(past('paused'))).toEqual({ text: 'Stopped · 4 of 7 merged', colour: 'var(--warn)', action: 'Resume →' });
		expect(pastLine(past('completed')).action).toBe('Use as a starting point →');
		expect(startingPoint(past('completed'))).toMatchObject({ title: 'Hooks tab, step by step', notes: 'notes', rules: { maxParallel: 3 } });
	});

	it('starts from the defaults when nothing is remembered', () => {
		expect(rulesOf(null)).toMatchObject({ autonomy: 'semiAuto', maxParallel: 2, maxAttempts: 3, verification: [], workspace: 'worktree' });
		// FEAT-146: a person resolves conflicts until the Rules say otherwise.
		expect(rulesOf(null).conflicts).toEqual({ by: 'ask' });
	});

	it('keeps who resolves conflicts and where tasks work, and reads an older setup as asking (FEAT-146, FEAT-147)', () => {
		expect(rulesOf(setup).conflicts).toEqual({ by: 'ask' });
		const remembered = rulesOf({ ...setup, workspace: 'copy', conflicts: { by: 'tool', command: 'mergiraf solve' } });
		expect(remembered.workspace).toBe('copy');
		expect(remembered.conflicts).toEqual({ by: 'tool', command: 'mergiraf solve' });
	});
});
