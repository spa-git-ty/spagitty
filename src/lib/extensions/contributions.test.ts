// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { extension } from '../../testing/extension-fixtures';
import {
	actionsFor,
	contributes,
	NO_FACTS,
	paletteCommands,
	panelsFor,
	placement,
	qualified,
	unavailable,
	type Facts
} from './contributions';
import { elapsed, headline, location, ordered, STATUS_LABELS, tally } from './describe';
import type { ReviewFinding, ReviewRecord } from './types';

const open: Facts = { ...NO_FACTS, repositoryOpen: true, hasWorkingChanges: true };

describe('who contributes', () => {
	it('an idle, enabled, compatible extension contributes; the rest leave nothing behind', () => {
		expect(contributes(extension())).toBe(true);
		expect(contributes(extension({ state: 'active' }))).toBe(true);
		for (const state of ['failed', 'disabled', 'incompatible'] as const) {
			expect(contributes(extension({ state }))).toBe(false);
			expect(paletteCommands([extension({ state })])).toEqual([]);
			expect(actionsFor([extension({ state })], 'workingCopy')).toEqual([]);
			expect(panelsFor([extension({ state })], 'workingCopy')).toEqual([]);
		}
		expect(contributes(extension({ enabled: false }))).toBe(false);
		expect(contributes(extension({ compatibility: { compatible: false, reasons: ['x'] } }))).toBe(false);
	});

	it('qualifies every id with its extension', () => {
		expect(qualified('com.example.hello', 'hello')).toBe('com.example.hello/hello');
		const keys = paletteCommands([extension()]).map((p) => p.key);
		expect(new Set(keys).size).toBe(keys.length);
		expect(keys.every((k) => k.startsWith('com.example.hello/'))).toBe(true);
	});
});

describe('where they show', () => {
	it('puts actions on the screen whose context they name, and honours menu and palette', () => {
		const titles = actionsFor([extension()], 'workingCopy').map((p) => p.command.title);
		expect(titles).toEqual(['Menu only', 'Review changes', 'Say hello']);
		expect(actionsFor([extension()], 'farmTask').map((p) => p.command.id)).toEqual(['task']);
		const palette = paletteCommands([extension()]).map((p) => p.command.id);
		expect(palette).toContain('hidden');
		expect(palette).not.toContain('quiet');
	});

	it('places panels by location, or by renderer when none is given', () => {
		const here = panelsFor([extension()], 'workingCopy');
		expect(here.map((p) => p.panel.id)).toEqual(['findings', 'about']);
		expect(here[0].provider).toBe('review');
		expect(panelsFor([extension()], 'pullRequest').map((p) => p.panel.id)).toEqual(['pr']);
		expect(placement({ id: 's', title: 'S', renderer: 'summary' })).toBe('global');
	});
});

describe('why a command cannot run', () => {
	const ext = extension();
	const hello = ext.manifest.contributes!.commands![0];
	const quiet = ext.manifest.contributes!.commands![4];
	const task = ext.manifest.contributes!.commands![2];

	it('runs when its context and conditions are met', () => {
		expect(unavailable(ext, hello, open)).toBeNull();
	});

	it('names the unmet condition in a few words', () => {
		expect(unavailable(ext, hello, NO_FACTS)).toBe('Open a repository');
		expect(unavailable(ext, quiet, { ...open, hasWorkingChanges: false })).toBe('No changes');
		expect(unavailable(ext, task, open)).toBe('Select a task');
	});

	it('passes on what the extension itself reported', () => {
		const reporting = extension({ unavailable: [{ id: 'hello', reason: 'The tool is missing' }] });
		expect(unavailable(reporting, hello, open)).toBe('The tool is missing');
	});

	it('says why an extension that does not contribute cannot run it', () => {
		expect(unavailable(extension({ state: 'failed', stateReason: 'It crashed.' }), hello, open)).toBe('It crashed.');
		expect(unavailable(extension({ state: 'stopping' }), hello, open)).toBe('Stopping');
	});
});

describe('the words for a review', () => {
	const finding = (over: Partial<ReviewFinding>): ReviewFinding => ({
		id: 'f',
		reviewId: 'r',
		providerId: 'p',
		severity: 'low',
		title: 't',
		message: 'm',
		disposition: 'open',
		...over
	});

	it('never invents a line', () => {
		expect(location(finding({}))).toBe('Whole review');
		expect(location(finding({ path: 'a.rs' }))).toBe('a.rs');
		expect(location(finding({ path: 'a.rs', startLine: 3 }))).toBe('a.rs:3');
		expect(location(finding({ path: 'a.rs', startLine: 3, endLine: 9 }))).toBe('a.rs:3–9');
	});

	it('orders by severity, stably, and tallies what is there', () => {
		const list = [finding({ id: 'a', severity: 'low' }), finding({ id: 'b', severity: 'critical' }), finding({ id: 'c', severity: 'low' })];
		expect(ordered(list).map((f) => f.id)).toEqual(['b', 'a', 'c']);
		expect(tally(list)).toEqual([
			{ severity: 'critical', count: 1 },
			{ severity: 'low', count: 2 }
		]);
	});

	it('tells completed from approved, and a partial failure from a clean one', () => {
		const record = (status: ReviewRecord['result']['status'], findings: ReviewFinding[]) =>
			({ result: { status, findings } }) as unknown as ReviewRecord;
		expect(headline(record('completed', []))).toBe('Reviewed — no findings');
		expect(headline(record('completed', [finding({})]))).toBe('Reviewed — 1 finding');
		expect(headline(record('failed', [finding({}), finding({})]))).toBe('Failed · 2 findings before it stopped');
		expect(headline(record('skipped', []))).toBe(STATUS_LABELS.skipped);
		expect(Object.values(STATUS_LABELS).some((l) => /approved|passed/i.test(l))).toBe(false);
	});

	it('shows elapsed time, never a percentage', () => {
		expect(elapsed(0)).toBe('0s');
		expect(elapsed(72_000)).toBe('1m 12s');
		expect(elapsed(-5)).toBe('0s');
	});
});
