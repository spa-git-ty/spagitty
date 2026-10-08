// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merge with an agent (2.0): the pure parts, assigning, and landing at
 * Unattended.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { aMerge, aSnapshot, aStep } from '../../testing/agent-fixtures';
import { conflicts, forecast } from '../../testing/merger-fixtures';

vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/agents/api', () => ({
	control: vi.fn(() => Promise.resolve()),
	start: vi.fn((r) => Promise.resolve({ ...r })),
	consent: vi.fn(() => Promise.resolve())
}));

import * as api from '$lib/api';
import * as agentsApi from '$lib/agents/api';
import { agents } from '$lib/agents/store.svelte';
import { control as repoControl } from '../../testing/repo-store.svelte';
import * as work from './agent.svelte';
import { resolving } from './resolve.svelte';
import { merger } from './store.svelte';

beforeEach(async () => {
	vi.clearAllMocks();
	work.forgetActed();
	merger.reset();
	resolving.reset();
	agents.reset(aSnapshot());
	repoControl.reset();
	repoControl.setInfo({ path: '/work/spagitty', name: 'spagitty', bare: false, head: { branch: 'main', detached: false, id: 'a', short: 'a' }, lastFetched: null } as never);
	vi.mocked(api.inTauri).mockReturnValue(false);
	vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
	vi.mocked(api.mergerConflicts).mockResolvedValue(conflicts());
	vi.mocked(api.mergerLand).mockResolvedValue({ target: 'main', commit: 'c'.repeat(40), short: 'ccccccc', written: 1 });
	merger.seed({ a: 'main', b: 'feat/tab-drag', forecast: forecast() });
});

describe('words', () => {
	it('credit each agent in a trailer, once', () => {
		expect(work.withTrailers("Merge branch 'feat' into main", ['Codex'])).toBe(
			"Merge branch 'feat' into main\n\nCo-authored-by: Codex <agent@spagitty.invalid>"
		);
		const once = work.withTrailers('x', ['Codex']);
		expect(work.withTrailers(once, ['Codex'])).toBe(once);
		expect(work.withTrailers('x', [])).toBe('x');
	});

	it('know which conflict the agent is on', () => {
		expect(work.workingOn(aMerge())).toEqual({ path: 'src/lib/chrome/Tabs.svelte', index: 1 });
		expect(work.workingOn(aMerge({ state: 'done' }))).toBeNull();
		expect(work.workingOn(aMerge({ steps: [aStep()] }))).toBeNull();
		expect(work.workingOn(null)).toBeNull();
	});
});

describe('assigning', () => {
	it('reads the conflicts first, and hands them over with both tips and the receiving branch', async () => {
		await work.assign({ agent: 'codex', level: 'signOff', note: '', lands: true });
		const sent = vi.mocked(agentsApi.start).mock.calls[0][0];
		expect(sent.target).toMatchObject({ kind: 'merge', a: 'main', b: 'feat/tab-drag', into: 'main' });
		expect(sent.lands).toBe(true);
		expect(sent.work.job === 'merge' && sent.work.files.map((f) => f.path)).toEqual([
			'src/lib/chrome/Tabs.svelte',
			'src/lib/metrics.ts',
			'CHANGELOG.md'
		]);
	});

	it('resumes from the record, with the same hand-over', async () => {
		await work.resume(aMerge({ state: 'stopped', lands: true }));
		expect(vi.mocked(agentsApi.start).mock.calls[0][0]).toMatchObject({ resume: 'merge-1', lands: true, level: 'signOff' });
	});
});

describe('landing at Unattended', () => {
	it('lands what was chosen, with the trailer, and says so to the agent', async () => {
		await resolving.open('/work/spagitty', null);
		work.follow();
		for (const file of resolving.files) {
			for (const region of file.regions) {
				resolving.choose(file.path, region.index, { mode: 'a' }, { agent: 'Codex', decided: 'agent' });
			}
		}
		agents.absorb(aMerge({ repo: '/work/spagitty', lastAct: { kind: 'land' } }));
		await vi.waitFor(() => expect(api.mergerLand).toHaveBeenCalled());
		expect(vi.mocked(api.mergerLand).mock.calls[0][0].message).toContain('Co-authored-by: Codex <agent@spagitty.invalid>');
		await vi.waitFor(() => expect(agentsApi.control).toHaveBeenCalledWith('merge-1', { kind: 'acted', ok: true }));
	});

	it('refuses with the reason when a conflict is left', async () => {
		await resolving.open('/work/spagitty', null);
		work.follow();
		agents.absorb(aMerge({ repo: '/work/spagitty', lastAct: { kind: 'land' } }));
		await vi.waitFor(() =>
			expect(agentsApi.control).toHaveBeenCalledWith('merge-1', {
				kind: 'acted',
				ok: false,
				message: 'A conflict was left unresolved, so nothing was landed.'
			})
		);
		expect(api.mergerLand).not.toHaveBeenCalled();
	});

	it('refuses when Merger is on another merge', async () => {
		work.follow();
		agents.absorb(aMerge({ repo: '/elsewhere', lastAct: { kind: 'land' } }));
		await vi.waitFor(() =>
			expect(agentsApi.control).toHaveBeenCalledWith('merge-1', expect.objectContaining({ message: 'Merger is no longer on this merge.' }))
		);
	});
});
