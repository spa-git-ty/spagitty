// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, fire, press, render, type Mounted } from '../../testing/mount';
import { openRepository, request } from '../../testing/git-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from '$lib/api';
import { requests } from '$lib/requests/store.svelte';
import { review } from '$lib/review/store.svelte';
import type { DiffLine, FileChange, FullFile, PullRequestComment } from '$lib/types';
import Page from './+page.svelte';

let view: Mounted;
const button = (name: string) =>
	view.all('button').find((b) => b.textContent?.replace(/\s+/g, ' ').trim() === name)!;
const pill = () => view.get('[aria-label="Review controls"]');
const files = () => view.get('aside[aria-label="Touched files"]');
const conversation = () => view.get('aside[aria-label="Conversation"]');
const focused = () => view.find('.line.focused')?.textContent?.replace(/\s+/g, ' ').trim();

const PR = request({
	id: 'PR_214',
	number: 214,
	title: 'Cache avatars on disk instead of in memory',
	authorName: 'yasser-dev',
	reviewRequested: true,
	headSha: 'h1'
});

const AVATARS = 'crates/core/src/avatars.rs';
const TYPES = 'src/lib/types.ts';

/** 29 lines, the 15th edited: 30 rows with the removed one in place. */
function edited(): DiffLine[] {
	const lines: DiffLine[] = [];
	for (let n = 1; n <= 29; n++) {
		if (n === 15) {
			lines.push({ origin: 'removed', old: 15, new: null, text: 'MEMORY.lock().get(&key)' });
			lines.push({ origin: 'added', old: null, new: 15, text: 'path.exists() && !is_stale(&path)' });
		} else {
			lines.push({ origin: 'context', old: n, new: n, text: n === 1 ? 'pub fn avatar() {' : `\tline ${n}` });
		}
	}
	return lines;
}

const LISTED: FileChange[] = [
	{ path: AVATARS, status: 'modified', binary: false, tooLarge: false, added: 1, removed: 1, oldBlob: 'o1', newBlob: 'n1' },
	{ path: TYPES, status: 'modified', binary: false, tooLarge: false, added: 1, removed: 0, oldBlob: 'o2', newBlob: 'n2' }
];

function whole(path: string): FullFile {
	const lines =
		path === AVATARS
			? edited()
			: [
					{ origin: 'context', old: 1, new: 1, text: 'export interface Author {' },
					{ origin: 'added', old: null, new: 2, text: '\tinitials: string;' },
					{ origin: 'context', old: 2, new: 3, text: '}' }
				];
	const listed = LISTED.find((file) => file.path === path)!;
	return {
		path,
		oldPath: null,
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: listed.added,
		removed: listed.removed,
		oldBlob: listed.oldBlob ?? null,
		newBlob: listed.newBlob ?? null,
		lines: lines as DiffLine[]
	};
}

function comment(id: number, extra: Partial<PullRequestComment>): PullRequestComment {
	return {
		id,
		inReplyTo: null,
		path: AVATARS,
		line: 15,
		side: 'RIGHT',
		body: `comment ${id}`,
		author: 'nour.h',
		createdAt: 1_787_650_000 + id,
		resolved: false,
		...extra
	};
}

const COMMENTS = [
	comment(1, { body: 'Write to a temp file, then rename?' }),
	comment(2, { inReplyTo: 1, author: 'yasser-dev', body: 'Good catch.' }),
	comment(3, { line: 2, body: 'Is anything else still reading memo?', resolved: true })
];

