// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Settings › Agents on screen (2.0), and the store it reads.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { click, fire, render } from '../../testing/mount';
import { aLocal, aRemote, aReview, aSnapshot, nothingSetUp, someRules } from '../../testing/agent-fixtures';

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('./api', () => ({
	snapshot: vi.fn(),
	list: vi.fn(() => Promise.resolve([])),
	setJobs: vi.fn(() => Promise.resolve()),
	setCodexFullAccess: vi.fn(() => Promise.resolve()),
	setOmpOptions: vi.fn(() => Promise.resolve()),
	setAgyAutoApprove: vi.fn(() => Promise.resolve()),
	setRules: vi.fn(() => Promise.resolve()),
	setDefaults: vi.fn(() => Promise.resolve()),
	remove: vi.fn(() => Promise.resolve()),
	testLocal: vi.fn(),
	testRemote: vi.fn(),
	takeOffer: vi.fn(() => Promise.resolve()),
	saveCustom: vi.fn(() => Promise.resolve()),
	start: vi.fn(),
	consent: vi.fn(() => Promise.resolve()),
	control: vi.fn(() => Promise.resolve()),
	forget: vi.fn(() => Promise.resolve())
}));

import * as api from './api';
import { control as repoControl } from '../../testing/repo-store.svelte';
import { agents } from './store.svelte';
import AgentsSection from './AgentsSection.svelte';

const repoInfo = {
	path: '/work/spagitty',
	name: 'spagitty',
	bare: false,
	head: { kind: 'branch', name: 'main', id: 'abc' },
	lastFetched: null
} as never;

beforeEach(() => {
	vi.clearAllMocks();
	repoControl.reset();
	repoControl.setInfo(repoInfo);
	agents.reset(aSnapshot());
});

