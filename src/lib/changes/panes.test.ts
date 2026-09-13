// SPDX-License-Identifier: GPL-3.0-or-later

import { readFileSync } from 'node:fs';
import { beforeEach, describe, expect, it } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import type { FileDiff, Signing, StatusEntry, WorkingCopy } from '$lib/types';

import { vi } from 'vitest';
vi.mock('$lib/changes/store.svelte', async () => await import('../../testing/changes-store.svelte'));

import { calls, control } from '../../testing/changes-store.svelte';
import FileColumn from './FileColumn.svelte';
import HunkPane from './HunkPane.svelte';
import MessageBox from './MessageBox.svelte';

function entry(path: string, status: StatusEntry['status'] = 'modified'): StatusEntry {
	return { path, status };
}

function work(overrides: Partial<WorkingCopy> = {}): WorkingCopy {
	return { staged: [], unstaged: [], conflicted: [], ...overrides };
}

function diff(overrides: Partial<FileDiff> = {}): FileDiff {
	return {
		path: 'core.txt',
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 1,
		removed: 1,
		hunks: [
			{
				oldStart: 1,
				oldLines: 2,
				newStart: 1,
				newLines: 2,
				header: '@@ -1,2 +1,2 @@',
				lines: [
					{ origin: 'context', old: 1, new: 1, text: 'kept' },
					{ origin: 'removed', old: 2, new: null, text: 'gone' },
					{ origin: 'added', old: null, new: 2, text: 'arrived' }
				]
			}
		],
		...overrides
	};
}

beforeEach(() => {
	control.reset();
});

