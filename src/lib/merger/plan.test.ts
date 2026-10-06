// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import { cleanForecast, file, forecast, side } from '../../testing/merger-fixtures';
import {
	cannotLand,
	cardRole,
	conflictSummary,
	connector,
	defaultNewName,
	effectiveStrategy,
	fileStatus,
	filesLine,
	meter,
	primaryAction,
	roles,
	sentence,
	stats,
	strategies
} from './plan';

const plan = forecast();
const intoMain = roles(plan, 'a', '');
const intoFeat = roles(plan, 'b', '');
const intoNew = roles(plan, 'new', 'merge/main-tab-drag');

describe('roles', () => {
	it('B comes into A, A into B, and a new branch starts from A', () => {
		expect(intoMain).toMatchObject({ source: 'b', onto: 'a', targetName: 'main', isNew: false });
		expect(intoFeat).toMatchObject({ source: 'a', onto: 'b', targetName: 'feat/tab-drag' });
		expect(intoNew).toMatchObject({ source: 'b', onto: 'a', targetName: 'merge/main-tab-drag', isNew: true });
	});

	it('names a new branch from the last part of each', () => {
		expect(defaultNewName('main', 'feat/tab-drag')).toBe('merge/main-tab-drag');
		expect(defaultNewName('origin/release/2.0', 'fix')).toBe('merge/2.0-fix');
	});
});

describe('where it can land', () => {
	it('only on a branch here', () => {
		const remote = forecast({ b: side({ name: 'origin/feat', kind: 'remote' }) });
		expect(cannotLand(remote, 'b', '', [])).toContain('remote branch');
		const tag = forecast({ b: side({ name: 'v1.0', kind: 'tag' }) });
		expect(cannotLand(tag, 'b', '', [])).toContain('not a branch');
		expect(cannotLand(tag, 'a', '', [])).toBeNull();
	});

	it('a new branch needs a name nobody has', () => {
		expect(cannotLand(plan, 'new', '  ', [])).toBe('Name the new branch');
		expect(cannotLand(plan, 'new', 'main', ['main'])).toBe('main already exists');
		expect(cannotLand(plan, 'new', 'merge/x', ['main'])).toBeNull();
	});
});

describe('strategies', () => {
	it('fast-forward is refused, with the reason, when both have their own commits', () => {
		const ff = strategies(plan, intoMain).find((choice) => choice.id === 'ff')!;
		expect(ff.reason).toBe('Both branches have commits the other lacks');
		expect(ff.description).toBe('Not possible here: both branches have commits the other lacks.');
		expect(effectiveStrategy(strategies(plan, intoMain), 'ff')).toBe('merge');
	});

	it('fast-forward is offered when the receiving side is behind', () => {
		const behind = forecast({ a: side({ ahead: 0 }), bHasA: true });
		const ff = strategies(behind, roles(behind, 'a', '')).find((choice) => choice.id === 'ff')!;
		expect(ff.reason).toBeNull();
		// The other way round, nothing comes in at all.
		const back = strategies(behind, roles(behind, 'b', ''));
		expect(back.every((choice) => choice.reason?.startsWith('Nothing comes in'))).toBe(true);
	});

	it('names the branches in the descriptions', () => {
		const squash = strategies(plan, intoFeat).find((choice) => choice.id === 'squash')!;
		expect(squash.description).toBe('Everything main changed, as one new commit on feat/tab-drag.');
	});
});

