// SPDX-License-Identifier: GPL-3.0-or-later
import type { PullRequest, Tag, Reflog, ReflogEntry, CommitDiff, FileChange, FileDiff, StashEntry, ConflictRegion, ConflictFile, ConflictSide, ConflictSides, ConflictState } from '$lib/types';
import { control as repoControl } from './repo-store.svelte';
export function tag(name: string, overrides: Partial<Tag> = {}): Tag {
	return {
		name,
		target: 'a'.repeat(40),
		targetShort: 'aaaaaaa',
		annotated: true,
		message: `${name} released`,
		taggerName: 'Ada Lovelace',
		time: 1_800_000_000,
		summary: 'The tagged commit',
		...overrides
	};
}

export function openRepository() {
	repoControl.setInfo({
		path: '/repos/fixture',
		name: 'fixture',
		bare: false,
		head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' },
		lastFetched: null
	});
}


export function entry(index: number, overrides: Partial<ReflogEntry> = {}): ReflogEntry {
	return {
		index,
		revision: `HEAD@{${index}}`,
		before: 'b'.repeat(40),
		beforeShort: 'bbbbbbb',
		after: 'a'.repeat(40),
		afterShort: 'aaaaaaa',
		created: false,
		authorName: 'Ada Lovelace',
		time: 1_800_000_000 - index,
		message: `commit: change ${index}`,
		operation: 'commit',
		...overrides
	};
}

export function log(overrides: Partial<Reflog> = {}): Reflog {
	return {
		reference: 'HEAD',
		entries: [entry(0), entry(1)],
		truncated: false,
		exists: true,
		...overrides
	};
}


export function fileChange(path: string, added = 1, removed = 1): FileChange {
	return { path, status: 'modified', binary: false, tooLarge: false, added, removed };
}

export function commit(id: string, paths: string[]): CommitDiff {
	const files = paths.map((p) => fileChange(p));
	return {
		id,
		short: id.slice(0, 7),
		summary: `commit ${id}`,
		files,
		added: files.length,
		removed: files.length
	};
}

export function hunks(path: string): FileDiff {
	return {
		path,
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 1,
		removed: 1,
		hunks: [
			{
				oldStart: 1,
				oldLines: 1,
				newStart: 1,
				newLines: 1,
				header: '@@ -1 +1 @@',
				lines: [
					{ origin: 'removed', old: 1, new: null, text: `old ${path}` },
					{ origin: 'added', old: null, new: 1, text: `new ${path}` }
				]
			}
		]
	};
}


export function stashEntry(index: number, overrides: Partial<StashEntry> = {}): StashEntry {
	const id = `${index}`.padStart(40, 'a');
	return {
		index,
		name: `stash@{${index}}`,
		id,
		short: id.slice(0, 7),
		message: `On main: entry ${index}`,
		time: 1_700_000_000 - index * 60,
		authorName: 'Ada Lovelace',
		parent: 'b'.repeat(40),
		parentShort: 'bbbbbbb',
		parentSummary: 'Merge feature/split-view',
		...overrides
	};
}


export function region(overrides: Partial<ConflictRegion> = {}): ConflictRegion {
	return {
		index: 0,
		startLine: 2,
		endLine: 6,
		ours: 'OURS\n',
		base: null,
		theirs: 'THEIRS\n',
		...overrides
	};
}

export function file(path: string, kind: ConflictFile['kind'] = 'bothModified'): ConflictFile {
	return { path, kind };
}

export function side(text: string, overrides: Partial<ConflictSide> = {}): ConflictSide {
	return {
		text,
		lines: text === '' ? 0 : text.replace(/\n$/, '').split('\n').length,
		bytes: text.length,
		binary: false,
		tooLarge: false,
		...overrides
	};
}

export function sides(path: string, overrides: Partial<ConflictSides> = {}): ConflictSides {
	return {
		path,
		kind: 'bothModified',
		base: side('one\ntwo\nthree\n'),
		ours: side('one\nOURS\nthree\n'),
		theirs: side('one\nTHEIRS\nthree\n'),
		merged: side('one\n<<<<<<< HEAD\nOURS\n=======\nTHEIRS\n>>>>>>> theirs\nthree\n'),
		...overrides
	};
}

export function state(overrides: Partial<ConflictState> = {}): ConflictState {
	return { operation: 'merge', files: [file('shared.txt')], ...overrides };
}


export function request(overrides: Partial<PullRequest> = {}): PullRequest {
	return {
		id: 'PR_1',
		number: 412,
		title: 'Give the graph a footer',
		body: 'PR body markdown',
		authorName: 'grace',
		updated: 1_787_650_200,
		sourceBranch: 'feature/footer',
		targetBranch: 'main',
		draft: false,
		review: 'awaitingReview',
		checks: 'passing',
		needsYou: false,
		needsYouBecause: null,
		changedFiles: 7,
		added: 120,
		removed: 34,
		mergeable: true,
		...overrides
	};
}