describe('FileColumn', () => {
	it('shows staged and unstaged separately, with counts', () => {
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('b.txt')] }));
		const view = render(FileColumn, {});

		expect(view.text()).toContain('Staged');
		expect(view.text()).toContain('Unstaged');
		expect(view.text()).toContain('a.txt');
		expect(view.text()).toContain('b.txt');

		view.destroy();
	});

	it('draws staged rows solid and unstaged rows dashed', () => {
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('b.txt')] }));
		const view = render(FileColumn, {});

		expect(view.all('.row.solid')).toHaveLength(1);
		expect(view.all('.row.dashed')).toHaveLength(1);

		view.destroy();
	});

	it('shows a path in both lists when it is staged in part', () => {
		// Collapsing this into one row is what makes people commit something
		// they did not mean to.
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('a.txt')] }));
		const view = render(FileColumn, {});

		expect(view.all('.row').filter((r) => r.textContent?.includes('a.txt'))).toHaveLength(2);
		view.destroy();
	});

	/**
	 * FEAT-048. The controls that throw work away are asserted for where they
	 * are as much as for what they do: a discard button in the staged section
	 * would sit next to `−` and mean something very different.
	 */
	it('offers discard on unstaged rows and on no staged row', () => {
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('b.txt')] }));
		const view = render(FileColumn, {});

		const rows = view.all('.row');
		expect(rows[0].querySelector('.act.discard')).toBeNull();
		expect(rows[1].querySelector('.act.discard')).not.toBeNull();

		view.destroy();
	});

	it('says an untracked row will be deleted rather than reverted', () => {
		control.setWork(work({ unstaged: [entry('new.txt', 'untracked')] }));
		const view = render(FileColumn, {});

		expect(view.get('.act.discard').getAttribute('title')).toBe(
			'Delete new.txt — this cannot be undone'
		);

		view.destroy();
	});

	it('warns on a tracked row that the change cannot come back', () => {
		control.setWork(work({ unstaged: [entry('b.txt')] }));
		const view = render(FileColumn, {});

		expect(view.get('.act.discard').getAttribute('title')).toBe(
			'Discard changes to b.txt — this cannot be undone'
		);

		view.destroy();
	});

	it('offers Discard all beside Stage all, and neither with nothing unstaged', () => {
		control.setWork(work({ unstaged: [entry('b.txt')] }));
		const view = render(FileColumn, {});
		expect(view.text()).toContain('Discard all');
		view.destroy();

		control.setWork(work({ staged: [entry('a.txt')] }));
		const empty = render(FileColumn, {});
		expect(empty.text()).not.toContain('Discard all');
		empty.destroy();
	});

	it('marks untracked files with the untracked glyph', () => {
		control.setWork(work({ unstaged: [entry('new.txt', 'untracked')] }));
		const view = render(FileColumn, {});

		expect(view.get('.glyph').textContent?.trim()).toBe('?');
		view.destroy();
	});

	it('stages one file from its row', () => {
		control.setWork(work({ unstaged: [entry('b.txt')] }));
		const view = render(FileColumn, {});

		click(view.get('.row.dashed .act'));

		expect(calls.staged).toEqual([['b.txt']]);
		view.destroy();
	});

	it('unstages one file from its row', () => {
		control.setWork(work({ staged: [entry('a.txt')] }));
		const view = render(FileColumn, {});

		click(view.get('.row.solid .act'));

		expect(calls.unstaged).toEqual([['a.txt']]);
		view.destroy();
	});

	it('stages and unstages everything from the section headers', () => {
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('b.txt'), entry('c.txt')] }));
		const view = render(FileColumn, {});

		const stageAll = view.all('button').find((b) => b.textContent?.includes('Stage all'));
		const unstageAll = view.all('button').find((b) => b.textContent?.includes('Unstage all'));
		click(stageAll as HTMLElement);
		click(unstageAll as HTMLElement);

		expect(calls.staged).toEqual([['b.txt', 'c.txt']]);
		expect(calls.unstaged).toEqual([['a.txt']]);
		view.destroy();
	});

	it('opens a row on the side it belongs to', () => {
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('b.txt')] }));
		const view = render(FileColumn, {});

		click(view.get('.row.solid .open'));
		click(view.get('.row.dashed .open'));

		expect(calls.opened).toEqual([
			{ path: 'a.txt', side: 'staged' },
			{ path: 'b.txt', side: 'unstaged' }
		]);
		view.destroy();
	});

	it('marks the open row', () => {
		control.setWork(work({ unstaged: [entry('b.txt'), entry('c.txt')] }));
		control.setSelection({ path: 'c.txt', side: 'unstaged' });
		const view = render(FileColumn, {});

		const selected = view.all('.row.selected');
		expect(selected).toHaveLength(1);
		expect(selected[0].textContent).toContain('c.txt');
		view.destroy();
	});

	it('does not mark the same path on the other side as open', () => {
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('a.txt')] }));
		control.setSelection({ path: 'a.txt', side: 'staged' });
		const view = render(FileColumn, {});

		expect(view.all('.row.selected')).toHaveLength(1);
		expect(view.get('.row.solid').classList.contains('selected')).toBe(true);
		view.destroy();
	});

	it('says what conflicts mean and offers no action for them', () => {
		control.setWork(work({ conflicted: [entry('shared.txt')] }));
		const view = render(FileColumn, {});

		expect(view.text()).toContain('shared.txt');
		expect(view.text()).toContain('Nothing can be committed until these are resolved');
		expect(view.get('.row.conflicted').querySelector('.act')).toBeNull();
		view.destroy();
	});

	it('says each section is empty rather than showing nothing', () => {
		control.setWork(work());
		const view = render(FileColumn, {});

		expect(view.text()).toContain('Nothing staged');
		expect(view.text()).toContain('Nothing unstaged');
		view.destroy();
	});

	it('disables every action while a write is in flight', () => {
		control.setWork(work({ staged: [entry('a.txt')], unstaged: [entry('b.txt')] }));
		control.setBusy(true);
		const view = render(FileColumn, {});

		for (const button of view.all('.act')) {
			expect((button as HTMLButtonElement).disabled).toBe(true);
		}
		view.destroy();
	});
});