describe('the forecast', () => {
	it('says one plain sentence per strategy', () => {
		expect(sentence(plan, intoMain, 'merge')).toBe(
			'main gets 5 commits from feat/tab-drag, tied together by one merge commit. feat/tab-drag stays as it is.'
		);
		expect(sentence(plan, intoMain, 'squash')).toBe(
			'main gets one new commit holding everything feat/tab-drag changed. Its 5 commits are not carried over.'
		);
		expect(sentence(plan, intoNew, 'rebase')).toBe(
			'A new branch starts from main. feat/tab-drag’s 5 commits are replayed on top of merge/main-tab-drag as new commits, then merge/main-tab-drag moves forward to them. No merge commit.'
		);
	});

	it('counts what comes in, what is written, and what changes', () => {
		expect(stats(plan, intoMain, 'merge')).toEqual([
			{ value: 5, label: 'commits come in' },
			{ value: 1, label: 'merge commit' },
			{ value: 6, label: 'files change' }
		]);
		expect(stats(plan, intoFeat, 'merge')[2]).toEqual({ value: 5, label: 'files change' });
		expect(stats(plan, intoMain, 'rebase')[1]).toEqual({ value: 0, label: 'merge commits' });
	});

	it('says how many conflicts, where they were found, and for a rebase how often it may stop', () => {
		expect(conflictSummary(plan, intoMain, 'merge')).toMatchObject({
			title: '4 conflicts in 3 files',
			clean: '4 files merge on their own',
			note: 'Found by a dry run against 29c36a1, where the two branches split.'
		});
		expect(conflictSummary(plan, intoMain, 'rebase').note).toBe(
			'Rebase applies one commit at a time, so it may stop on up to 3 commits. The regions are the same 4.'
		);
		expect(conflictSummary(cleanForecast(), intoMain, 'merge').title).toBe('No conflicts');
	});

	it('draws one segment per file, red for each conflict', () => {
		expect(meter(plan)).toEqual([true, true, true, false, false, false, false]);
		const many = forecast({
			files: Array.from({ length: 100 }, (_, i) => file(`f${i}`, { touch: i === 0 ? 'conflict' : 'both' }))
		});
		const segments = meter(many);
		expect(segments).toHaveLength(24);
		expect(segments.filter(Boolean)).toHaveLength(1);
	});

	it('resolves first when there are conflicts, and merges straight away when there are none', () => {
		expect(primaryAction(plan, 'merge')).toEqual({ kind: 'resolve', label: 'Resolve 4 conflicts' });
		expect(primaryAction(cleanForecast(), 'squash')).toEqual({ kind: 'merge', label: 'Merge now' });
	});
});

describe('the stage', () => {
	it('outlines where it lands, and says the other stays as it is', () => {
		expect(cardRole('a', intoMain)).toEqual({ role: 'lands', label: 'Lands here' });
		expect(cardRole('b', intoMain)).toEqual({ role: 'comes-in', label: 'Comes in · stays as is' });
		expect(cardRole('a', intoNew).label).toBe('Starting point');
		expect(cardRole('b', intoNew).label).toBe('Comes in');
	});

	it('draws the incoming arrow solid, with its count', () => {
		expect(connector(plan, 'b', intoMain)).toEqual({ solid: true, label: '5 commits come in' });
		expect(connector(plan, 'a', intoMain)).toEqual({ solid: false, label: 'continues as' });
		expect(connector(plan, 'a', intoNew).label).toBe('starts from');
	});
});

describe('what changes', () => {
	it('groups each file by meaning, and follows the direction', () => {
		const [tabs, , , repo, test, nav, gitlab] = plan.files;
		expect(fileStatus(tabs, plan, intoMain)).toMatchObject({ label: '2 conflicts', tone: 'danger' });
		expect(fileStatus(repo, plan, intoMain).label).toBe('Both changed · merges cleanly');
		expect(fileStatus(test, plan, intoMain).label).toBe('New file from feat/tab-drag');
		expect(fileStatus(nav, plan, intoMain).label).toBe('Comes in from feat/tab-drag');
		expect(fileStatus(gitlab, plan, intoMain)).toMatchObject({ label: 'Already on main · untouched', tone: 'muted' });
		expect(fileStatus(gitlab, plan, intoFeat)).toMatchObject({ label: 'Comes in from main', tone: 'a' });
		expect(fileStatus(nav, plan, intoFeat).label).toBe('Already on feat/tab-drag · untouched');
	});

	it('says how many it touched and how many change', () => {
		expect(filesLine(plan, intoMain)).toBe('7 files touched by either branch · 6 change on main');
	});
});
