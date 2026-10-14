// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Lives for Investigate's tests (FEAT-132): a file that was born, renamed,
 * moved and rewritten; a folder; a branch; a line.
 */

import type { BranchFacts, Hunk, LifeEntry, LifeKind, LifeToday } from '$lib/types';

export const DAY = 86_400;
/** 2026-01-01T00:00:00Z. Entries count days back from here. */
export const NOW = 1_767_225_600;

export const sha = (n: number) => n.toString(16).padStart(40, 'c');

const PEOPLE: Record<string, [string, string]> = {
	M: ['Mahmoud', 'mahmoud@example.com'],
	O: ['Omar', 'omar@example.com'],
	S: ['Sara', 'sara@example.com']
};

/** One entry, `daysAgo` before [`NOW`]. `who` is M, O, S, or C for the agent. */
export function entry(
	n: number,
	daysAgo: number,
	who: string,
	kind: LifeKind,
	summary: string,
	added = 0,
	removed = 0,
	extra: Partial<LifeEntry> = {}
): LifeEntry {
	const [authorName, authorEmail] = PEOPLE[who === 'C' ? 'M' : who];
	return {
		commit: sha(n),
		short: sha(n).slice(-7),
		authorName,
		authorEmail,
		time: NOW - daysAgo * DAY,
		summary,
		agent: who === 'C' ? 'Codex' : null,
		kind,
		added,
		removed,
		path: 'core/avatars.rs',
		oldPath: null,
		size: null,
		share: null,
		filesAdded: 0,
		filesRemoved: 0,
		filesRenamed: 0,
		text: null,
		note: null,
		...extra
	};
}

/** Newest first: an agent's edit, a rewrite, a move, a rename, an edit, the birth. */
export const FILE: LifeEntry[] = [
	entry(6, 2, 'C', 'edit', 'Cache avatars on disk', 9, 4, { size: 21, share: 0.25 }),
	entry(5, 40, 'O', 'rewrite', 'Expire avatars daily', 31, 52, { size: 16, share: 0.81 }),
	entry(4, 80, 'M', 'moved', 'Move avatars into core', 0, 0, {
		size: 37,
		share: 0,
		oldPath: 'src/avatars.rs',
		note: 'src → core'
	}),
	entry(3, 120, 'M', 'renamed', 'Call it what it holds', 0, 0, {
		size: 37,
		share: 0,
		path: 'src/avatars.rs',
		oldPath: 'src/avatar.rs',
		note: 'avatar.rs → avatars.rs'
	}),
	entry(2, 160, 'S', 'edit', 'Key avatars by size', 19, 0, { size: 37, share: 0, path: 'src/avatar.rs' }),
	entry(1, 200, 'M', 'born', 'Add avatar cache', 18, 0, { size: 18, path: 'src/avatar.rs' })
];

export const TODAY: LifeToday = {
	lines: 21,
	owners: [
		{ name: 'Mahmoud', email: 'mahmoud@example.com', agent: false, lines: 9, share: 0.43 },
		{ name: 'Omar', email: 'omar@example.com', agent: false, lines: 8, share: 0.38 },
		{ name: 'Codex', email: 'mahmoud@example.com', agent: true, lines: 4, share: 0.19 }
	],
	alive: { [sha(5)]: 5, [sha(6)]: 4, [sha(1)]: 6 }
};

export const HUNK: Hunk = {
	oldStart: 4,
	oldLines: 3,
	newStart: 4,
	newLines: 4,
	header: '@@ -4,3 +4,4 @@ pub fn avatar',
	lines: [
		{ origin: 'context', old: 4, new: 4, text: '    let key = cache_key(email, size);' },
		{ origin: 'removed', old: 5, new: null, text: '        return Ok(hit.clone());' },
		{ origin: 'added', old: null, new: 5, text: '        if hit.fetched.elapsed() < MAX_AGE {' },
		{ origin: 'added', old: null, new: 6, text: '            return Ok(hit.avatar.clone());' }
	]
};

const bare = { path: null };

/** A folder: born, thirty edits of growing size, a rename, a removal, an addition. */
export const FOLDER: LifeEntry[] = [
	entry(40, 2, 'S', 'added', 'Add the review state', 310, 0, { ...bare, filesAdded: 3 }),
	entry(39, 20, 'O', 'removed', 'Drop the old walker', 0, 420, { ...bare, filesRemoved: 2 }),
	entry(38, 30, 'M', 'renamed', 'Rename the shell boundary', 12, 12, { ...bare, filesRenamed: 1 }),
	...Array.from({ length: 30 }, (_, i) =>
		entry(100 + i, 40 + i, 'MOS'[i % 3], 'edit', `Edit ${i + 1}`, i + 1, 0, bare)
	),
	entry(37, 200, 'M', 'born', 'Start the core crate', 900, 0, { ...bare, filesAdded: 12 })
];

/** A branch that was merged into, rebased, and landed. */
export const BRANCH: LifeEntry[] = [
	entry(60, 1, 'M', 'merged', 'Merge pull request #71', 0, 0, { ...bare, note: 'into main' }),
	entry(59, 2, 'C', 'commit', 'Cache avatars on disk', 40, 12, bare),
	entry(58, 3, 'M', 'rebase', 'rebase (finish): refs/heads/feature/avatars onto 4b1e0aa', 0, 0, {
		...bare,
		note: 'a1b2c3d → 9f8e7d6'
	}),
	entry(57, 4, 'M', 'merge', 'Merge main into feature/avatars', 0, 0, bare),
	entry(56, 6, 'O', 'commit', 'Expire avatars daily', 31, 52, bare),
	entry(54, 10, 'M', 'born', 'Release 2.2.2', 0, 0, { ...bare, note: 'from main' })
];

export const BRANCH_FACTS: BranchFacts = {
	against: 'main',
	fork: sha(54),
	landed: sha(60),
	ahead: 0,
	behind: 3,
	local: true
};

/** A line: written, changed twice. */
export const LINE: LifeEntry[] = [
	entry(6, 2, 'C', 'edit', 'Cache avatars on disk', 1, 1, { text: '    fs::write(&path, &bytes)?;' }),
	entry(5, 40, 'O', 'edit', 'Expire avatars daily', 1, 1, { text: '    MEMORY.lock().insert(key, avatar);' }),
	entry(1, 200, 'M', 'born', 'Add avatar cache', 1, 0, { text: '    MEMORY.insert(key, bytes);', path: 'src/avatar.rs' })
];