describe('HunkPane', () => {
	it('renders a hunk with numbers on both sides', () => {
		control.setSelection({ path: 'core.txt', side: 'unstaged' });
		control.setFile(diff());
		const view = render(HunkPane, {});

		expect(view.text()).toContain('@@ -1,2 +1,2 @@');
		expect(view.get('.line.removed').textContent).toContain('gone');
		expect(view.get('.line.added').textContent).toContain('arrived');
		view.destroy();
	});

	it('offers to stage a hunk when the unstaged side is open', () => {
		control.setSelection({ path: 'core.txt', side: 'unstaged' });
		control.setFile(diff());
		const view = render(HunkPane, {});

		expect(view.text()).toContain('stage hunk');
		click(view.get('.chip'));

		expect(calls.hunks).toEqual([{ index: 0, header: '@@ -1,2 +1,2 @@' }]);
		view.destroy();
	});

	it('offers to take it back out when the staged side is open', () => {
		control.setSelection({ path: 'core.txt', side: 'staged' });
		control.setFile(diff());
		const view = render(HunkPane, {});

		expect(view.text()).toContain('unstage hunk');
		view.destroy();
	});

	it('offers to discard a hunk on the unstaged side only', () => {
		// FEAT-048. On the staged side the change has been kept once already
		// and unstaging is the reversible way back to it, so a destructive
		// button there would sit next to a safe one reading almost the same.
		control.setSelection({ path: 'core.txt', side: 'unstaged' });
		control.setFile(diff());
		const unstaged = render(HunkPane, {});
		expect(unstaged.text()).toContain('discard hunk');
		unstaged.destroy();

		control.setSelection({ path: 'core.txt', side: 'staged' });
		control.setFile(diff());
		const staged = render(HunkPane, {});
		expect(staged.text()).not.toContain('discard hunk');
		staged.destroy();
	});

	it('paints the discard chip as the destructive one', () => {
		control.setSelection({ path: 'core.txt', side: 'unstaged' });
		control.setFile(diff());
		const view = render(HunkPane, {});

		const chips = view.all('.chip');
		expect(chips[0].classList.contains('danger')).toBe(false);
		expect(chips[1].classList.contains('danger')).toBe(true);

		view.destroy();
	});

	it('asks for a file when none is open', () => {
		const view = render(HunkPane, {});
		expect(view.text()).toContain('Select a file');
		view.destroy();
	});

	it('says so when a selected file has nothing to show', () => {
		control.setSelection({ path: 'core.txt', side: 'unstaged' });
		const view = render(HunkPane, {});
		expect(view.text()).toContain('Could not read this diff');
		view.destroy();
	});

	it('can shrink, so the hunks have a pane to paint in', () => {
		const source = readFileSync('src/lib/changes/HunkPane.svelte', 'utf8');
		expect(source).toMatch(/\.pane \{[^}]*flex: 1 1 0/);
		expect(source).toMatch(/\.pane \{[^}]*min-height: 0/);
	});

	it('paints the @@ header on the same surface as the lines, so the left rule continues through it', () => {
		const source = readFileSync('src/lib/changes/HunkPane.svelte', 'utf8');
		const style = source.slice(source.indexOf('<style>'));
		const head = style.match(/\.hunk-head \{[^}]+\}/)?.[0] ?? '';
		expect(head).toMatch(/background:\s*var\(--bg\)/);
		expect(head).not.toMatch(/var\(--panel\)/);
	});

	it('says a binary file has no hunks to stage individually', () => {
		control.setSelection({ path: 'logo.bin', side: 'unstaged' });
		control.setFile(diff({ path: 'logo.bin', binary: true, hunks: [] }));
		const view = render(HunkPane, {});

		expect(view.text()).toContain('Binary file');
		expect(view.find('.chip')).toBeNull();
		view.destroy();
	});

	it('distinguishes too-large from mode-only from binary', () => {
		control.setSelection({ path: 'huge.sql', side: 'unstaged' });
		control.setFile(diff({ tooLarge: true, hunks: [] }));
		const big = render(HunkPane, {});
		expect(big.text()).toContain('too large');
		big.destroy();

		control.setFile(diff({ hunks: [] }));
		const mode = render(HunkPane, {});
		expect(mode.text()).toContain("only the file's mode changed");
		mode.destroy();
	});

	it('shows a file error instead of hunks', () => {
		control.setSelection({ path: 'core.txt', side: 'unstaged' });
		control.setFileError('core.txt changed since it was read; reload and try again');
		const view = render(HunkPane, {});

		expect(view.text()).toContain('changed since it was read');
		view.destroy();
	});
});