beforeEach(() => {
	vi.clearAllMocks();
	requests.clear();
	review.clear();
	control.reset();
	openRepository();
	localStorage.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.forgeRepo).mockResolvedValue({
		kind: 'gitHub',
		host: 'github.com',
		owner: 'spa-git-ty',
		name: 'spagitty'
	});
	vi.mocked(api.forgeAccounts).mockResolvedValue([{ kind: 'gitHub', host: 'github.com', user: 'mahmoud' }]);
	vi.mocked(api.pullRequests).mockResolvedValue([PR]);
	vi.mocked(api.reviewState).mockResolvedValue(null);
	vi.mocked(api.setReviewState).mockResolvedValue(undefined);
	vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h1', base: 'b1', mergeBase: 'm1' });
	vi.mocked(api.reviewFiles).mockResolvedValue(LISTED);
	vi.mocked(api.reviewFile).mockImplementation(async (_from, _to, path) => whole(path));
	vi.mocked(api.pullRequestComments).mockResolvedValue(COMMENTS);
});

afterEach(() => {
	view?.destroy();
	requests.clear();
	review.clear();
	control.reset();
});

async function openRoom() {
	view = render(Page, {});
	await vi.waitFor(() => expect(view.all('button.row')).toHaveLength(1));
	await review.open(PR);
	await vi.waitFor(() => expect(view.find('[aria-label="Review controls"]')).not.toBeNull());
	await vi.waitFor(() => expect(view.text()).not.toContain('Reading…'));
}

it('fetches the pull request and lands on the first file not yet viewed', async () => {
	vi.mocked(api.reviewState).mockResolvedValue({
		headSha: 'h0',
		viewed: { [AVATARS]: 'n1', [TYPES]: 'an older blob' },
		files: 2
	});
	await openRoom();

	expect(api.reviewCheckout).toHaveBeenCalledWith(214, 'main', 'h1');
	expect(api.reviewFiles).toHaveBeenCalledWith('m1', 'h1');
	expect(pill().textContent).toContain('types.ts');
	expect(pill().textContent).toContain('2 of 2');
	expect(view.text()).toContain('1 of 2 viewed');
	// The tick on a file that changed since is dropped, and the head kept.
	const saved = vi.mocked(api.setReviewState).mock.calls.at(-1)![1] as Record<string, unknown>;
	expect(saved.viewed).toEqual({ [AVATARS]: 'n1' });
	expect(saved.headSha).toBe('h1');
	expect(saved.files).toBe(2);
	expect((files().querySelector(`input[aria-label="Viewed types.ts"]`) as HTMLInputElement).checked).toBe(false);
});

it('shows the changed part with its folds, and opens a fold on asking', async () => {
	await openRoom();

	expect(view.text()).toContain('@@ -12,7 +12,7 @@ pub fn avatar() {');
	expect(view.all('button.fold').map((b) => b.textContent?.trim())).toEqual([
		'11 unchanged lines',
		'11 unchanged lines'
	]);
	expect(view.text()).not.toContain('line 2 ');

	click(view.all('button.fold')[0]);
	expect(view.all('button.fold')).toHaveLength(1);
	expect(view.text()).toContain('unchanged · 1–11');
	expect(view.text()).toContain('line 2');
});

it('marks only the words that changed', async () => {
	await openRoom();
	const words = view.all('.line .word').map((span) => span.textContent);
	expect(words.length).toBeGreaterThan(0);
	expect(words.join(' ')).not.toContain('line');
});

it('shows the whole file with nothing folded', async () => {
	await openRoom();
	click(button('Whole file'));
	expect(view.all('button.fold')).toHaveLength(0);
	expect(view.text()).toContain('line 29');
	click(button('Changes'));
	expect(view.all('button.fold')).toHaveLength(2);
});

it('ticks a file and goes on to the next one not viewed', async () => {
	await openRoom();
	click(button('Viewed, next'));
	await vi.waitFor(() => expect(pill().textContent).toContain('types.ts'));

	const saved = vi.mocked(api.setReviewState).mock.calls.at(-1)![1] as Record<string, unknown>;
	expect(saved.viewed).toEqual({ [AVATARS]: 'n1' });
	expect(view.text()).toContain('1 of 2 viewed');
	await vi.waitFor(() => expect(view.text()).toContain('initials: string;'));

	fire(files().querySelector('input[aria-label="Viewed types.ts"]')!, 'change');
	await vi.waitFor(() => expect(view.text()).toContain('2 of 2 viewed'));
});

