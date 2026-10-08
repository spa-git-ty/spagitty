// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import {
	LEVELS,
	above,
	agentRead,
	atGate,
	capped,
	choiceWords,
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
	openFindings,
	perRun,
	planned,
	readingNow,
	sentenceFor,
	share,
	stepTime,
	unitsDone
} from './levels';
import { aFinding, aMerge, aReview, aStep } from '../../testing/agent-fixtures';

describe('the four levels', () => {
	it('are named for what the person does, in order', () => {
		expect(LEVELS.map((l) => l.label)).toEqual(['Suggest', 'Step by step', 'Sign off', 'Unattended']);
		expect(levelWords('stepByStep').short).toBe('Step');
	});

	it('say where the agent stops, per job', () => {
		expect(sentenceFor('stepByStep', 'review')).toBe('Stops after the plan and each file.');
		expect(sentenceFor('stepByStep', 'merge')).toBe('Stops at each conflict and at the checks.');
		expect(sentenceFor('unattended', 'review')).toContain('as Comment unless');
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