describe('the section', () => {
	it('saves agy tool auto-approval and explains command access', async () => {
		agents.reset(aSnapshot({ local: [aLocal({ id: 'agy', name: 'agy', provider: 'agy' })] }));
		const view = render(AgentsSection, {});
		click(view.all('button').find((b) => b.textContent?.trim() === 'Auto-approve tools')!);
		await vi.waitFor(() => expect(api.setAgyAutoApprove).toHaveBeenCalledWith(true));
		agents.reset(aSnapshot({ agyAutoApprove: true, local: [aLocal({ id: 'agy', name: 'agy', provider: 'agy' })] }));
		flushSync();
		expect(view.text()).toContain('agy can run commands and access files without asking.');
		click(view.all('button').find((b) => b.textContent?.trim() === 'Configured rules')!);
		await vi.waitFor(() => expect(api.setAgyAutoApprove).toHaveBeenCalledWith(false));
	});

	it('edits and saves the model and profile for OMP launches', async () => {
		const snapshot = aSnapshot({ omp: { model: 'provider/old-model', profile: 'work' },
			local: [aLocal({ id: 'pi', name: 'Oh My Pi', provider: 'ohMyPi' })] });
		vi.mocked(api.snapshot).mockResolvedValueOnce(snapshot);
		agents.reset(snapshot);
		const view = render(AgentsSection, {});
		expect(view.text()).toContain('Model: provider/old-model · Profile: work');
		click(view.all('button').find((b) => b.textContent?.trim() === 'Model and profile…')!);
		const model = view.get('#omp-model') as HTMLInputElement;
		const profile = view.get('#omp-profile') as HTMLInputElement;
		expect(model.value).toBe('provider/old-model');
		expect(profile.value).toBe('work');
		model.value = ' provider/new-model ';
		profile.value = '';
		fire(model, 'input');
		fire(profile, 'input');
		click(view.all('button').find((b) => b.textContent?.trim() === 'Save')!);
		await vi.waitFor(() => expect(api.setOmpOptions).toHaveBeenCalledWith({ model: 'provider/new-model', profile: '' }));
	});

	it('saves Codex Full Access and shows its scope', async () => {
		agents.reset(aSnapshot({ local: [aLocal({ id: 'codex', name: 'Codex', provider: 'codex' })] }));
		const view = render(AgentsSection, {});
		expect(view.text()).not.toContain('Codex commands can access');
		click(view.all('button').find((b) => b.textContent?.trim() === 'Full Access')!);
		await vi.waitFor(() => expect(api.setCodexFullAccess).toHaveBeenCalledWith(true));
		agents.reset(aSnapshot({ codexFullAccess: true, local: [aLocal({ id: 'codex', name: 'Codex', provider: 'codex' })] }));
		flushSync();
		expect(view.text()).toContain('Codex commands can access files outside the repository and use the network.');
		click(view.all('button').find((b) => b.textContent?.trim() === 'Sandboxed')!);
		await vi.waitFor(() => expect(api.setCodexFullAccess).toHaveBeenCalledWith(false));
	});
	it('lists what detection found, with versions and paths, and what was not', () => {
		const view = render(AgentsSection, {});
		expect(view.text()).toContain('On this machine');
		expect(view.text()).toContain('2.1.259 · /usr/bin/claude');
		expect(view.text()).toContain('Not installed');
		// A missing agent offers no jobs and no test.
		const rows = view.all('.agent');
		expect(rows[1].querySelector('[aria-label^="What"]')).toBeNull();
	});

	it('shows an API agent by its model and key ending, never the key', () => {
		const view = render(AgentsSection, {});
		expect(view.text()).toContain('claude-sonnet-5-5');
		expect(view.text()).toContain('••••7Qx2 · 40k / run');
	});

	it('flips what an agent may do', async () => {
		const view = render(AgentsSection, {});
		const review = view.all('.agent')[0].querySelector<HTMLElement>('[aria-label="What Claude Code may do"] button')!;
		click(review);
		await vi.waitFor(() =>
			expect(api.setJobs).toHaveBeenCalledWith('claude', { review: false, merge: true, farm: true })
		);
	});

	it('caps the repository’s highest level from its card', async () => {
		const view = render(AgentsSection, {});
		const card = view.get('aside[aria-label="This repository"]');
		expect(card.textContent).toContain('spagitty');
		const unattended = [...card.querySelectorAll<HTMLElement>('[aria-label="Highest level"] button')].find(
			(b) => b.textContent?.trim() === 'Unattended'
		)!;
		click(unattended);
		await vi.waitFor(() =>
			expect(api.setRules).toHaveBeenCalledWith('/work/spagitty', expect.objectContaining({ highest: 'unattended' }))
		);
	});

	it('says agents may not approve until somebody says they may', () => {
		const view = render(AgentsSection, {});
		const card = view.get('aside[aria-label="This repository"]');
		expect(card.textContent).toMatch(/Agents may approve\s*off/);
		expect(card.textContent).toMatch(/Mark agent comments\s*on/);
	});

	it('lets consent be revoked', async () => {
		agents.reset(aSnapshot({ rules: someRules({ consent: ['anthropic'] }) }));
		const view = render(AgentsSection, {});
		expect(view.text()).toContain('Code may go to');
		const revoke = view.all('button').find((b) => b.textContent?.trim() === 'revoke')!;
		click(revoke);
		await vi.waitFor(() =>
			expect(api.setRules).toHaveBeenCalledWith('/work/spagitty', expect.objectContaining({ consent: [] }))
		);
	});

	it('with nothing set up, lists what was looked for and the two ways to add, and lectures nobody', () => {
		agents.reset(nothingSetUp());
		const view = render(AgentsSection, {});
		expect(view.text()).toContain('Not installed');
		expect(view.all('button').filter((b) => b.textContent?.trim() === 'Add')).toHaveLength(2);
		expect(view.text()).not.toMatch(/you need|set up an agent|AI/);
	});

	it('offers agents found in repositories, once', async () => {
		agents.reset(aSnapshot({ offer: [aLocal({ id: 'custom-bot', name: 'review-bot' }).definition] }));
		const view = render(AgentsSection, {});
		expect(view.text()).toContain('Found in your repositories’ farms');
		click(view.all('button').find((b) => b.textContent?.trim() === 'not now')!);
		await vi.waitFor(() => expect(api.takeOffer).toHaveBeenCalledWith([]));
	});

	it('tests a local agent and says what answered', async () => {
		vi.mocked(api.testLocal).mockResolvedValue({ ok: true, said: 'ok', ms: 3200 });
		const view = render(AgentsSection, {});
		const test = [...view.all('.agent')[0].querySelectorAll<HTMLElement>('button')].find(
			(b) => b.textContent?.trim() === 'Test'
		)!;
		click(test);
		await vi.waitFor(() => expect(view.text()).toContain('Answered in 3.2 s'));
	});
});