it('shows every file in one column on All', async () => {
	await openRoom();
	click(button('All'));
	await vi.waitFor(() => expect(view.text()).toContain('initials: string;'));
	expect(api.reviewFile).toHaveBeenCalledWith('m1', 'h1', TYPES, null);
	expect(pill().textContent).toContain('All files');
	expect(view.all('.file .path').map((p) => p.textContent)).toEqual([AVATARS, TYPES]);
});

it('draws threads under their lines and lists them, and goes to one on a click', async () => {
	await openRoom();
	expect(view.text()).toContain('Write to a temp file, then rename?');
	expect(view.text()).toContain('Good catch.');
	expect(conversation().textContent).toContain('Open 1');
	expect(conversation().textContent).toContain('Resolved 1');
	expect(conversation().textContent).toContain('avatars.rs:15');
	expect(conversation().textContent).toContain('1 reply');
	expect(files().textContent).toContain('1 thread');

	// The resolved one is on a folded line: going to it opens the fold.
	click(button('Resolved 1'));
	expect(view.text()).not.toContain('Is anything else still reading memo? ·');
	click(conversation().querySelector('button.item')!);
	await vi.waitFor(() => expect(view.text()).toContain('unchanged · 1–11'));
	expect(view.all('.part .thread .body').map((b) => b.textContent?.trim())).toContain(
		'Is anything else still reading memo?'
	);
	expect(focused()).toContain('line 2');
});

it('starts the ruler on the first change and moves it with j and k', async () => {
	await openRoom();
	await vi.waitFor(() => expect(focused()).toContain('MEMORY.lock()'));
	press(window, 'j');
	expect(focused()).toContain('path.exists()');
	press(window, 'k');
	press(window, 'k');
	expect(focused()).toContain('line 14');

	click(view.get('button[aria-label="Focus ruler"]'));
	expect(focused()).toBeUndefined();
});

it('sets code as it was before the reading set on Aa', async () => {
	await openRoom();
	const screen = () => view.get('.screen').getAttribute('style') ?? '';
	expect(screen()).not.toContain('--code-lh');
	click(button('Aa'));
	expect(screen()).toContain('--code-lh: 1.45');
	expect(button('Aa').getAttribute('aria-pressed')).toBe('false');
});

it('reads the host’s patch when the pull request cannot be fetched', async () => {
	vi.mocked(api.reviewCheckout).mockRejectedValue('no remote to fetch the pull request from');
	vi.mocked(api.pullRequestFiles).mockResolvedValue([
		{
			path: AVATARS,
			status: 'modified',
			binary: false,
			tooLarge: false,
			added: 1,
			removed: 0,
			hunks: [
				{
					oldStart: 4,
					oldLines: 1,
					newStart: 4,
					newLines: 2,
					header: '@@ -4,1 +4,2 @@',
					lines: [
						{ origin: 'context', old: 4, new: 4, text: 'fn a() {}' },
						{ origin: 'added', old: null, new: 5, text: 'fn b() {}' }
					]
				}
			]
		}
	]);
	await openRoom();

	expect(view.text()).toContain("From GitHub's patch: the pull request could not be fetched");
	expect(view.text()).toContain('fn b() {}');
	expect(button('Whole file').hasAttribute('disabled')).toBe(true);
	expect(api.reviewFile).not.toHaveBeenCalled();
});

it('says why when neither the head nor the patch can be read', async () => {
	vi.mocked(api.reviewCheckout).mockRejectedValue('no remote to fetch the pull request from');
	vi.mocked(api.pullRequestFiles).mockRejectedValue('offline');
	view = render(Page, {});
	await vi.waitFor(() => expect(view.all('button.row')).toHaveLength(1));
	await review.open(PR);
	await vi.waitFor(() => expect(view.text()).toContain('no remote to fetch the pull request from'));
	expect(view.find('[aria-label="Review controls"]')).toBeNull();
});
