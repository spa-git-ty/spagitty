// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import {
	LEVELS,
	above,
	agentRead,
	atGate,
	capped,
	choiceWords,
	currentStep,
	clock,
	consentSlug,
	findingLabel,
	findingsByPath,
	cardLine,
	grouped,
	isLive,
	leaves,
	levelWords,
	meter,
	nowLine,
	openFindings,
	perRun,
	planned,
	progressOf,
	quietFor,
	quietWords,
	readingNow,
	sentenceFor,
	share,
	stepTime,
	unitsDone
} from './levels';
import { aFinding, aMerge, aReview, aStep } from '../../testing/agent-fixtures';
import type { Assignment } from './types';

describe('the four levels', () => {
	it('are named for what the person does, in order', () => {
		expect(LEVELS.map((l) => l.label)).toEqual([
			'Read-only',
			'Ask every step',
			'Ask before sending or merging',
			'Full access'
		]);
		expect(LEVELS.map((l) => l.short)).toEqual(['Read-only', 'Ask every step', 'Ask at the end', 'Full access']);
		expect(levelWords('stepByStep').short).toBe('Ask every step');
	});

	it('keep the values saved settings already hold', () => {
		expect(LEVELS.map((l) => l.id)).toEqual(['suggest', 'stepByStep', 'signOff', 'unattended']);
	});

	it('describe what the agent may do, in each job', () => {
		expect(sentenceFor('suggest', 'review')).toBe('Reads and proposes. Nothing is applied until you accept it.');
		expect(sentenceFor('signOff', 'review')).toBe('Works through everything and stops before the review is sent.');
		expect(sentenceFor('signOff', 'merge')).toBe('Resolves everything and stops before the merge is made.');
		expect(sentenceFor('unattended', 'merge')).toBe(
			'Makes the merge itself, only when every conflict is sure and the checks pass.'
		);
	});

	it('say where the agent stops, per job', () => {
		expect(sentenceFor('stepByStep', 'review')).toBe('Stops for you after the plan and after each file.');
		expect(sentenceFor('stepByStep', 'merge')).toBe('Stops for you at each conflict and at the checks.');
		expect(sentenceFor('unattended', 'review')).toContain('as a comment unless');
	});

	it('are capped by the repository', () => {
		expect(above('unattended', 'signOff')).toBe(true);
		expect(above('suggest', 'signOff')).toBe(false);
		expect(capped('unattended', 'signOff')).toBe('signOff');
		expect(capped('stepByStep', 'signOff')).toBe('stepByStep');
	});
});

describe('the meter', () => {
	it('counts units done of planned, and the time', () => {
		const review = aReview({ steps: [...aReview().steps.slice(0, 2), aStep({ index: 2, kind: { kind: 'file', path: 'a' } })] });
		expect(planned(review)).toBe(4);
		expect(unitsDone(review)).toBe(1);
		expect(meter(review, 180)).toBe('1 / 4 · 1:20');
		expect(share(review)).toBe(0.25);
	});

	it('shows only the time before anything is planned', () => {
		expect(meter(aReview({ planned: 0 }), 112)).toBe('0:12');
		expect(share(aReview({ planned: 0 }))).toBe(0);
	});

	it('reads a long run in hours', () => {
		expect(clock(3725)).toBe('1:02:05');
		expect(stepTime(aStep({ startedAt: 0, endedAt: 31 }), 999)).toBe('0:31');
	});

	it('counts a merge in conflicts, with landing when it lands', () => {
		expect(planned(aMerge({ planned: 7, lands: true }))).toBe(4);
		expect(planned(aMerge({ planned: 6, lands: false }))).toBe(4);
	});
});

describe('where the agent is', () => {
	it('is on the file of a step that runs or waits', () => {
		expect(readingNow(aReview())).toBe('crates/spagitty-core/src/avatars.rs');
		expect(readingNow(aReview({ state: 'done' }))).toBeNull();
	});

	it('has read the files whose steps are done, apart from viewed', () => {
		const review = aReview({
			steps: [aStep({ kind: { kind: 'file', path: 'a.rs' } }), aStep({ kind: { kind: 'file', path: 'b.rs' }, state: 'running' })]
		});
		expect([...agentRead(review)]).toEqual(['a.rs']);
	});

	it('counts open findings per file', () => {
		const review = aReview({ proposals: [aFinding(), aFinding({ id: 'x', state: 'accepted' })] });
		expect(openFindings(review)).toHaveLength(1);
		expect(findingsByPath(review).get('crates/spagitty-core/src/avatars.rs')).toBe(1);
	});

	it('finds the step waiting at a gate, with what waits there', () => {
		const gate = atGate(aReview());
		expect(gate?.step.gate).toBe('file');
		expect(gate?.proposals).toHaveLength(2);
		expect(atGate(aReview({ state: 'working' }))).toBeNull();
	});
});