describe('the store', () => {
	it('offers only agents set up for the job and allowed here', () => {
		agents.reset(
			aSnapshot({
				local: [aLocal(), aLocal({ id: 'codex', name: 'Codex', jobs: { review: false, merge: true, farm: true } })],
				remote: [aRemote()],
				rules: someRules({ agents: ['claude', 'codex'] })
			})
		);
		expect(agents.usable('review').map((u) => u.id)).toEqual(['claude']);
		expect(agents.usable('merge').map((u) => u.id)).toEqual(['claude', 'codex']);
	});

	it('is empty with nothing set up: no agent, no trace', () => {
		agents.reset(nothingSetUp());
		expect(agents.usable('review')).toEqual([]);
		expect(agents.usable('merge')).toEqual([]);
		agents.reset(null);
		expect(agents.usable('review')).toEqual([]);
	});

	it('starts from the default, or the first offered', () => {
		expect(agents.defaultFor('review')).toBe('claude');
		expect(agents.defaultFor('merge')).toBe('claude');
	});

	it('finds a pull request’s assignment, live first, and says when it waits', () => {
		agents.reset(aSnapshot(), [aReview({ id: 'old', state: 'done' }), aReview()]);
		expect(agents.forReview('spa-git-ty', 'spagitty', 214)?.id).toBe('review-1');
		expect(agents.forReview('spa-git-ty', 'spagitty', 1)).toBeNull();
		expect(agents.waiting('review')).toBe(true);
		expect(agents.waiting('merge')).toBe(false);
	});

	it('takes an event and tells the listeners what it was before', () => {
		agents.reset(aSnapshot(), [aReview()]);
		const seen: string[] = [];
		const stop = agents.subscribe((next, before) => seen.push(`${before?.state}→${next.state}`));
		agents.absorb(aReview({ state: 'done' }));
		stop();
		agents.absorb(aReview({ state: 'failed' }));
		flushSync();
		expect(seen).toEqual(['waiting→done']);
		expect(agents.assignments[0].state).toBe('failed');
	});

	it('asks once before code goes to a provider, then keeps the answer', async () => {
		const { dialog } = await import('$lib/ui/dialog.svelte');
		const confirm = vi.spyOn(dialog, 'confirm').mockResolvedValue(true);
		vi.mocked(api.start)
			.mockRejectedValueOnce({ kind: 'consent', message: 'This sends parts of this repository to Anthropic.' })
			.mockResolvedValueOnce(aReview());
		const request = {
			repo: '/work/spagitty',
			agent: 'api-1',
			level: 'stepByStep' as const,
			note: '',
			target: aReview().target,
			work: { job: 'review' as const, description: '', threads: [], checks: '', conflictFixes: [] },
			lands: false
		};
		const started = await agents.start(request, 'Anthropic', 'anthropic');
		expect(confirm).toHaveBeenCalledWith(expect.objectContaining({ title: 'Send code to Anthropic?' }));
		expect(api.consent).toHaveBeenCalledWith('/work/spagitty', 'anthropic');
		expect(started?.id).toBe('review-1');
	});

	it('reads a backend failure as its sentence', () => {
		expect(agents.failure({ kind: 'refused', message: 'No.' }).message).toBe('No.');
		expect(agents.failure(new Error('boom')).message).toBe('boom');
	});
});