describe('MessageBox', () => {
	it('writes the subject and the body into the store', () => {
		const view = render(MessageBox, {});

		const subject = view.get('.subject') as HTMLInputElement;
		subject.value = 'A subject';
		subject.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();

		const body = view.get('.body') as HTMLTextAreaElement;
		body.value = 'A body.';
		body.dispatchEvent(new Event('input', { bubbles: true }));
		flushSync();

		expect(view.get('.subject').getAttribute('aria-label')).toBe('Commit subject');
		expect(body.getAttribute('aria-label')).toBe('Commit body');
	});

	it('gives the body a height WebKit will honour, so it cannot eat the hunks', () => {
		const view = render(MessageBox, {});
		expect(Number((view.get('.body') as HTMLTextAreaElement).rows)).toBe(3);
		view.destroy();

		const source = readFileSync('src/lib/changes/MessageBox.svelte', 'utf8');
		expect(source).toMatch(/\.message \{[^}]*max-height: 40%/);
	});

	it('shows a character count only once the subject is long', () => {
		const short = render(MessageBox, {});
		expect(short.find('.count')).toBeNull();
		short.destroy();

		control.setSubject('x'.repeat(60));
		const long = render(MessageBox, {});
		expect(long.get('.count').textContent?.trim()).toBe('60');
		long.destroy();
	});

	/**
	 * FEAT-019. The item asked that a repository which cannot sign be told so
	 * *at* the point of commit rather than by a failure afterwards, and this is
	 * where that promise is kept or broken.
	 */
	function signing(overrides: Partial<Signing> = {}): Signing {
		return {
			enabled: true,
			origin: 'global',
			format: 'openPgp',
			key: null,
			program: 'gpg',
			problem: null,
			repository: true,
			global: true,
			local: null,
			...overrides
		};
	}

	it('says nothing about signing when signing is off', () => {
		// The ordinary case. A note on every commit in a repository that never
		// signs is noise on every commit, and the absence already says it.
		control.setSigning(signing({ enabled: false, problem: null }));
		const view = render(MessageBox, {});

		expect(view.find('.signing')).toBeNull();
		view.destroy();
	});

	it('says nothing before signing has been read', () => {
		const view = render(MessageBox, {});
		expect(view.find('.signing')).toBeNull();
		view.destroy();
	});

	it('says the commit will be signed, and names the program', () => {
		control.setSigning(signing({ program: 'gpg2' }));
		const view = render(MessageBox, {});

		const note = view.get('.signing');
		expect(note.textContent).toContain('will be signed');
		expect(note.textContent).toContain('gpg2');
		// A statement, not an alarm: signing being on is ordinary for anyone
		// who signs.
		expect(note.classList.contains('warn')).toBe(false);

		view.destroy();
	});

	it('warns before the button when the signing program is not installed', () => {
		control.setSigning(
			signing({ problem: { kind: 'missingProgram', detail: 'spagitty-no-such-signer' } })
		);
		const view = render(MessageBox, {});

		const note = view.get('.signing');
		expect(note.textContent).toContain('spagitty-no-such-signer');
		expect(note.classList.contains('warn')).toBe(true);

		view.destroy();
	});

	it('warns when ssh signing has no key, which cannot work', () => {
		control.setSigning(signing({ format: 'ssh', problem: { kind: 'noSigningKey' } }));
		const view = render(MessageBox, {});

		const note = view.get('.signing');
		expect(note.textContent).toContain('user.signingkey');
		expect(note.classList.contains('warn')).toBe(true);

		view.destroy();
	});

	it('toggles amending and says what it does', () => {
		const view = render(MessageBox, {});

		click(view.get('.chip'));
		expect(calls.amends).toEqual([true]);

		view.destroy();

		control.setAmend(true);
		const amending = render(MessageBox, {});
		expect(amending.text()).toContain('rewrites the last commit');
		amending.destroy();
	});
});
