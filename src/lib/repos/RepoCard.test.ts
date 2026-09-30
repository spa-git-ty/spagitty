// SPDX-License-Identifier: GPL-3.0-or-later

import { readFileSync } from 'node:fs';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, render } from '../../testing/mount';
import type { RepoSummary } from '$lib/types';

vi.mock('$lib/api', () => ({
	recentRepos: vi.fn(() => Promise.resolve([])),
	forgetRepo: vi.fn(() => Promise.resolve())
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import * as api from '$lib/api';
import { calls as repoCalls, control as repoControl } from '../../testing/repo-store.svelte';
import RepoCard from './RepoCard.svelte';

const forgetRepo = vi.mocked(api.forgetRepo);

function card(overrides: Partial<RepoSummary> = {}): RepoSummary {
	return {
		path: '/repos/fixture',
		name: 'fixture',
		present: true,
		bare: false,
		branch: 'main',
		detached: false,
		short: 'aaaaaaa',
		summary: 'Merge feature/split-view',
		time: Math.floor(Date.now() / 1000) - 3600,
		dirty: 0,
		conflicts: 0,
		stashes: 0,
		branches: 1,
		...overrides
	};
}

beforeEach(() => {
	vi.clearAllMocks();
	repoControl.reset();
});

describe('RepoCard', () => {
	it('names the repository, its branch, its path and what it was last doing', () => {
		const view = render(RepoCard, { card: card() });

		expect(view.text()).toContain('fixture');
		expect(view.text()).toContain('main');
		expect(view.get('.path').textContent).toBe('/repos/fixture');
		expect(view.text()).toContain('Merge feature/split-view');

		view.destroy();
	});

	it('says where a detached HEAD is rather than showing no branch', () => {
		const view = render(RepoCard, { card: card({ branch: null, detached: true }) });
		expect(view.text()).toContain('detached at aaaaaaa');
		view.destroy();
	});

	it('says a repository has no commits yet', () => {
		const view = render(RepoCard, {
			card: card({ branch: null, short: null, summary: null, detached: false })
		});
		expect(view.text()).toContain('no commits yet');
		view.destroy();
	});

	it('earns a chip per thing going on, and shows none when nothing is', () => {
		const quiet = render(RepoCard, { card: card() });
		expect(quiet.all('.chip')).toHaveLength(0);
		quiet.destroy();

		const busy = render(RepoCard, {
			card: card({ dirty: 2, stashes: 1, conflicts: 3, bare: false })
		});
		const labels = busy.all('.chip').map((c) => c.textContent?.trim());
		expect(labels).toEqual(['3 conflicted', '2 changed', '1 stashed']);
		busy.destroy();
	});

	it('leads with conflicts, which are the thing that stops work', () => {
		const view = render(RepoCard, { card: card({ dirty: 5, conflicts: 1 }) });
		expect(view.all('.chip')[0].textContent?.trim()).toBe('1 conflicted');
		view.destroy();
	});

	it('says a missing repository is missing and does not offer to open it', () => {
		const view = render(RepoCard, { card: card({ present: false }) });

		expect(view.text()).toContain('Not here any more');
		const open = view.all('button').find((b) => b.textContent?.includes('Open'));
		expect((open as HTMLButtonElement).disabled).toBe(true);
		// The path stays visible: it is the only clue to where it went.
		expect(view.get('.path').textContent).toBe('/repos/fixture');

		view.destroy();
	});

	it('marks the open repository and does not offer to open it again', () => {
		repoControl.setInfo({
			path: '/repos/fixture',
			name: 'fixture',
			bare: false,
			lastFetched: null,
			head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
		});
		const view = render(RepoCard, { card: card() });

		expect(view.text()).toContain('open');
		const open = view.all('button').find((b) => b.textContent?.includes('Already open'));
		expect((open as HTMLButtonElement).disabled).toBe(true);

		view.destroy();
	});

	it('opens a repository from its card', () => {
		const view = render(RepoCard, { card: card() });

		const open = view.all('button').find((b) => b.textContent?.trim() === 'Open');
		click(open as HTMLElement);

		expect(repoCalls.opened).toEqual(['/repos/fixture']);
		view.destroy();
	});

	it('forgetting says plainly that the directory is not touched', () => {
		const view = render(RepoCard, { card: card() });

		const forget = view.all('button').find((b) => b.textContent?.includes('Forget'));
		expect(forget?.getAttribute('title')).toContain('directory on disk is not touched');

		click(forget as HTMLElement);
		expect(forgetRepo).toHaveBeenCalledWith('/repos/fixture');

		view.destroy();
	});

	it('renders an idle card dashed', () => {
		const view = render(RepoCard, { card: card(), idle: true });
		expect(view.get('.card').classList.contains('idle')).toBe(true);
		view.destroy();
	});

	it('shows a branch count only when there is more than one', () => {
		const one = render(RepoCard, { card: card({ branches: 1 }) });
		expect(one.text()).not.toContain('branches');
		one.destroy();

		const many = render(RepoCard, { card: card({ branches: 6 }) });
		expect(many.text()).toContain('6 branches');
		many.destroy();
	});

	it('keeps a long branch name and the count as separate elements', () => {
		const view = render(
			RepoCard,
			{ card: card({ branch: 'correzioni-e-rilavorazioni-su-i-bilanci', branches: 7 }) }
		);

		// Both are present and neither has swallowed the other's text.
		expect(view.get('.ref').textContent).toContain('correzioni-e-rilavorazioni');
		expect(view.get('.count').textContent).toBe('7 branches');

		view.destroy();
	});
});

/**
 * BUG-006 — a long branch name overlapping the branch count.
 *
 * These assertions read the stylesheets rather than a rendered card, for the
 * reason `src/lib/ui/btn.test.ts` sets out: the test environment mounts
 * components without applying any CSS, so `getComputedStyle` here would report
 * the same thing whether the rules exist or not. What can be checked honestly
 * is the rule that caused it.
 *
 * The cause was not the ellipsis rules — those were already right. It was the
 * flexbox automatic minimum size: a flex item's `min-width` defaults to `auto`,
 * whose used value is the content's own width, so the chip refused to shrink
 * below a long branch name and `max-width: 100%` never applied. The overflow
 * then painted over the "N branches" span beside it.
 */
describe('BUG-006 — the chip gives way before its neighbours', () => {
	const chip = readFileSync('src/lib/ui/RefChip.svelte', 'utf8');
	const repoCard = readFileSync('src/lib/repos/RepoCard.svelte', 'utf8');

	/** The body of one CSS rule, by selector, out of a component's `<style>`. */
	function rule(source: string, selector: string): string {
		const found = new RegExp(`${selector.replace(/[.\s]/g, (c) => `\\${c}`)}\\s*\\{([^}]*)\\}`).exec(
			source
		);
		if (!found) throw new Error(`no rule for ${selector}`);
		return found[1];
	}

	it('lets the chip shrink below its content width', () => {
		expect(rule(chip, '.ref')).toMatch(/min-width:\s*0/);
	});

	/** Without these the `min-width` has nothing to do; all four work together. */
	it('still asks the chip to ellipsise rather than wrap', () => {
		const ref = rule(chip, '.ref');
		expect(ref).toMatch(/max-width:\s*100%/);
		expect(ref).toMatch(/overflow:\s*hidden/);
		expect(ref).toMatch(/text-overflow:\s*ellipsis/);
		expect(ref).toMatch(/white-space:\s*nowrap/);
	});

	/**
	 * The branch name is the part that gives way, never the count: "7 branches"
	 * is four characters a card is useless without, and the name has an ellipsis
	 * and a `title` to fall back on.
	 */
	it('holds the branch count at its full width', () => {
		expect(rule(repoCard, '.branch .count')).toMatch(/flex:\s*none/);
	});

	it('keeps the row that holds them able to shrink', () => {
		expect(rule(repoCard, '.branch')).toMatch(/min-width:\s*0/);
	});
});
