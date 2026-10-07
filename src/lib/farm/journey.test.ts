// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import { sampleTask, sampleSnapshot } from '../../testing/farm-fixtures';
import {
	track,
	waves,
	attention,
	readyToLand,
	agentMonogram,
	agentColour,
	phaseFor,
	nowSentence,
	whereYouComeIn,
	FARM_COPY,
	filterEvents,
	retryAgent,
	relativeTime,
	elapsed,
	planningKind,
	plannedCount,
	segment,
	statsLine,
	failureLine,
	runningCheck,
	freeLine,
	listIds,
	runLine,
	isQuiet,
	checksFailedAfter,
	worthALook,
	askedBy,
	strengths,
	transcriptKind,
	journeyHeading
} from './describe';
import type { AgentRun, TaskDetail } from './types';
const detail = (id = 'T1'): TaskDetail => ({
	task: sampleTask(id, { status: 'review' }),
	verification: { passed: true, unverified: false, results: [] },
	review: { decision: 'approve', summary: 'Good', issues: [] },
	handoff: null,
	runs: []
});
describe('Farm journey', () => {
	it('maps every running step, landing and cancellation', () => {
		expect(track(sampleTask('T', { status: 'running' }))).toEqual([
			'done',
			'done',
			'current',
			'soft',
			'soft',
			'soft'
		]);
		expect(track(sampleTask('T', { status: 'assigned' }))[1]).toBe('current');
		expect(track(sampleTask('T', { status: 'verification' }))[3]).toBe('current');
		expect(track(sampleTask('T', { status: 'review' }))[4]).toBe('current');
		expect(track(sampleTask('T', { status: 'done' }))).toEqual(Array(6).fill('done'));
		expect(track(sampleTask('T', { status: 'cancelled' }))).toEqual(Array(6).fill('soft'));
		expect(track(sampleTask('T'))[0]).toBe('waiting');
	});
	it('requires checks and approval before inviting a merge', () => {
		const d = detail();
		expect(readyToLand(d)).toBe(true);
		expect(track(d.task, d)[5]).toBe('yourTurn');
		d.verification!.unverified = true;
		expect(readyToLand(d)).toBe(false);
		d.verification!.unverified = false;
		d.review!.decision = 'request_changes';
		expect(readyToLand(d)).toBe(false);
		d.task.status = 'cancelled';
		expect(readyToLand(d)).toBe(false);
	});
	it('marks a failed check at Checks', () => {
		const d = detail();
		d.task.status = 'blocked';
		d.verification!.passed = false;
		expect(track(d.task, d)).toEqual(['done', 'done', 'done', 'failed', 'soft', 'soft']);
	});
	it('orders dependency waves and isolates cycles and missing prerequisites', () => {
		const a = sampleTask('A');
		const b = sampleTask('B', { dependsOn: ['A'] });
		const c = sampleTask('C', { dependsOn: ['B'] });
		expect(waves([c, b, a]).map((w) => w.tasks.map((t) => t.id))).toEqual([['A'], ['B'], ['C']]);
		expect(
			waves([sampleTask('X', { dependsOn: ['Y'] }), sampleTask('Y', { dependsOn: ['X'] }), a]).map(
				(w) => w.unordered
			)
		).toEqual([false, true]);
		expect(waves([sampleTask('Z', { dependsOn: ['missing'] })])[0].unordered).toBe(true);
	});
	it('keeps dependency waits out of Needs you', () => {
		const a = sampleTask('A');
		const b = sampleTask('B', { status: 'blocked', dependsOn: ['A'] });
		expect(attention(b, [a, b], undefined, [])).toBe(false);
		a.status = 'done';
		expect(attention(b, [a, b], undefined, [])).toBe(true);
	});
	it('uses stable provider identities and readable summary copy', () => {
		const a = sampleSnapshot().agents[0].definition;
		expect(agentMonogram(a)).toBe('CC');
		expect(agentColour(a)).toBe('var(--lane-1)');
		const stuck = [sampleTask('X', { status: 'failed' }), sampleTask('Y', { status: 'failed' })];
		expect(
			nowSentence(
				[
					sampleTask('A', { status: 'running' }),
					sampleTask('B', { status: 'verification' }),
					...stuck
				],
				stuck,
				false
			)
		).toBe('One agent is working, one task is being checked, and two need you.');
		expect(nowSentence([], [], true)).toBe(FARM_COPY.paused);
		expect(whereYouComeIn('semiAuto')).toBe('only to approve merges, and when a task is stuck.');
	});

	it('counts a reviewed task waiting for a merge once, as needing you', () => {
		const ready = sampleTask('R', { status: 'review' });
		const checking = sampleTask('C', { status: 'review' });
		expect(nowSentence([ready, checking], [ready], false)).toBe(
			'One task is being checked, and one needs you.'
		);
	});

	it('says time the way the board does', () => {
		expect(relativeTime(1_000, 9_000)).toBe('8 s');
		expect(relativeTime(0, 6 * 60_000 + 12_000)).toBe('6 min');
		expect(elapsed(50 * 60_000)).toBe('50 min');
		expect(elapsed(134 * 60_000)).toBe('2 h 14 min');
		expect(elapsed(120 * 60_000)).toBe('2 h');
	});

	it('labels what the planner is doing', () => {
		expect(planningKind('Reading AGENTS.md')).toBe('reading');
		expect(planningKind('Proposed T-02: gitea.rs')).toBe('task');
		expect(planningKind('Review comments depend on listing PRs first')).toBe('thinking');
		expect(plannedCount(0, true)).toBe('no tasks yet');
		expect(plannedCount(1, true)).toBe('1 task so far');
		expect(plannedCount(9, false)).toBe('9 tasks');
	});

	it('puts the farm in its phase', () => {
		expect(phaseFor(null, null, 0)).toBe(0);
		expect(phaseFor(null, null, 1)).toBe(1);
		expect(phaseFor(sampleSnapshot().farm, null, 1)).toBe(2);
		expect(phaseFor(sampleSnapshot([sampleTask('A', { status: 'running' })]).farm, null, 1)).toBe(
			3
		);
		const done = sampleSnapshot([sampleTask('A', { status: 'done' })]).farm!;
		done.status = 'completed';
		expect(phaseFor(done, null, 1)).toBe(4);
	});
	it('filters events without showing transcript floods', () => {
		expect(
			filterEvents(
				[
					{ kind: 'agentOutput', run: 'R', task: 'T', line: 'hi', atMs: 1 },
					{ kind: 'mergeCompleted', task: 'T', branch: 'b', ok: true, error: null, atMs: 2 }
				],
				'Landed',
				[]
			)
		).toHaveLength(1);
	});
	it('never suggests a disabled agent for retry', () => {
		const s = sampleSnapshot();
		expect(
			retryAgent(sampleTask('T', { implementedBy: 'claude-1' }), s.agents, [])?.definition.id
		).toBe('claude-1');
	});
});