describe('words', () => {
	it('badge a finding with its author and severity', () => {
		const finding = aFinding().body;
		if (finding.kind !== 'comment') throw new Error('a comment');
		expect(findingLabel('Codex', finding)).toBe('Codex · proposed · high');
	});

	it('name a resolution in the resolver’s words', () => {
		const names = { a: 'main', b: 'feat/tab-drag' };
		expect(choiceWords({ mode: 'ab' }, names)).toBe('Both, main first');
		expect(choiceWords({ mode: 'b' }, names)).toBe('Take feat/tab-drag');
		expect(choiceWords({ mode: 'edit' }, names)).toBe('Edit');
	});

	it('say what leaves the machine, and when nothing does', () => {
		expect(leaves({ local: false, providerLabel: 'Anthropic' })).toBe(
			'Code you assign it is sent to Anthropic. Each repository asks once.'
		);
		expect(leaves({ local: true, providerLabel: 'Ollama' })).toBe('Everything stays on this machine.');
	});

	it('keep consent under the backend’s slugs', () => {
		expect(consentSlug({ provider: 'openAi' })).toBe('openai');
		expect(consentSlug({ provider: 'compatible' })).toBe('compatible');
		expect(consentSlug({ provider: 'anthropic' })).toBe('anthropic');
		expect(consentSlug({ provider: 'google' })).toBe('google');
	});

	it('count tokens the way people do', () => {
		expect(grouped(400000)).toBe('400,000');
		expect(perRun(40000)).toBe('40k / run');
		expect(perRun(500)).toBe('500 / run');
		expect(perRun(null)).toBe('no limit');
	});

	it('tell a live assignment from a finished one', () => {
		expect(isLive(aReview())).toBe(true);
		expect(isLive(aReview({ state: 'failed' }))).toBe(false);
	});
});

describe('the card line', () => {
	it('says how far the agent got, or that it waits', () => {
		expect(cardLine(aReview())).toBe('Claude Code · waiting for you');
		expect(cardLine(aReview({ state: 'working', steps: [aStep({ kind: { kind: 'file', path: 'a' } })] }))).toBe(
			'Claude Code · 1 of 4 files'
		);
		expect(cardLine(aMerge({ state: 'working', planned: 0, steps: [] }))).toBe('Codex · starting');
		expect(cardLine(aReview({ state: 'done' }))).toBe('Claude Code · finished');
		expect(cardLine(aReview({ state: 'stopped' }))).toBe('Claude Code · stopped');
		expect(cardLine(aReview({ state: 'failed' }))).toBe('Claude Code · failed');
		expect(cardLine(aReview({ state: 'paused' }))).toBe('Claude Code · paused');
	});
});

describe('how far through a run is (FEAT-119)', () => {
	const NOW = 10_000;

	it('counts the steps before the plan has said how many files', () => {
		const a = aReview({
			state: 'working',
			planned: 0,
			steps: [aStep({ index: 0, state: 'done' }), aStep({ index: 1, state: 'running', endedAt: null })]
		});
		expect(progressOf(a)).toEqual({ done: 1, total: null, unit: 'steps' });
		expect(share(a)).toBe(0);
		// Nothing says how many there will be, so only the time is claimed.
		expect(meter(a, NOW)).toMatch(/^\d+:\d\d$|^\d+:\d\d:\d\d$/);
	});

	it('counts the steps against the plan once it names them, when there are no units', () => {
		const a = aReview({ state: 'working', planned: 3, steps: [aStep({ state: 'done' })] });
		expect(progressOf(a)).toEqual({ done: 1, total: 3, unit: 'steps' });
		expect(meter(a, a.startedAt + 62)).toBe('1 / 3 steps · 1:02');
		expect(share(a)).toBeCloseTo(1 / 3);
	});

	it('counts the files or conflicts once they are known', () => {
		const a = aReview({
			state: 'working',
			planned: 7,
			steps: [aStep({ kind: { kind: 'file', path: 'a.rs' }, state: 'done' })]
		});
		expect(progressOf(a)).toEqual({ done: 1, total: 4, unit: 'files' });
		expect(progressOf(aMerge({ state: 'working', planned: 4, steps: [] }))).toMatchObject({ unit: 'conflicts' });
	});

	it('has no progress before there is a first step', () => {
		expect(progressOf(aReview({ planned: 0, steps: [] }))).toBeNull();
	});

	it('names the step it is on and how long it has been on it', () => {
		const a = aReview({
			state: 'working',
			steps: [
				aStep({ index: 0, label: 'Read the pull request', state: 'done', startedAt: 100, endedAt: 112 }),
				aStep({ index: 1, label: 'avatars.rs', state: 'running', startedAt: 120, endedAt: null })
			]
		});
		expect(currentStep(a)?.label).toBe('avatars.rs');
		expect(nowLine(a, 165)).toBe('avatars.rs · 0:45');
		expect(nowLine(aReview({ state: 'done' }), 165)).toBeNull();
	});

	it('is quiet only when it is working, has said nothing, and for long enough', () => {
		const quiet = (state: Assignment['state'], since: number | null) => quietFor(aReview({ state, quietSince: since }), 1_000);
		expect(quiet('working', 1_000 - 240)).toBe(240);
		expect(quiet('working', 1_000 - 60)).toBeNull();
		expect(quiet('working', null)).toBeNull();
		expect(quiet('waiting', 1_000 - 240)).toBeNull();
		expect(quiet('done', 1_000 - 240)).toBeNull();
		expect(quietWords(240)).toBe('4 min');
		expect(quietWords(3_900)).toBe('1 h 5 min');
	});

	it('shows a quiet run on the card line, and the step it is on before there are units', () => {
		expect(cardLine(aReview({ state: 'working', quietSince: 1_000 - 300 }), 1_000)).toBe('Claude Code · quiet for 5 min');
		expect(
			cardLine(aReview({ state: 'working', planned: 0, steps: [aStep({ label: 'Read the pull request', state: 'running', endedAt: null })] }), 1_000)
		).toBe('Claude Code · Read the pull request');
	});
});