describe('Building, Task and Plan review copy', () => {
	const run = (over: Partial<AgentRun> = {}): AgentRun => ({
		id: 'R',
		task: 'T',
		agent: 'claude-1',
		phase: 'implementation',
		outcome: { state: 'running' },
		command: [],
		startedMs: 0,
		endedMs: null,
		logFile: null,
		lastOutputMs: null,
		...over
	});

	it('places each task in the bar by what it waits on', () => {
		const ready = detail('R');
		expect(segment(sampleTask('D', { status: 'done' }), undefined, false)).toBe('landed');
		expect(segment(ready.task, ready, true)).toBe('ready');
		expect(segment(sampleTask('F', { status: 'failed' }), undefined, true)).toBe('stuck');
		expect(segment(sampleTask('W', { status: 'running' }), undefined, false)).toBe('working');
		expect(segment(sampleTask('V', { status: 'verification' }), undefined, false)).toBe('checking');
		expect(segment(sampleTask('Q'), undefined, false)).toBe('waiting');
	});

	it('counts a branch in commits and lines', () => {
		expect(statsLine(null)).toBeNull();
		expect(statsLine({ commits: 1, files: [] })).toBe('1 commit');
		expect(
			statsLine({
				commits: 4,
				files: [
					{ path: 'a', added: 400, removed: 6, binary: false },
					{ path: 'b', added: 12, removed: 12, binary: false }
				]
			})
		).toBe('4 commits, +412 −18');
	});

	it('says why a task stopped, in the agent’s words first', () => {
		const task = sampleTask('T', { status: 'failed', attempts: 3 });
		const failing = {
			...detail('T'),
			verification: {
				passed: false,
				unverified: false,
				results: [
					{
						command: 'cargo test',
						passed: false,
						output: 'compiling\nlibsecret-1 was not found\n',
						durationMs: 1
					}
				]
			}
		};
		expect(failureLine(task, failing, 3)).toBe(
			'Failed its checks 3 of 3 times: libsecret-1 was not found'
		);
		failing.handoff = {
			status: 'failed',
			summary: 'libsecret-1 is missing on this machine.',
			filesChanged: [],
			commits: [],
			tests: [],
			risks: [],
			questions: [],
			proposedTasks: []
		};
		expect(failureLine(task, failing, 3)).toBe(
			'Failed its checks 3 of 3 times: libsecret-1 is missing on this machine'
		);
		expect(failureLine(sampleTask('S', { attempts: 1 }), undefined, 1)).toBe('Stopped 1 of 1 time');
	});

	it('names the check running now, and none once it finished', () => {
		const started = {
			kind: 'verificationStarted' as const,
			task: 'T',
			command: 'cargo test',
			atMs: 1
		};
		expect(runningCheck('T', [started])).toBe('cargo test');
		expect(
			runningCheck('T', [
				started,
				{ ...started, kind: 'verificationFinished' as const, passed: true, output: '', atMs: 2 }
			])
		).toBeNull();
		expect(runningCheck('Other', [started])).toBeNull();
	});

	it('says what a free agent picks up next', () => {
		const tasks = [
			sampleTask('A', { status: 'running' }),
			sampleTask('B', { status: 'waiting', assignedAgent: 'codex', dependsOn: ['A'] }),
			sampleTask('C', { status: 'verification', implementedBy: 'pi' })
		];
		expect(freeLine('codex', tasks)).toBe('takes B when A lands');
		expect(freeLine('pi', tasks)).toBe('its C is in checks');
		expect(freeLine('pi', tasks, true)).toBe('finished C, its checks are running');
		expect(freeLine('nobody', tasks)).toBeNull();
		expect(listIds(['A', 'B', 'C'])).toBe('A, B and C');
	});

	it('marks a run quiet after three minutes, and only a live one', () => {
		const now = 10 * 60_000;
		expect(runLine(run({ startedMs: now - 6 * 60_000, lastOutputMs: now - 8_000 }), now)).toBe(
			'6 min · spoke 8 s ago'
		);
		const quiet = run({ startedMs: now - 11 * 60_000, lastOutputMs: now - 4 * 60_000 });
		expect(isQuiet(quiet, now)).toBe(true);
		expect(runLine(quiet, now)).toBe('11 min · quiet for 4 min');
		expect(isQuiet({ ...quiet, outcome: { state: 'completed', exitCode: 0 } }, now)).toBe(false);
	});

	it('stripes a run whose checks failed before the next attempt', () => {
		const first = run({ id: 'R1', startedMs: 0, endedMs: 10 });
		const second = run({ id: 'R2', startedMs: 20, endedMs: 30 });
		const failed = {
			kind: 'verificationFinished' as const,
			task: 'T',
			command: 'test',
			passed: false,
			output: '',
			atMs: 15
		};
		expect(checksFailedAfter(first, [first, second], [failed])).toBe(true);
		expect(checksFailedAfter(second, [first, second], [failed])).toBe(false);
		expect(checksFailedAfter({ ...first, phase: 'review' }, [first], [failed])).toBe(false);
	});

	it('finds what is worth a look in a plan', () => {
		const tasks = [
			sampleTask('A', { allowedPaths: ['src/hosts/'] }),
			sampleTask('B', { allowedPaths: ['src/hosts/gitea.rs'] }),
			sampleTask('C', { dependsOn: ['A', 'B'], allowedPaths: ['docs/'] })
		];
		expect(worthALook(tasks)).toEqual([
			'C waits for two tasks, so it starts last.',
			'A and B both touch src/hosts/; whichever lands second is rebased on the first.'
		]);
		expect(
			worthALook([
				sampleTask('X', { allowedPaths: ['src/a'] }),
				sampleTask('Y', { allowedPaths: ['src/ab'] })
			])
		).toEqual([]);
	});

	it('names who asked, and what an agent is good at', () => {
		const agents = sampleSnapshot().agents;
		expect(askedBy({ kind: 'person' }, agents)).toBe('You');
		expect(askedBy({ kind: 'planned', agent: 'claude-1' }, agents)).toBe('Planned by Claude Code');
		expect(strengths(agents[0].definition)).toBe('Plans well');
		expect(
			strengths({ ...agents[0].definition, capabilities: ['coding', 'review', 'backend'] })
		).toBe('Backend, review');
	});

	it('colours transcript lines by what they are', () => {
		expect(transcriptKind('$ cargo check')).toBe('command');
		expect(transcriptKind('+ src/hosts/gitea/review.rs +86')).toBe('edit');
		expect(transcriptKind('error[E0308]: mismatched types')).toBe('error');
		expect(transcriptKind('Mapping the review payload')).toBe('think');
	});

	it('heads the stepper with where the task is', () => {
		const agents = sampleSnapshot().agents;
		const ready = detail('R');
		expect(journeyHeading(ready.task, ready, undefined, agents)).toBe(FARM_COPY.ready);
		expect(journeyHeading(sampleTask('W', { status: 'running' }), undefined, run(), agents)).toBe(
			'Claude Code is writing it'
		);
		expect(
			journeyHeading(sampleTask('S', { status: 'failed' }), undefined, undefined, agents)
		).toBe('Stuck. The farm stopped trying.');
	});
});
