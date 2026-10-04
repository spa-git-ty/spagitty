// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import { request } from '../../testing/git-fixtures';
import { emptyRecord, keyFor, keyString, normalise, viewedCount } from './record';

const REPO = { kind: 'gitHub' as const, host: 'github.com', owner: 'team', name: 'app' };

describe('normalise', () => {
	it('reads nothing, or something that is not an object, as an empty record', () => {
		expect(normalise(null)).toEqual(emptyRecord());
		expect(normalise('a string')).toEqual(emptyRecord());
		expect(normalise([1, 2])).toEqual(emptyRecord());
	});

	it('keeps what a well-formed record carries', () => {
		const saved = {
			version: 1,
			headSha: 'abc',
			viewed: { 'a.rs': 'blob1' },
			drafts: [
				{
					id: 'd1',
					path: 'a.rs',
					line: 20,
					side: 'RIGHT',
					startLine: 18,
					startSide: 'RIGHT',
					body: 'Write to a temp file?',
					headSha: 'abc',
					createdAt: 5
				}
			],
			body: 'Looks close.',
			files: 6,
			conflictFiles: ['a.rs'],
			conflictMerges: ['7c1e9a0']
		};
		expect(normalise(saved)).toEqual(saved);
		expect(viewedCount(normalise(saved))).toBe(1);
	});

	it('costs a bad field only itself, and never the pending comments', () => {
		const record = normalise({
			viewed: 'not a map',
			files: -3,
			drafts: [
				{ path: 'a.rs', line: 3, body: 'keep me' },
				{ path: 'a.rs', line: 0, body: 'line zero is not a line' },
				{ path: 'a.rs', line: 4, body: '   ' },
				'not a comment'
			]
		});

		expect(record.viewed).toEqual({});
		expect(record.files).toBe(0);
		expect(record.drafts).toHaveLength(1);
		expect(record.drafts[0]).toMatchObject({ path: 'a.rs', line: 3, side: 'RIGHT', body: 'keep me' });
	});

	it('drops a range that does not end after it starts', () => {
		const [draft] = normalise({
			drafts: [{ path: 'a', line: 5, startLine: 9, body: 'x', side: 'LEFT' }]
		}).drafts;
		expect(draft.startLine).toBeNull();
		expect(draft.startSide).toBeNull();
		expect(draft.side).toBe('LEFT');
	});
});

describe('keyFor', () => {
	it('keys a row of the open repository by its forge and number', () => {
		expect(keyFor(REPO, request({ number: 214 }))).toEqual({
			host: 'github.com',
			owner: 'team',
			name: 'app',
			number: 214
		});
	});

	it('keys a row from a search by the repository it names, on the searched host', () => {
		const pr = request({ number: 7, repository: 'group/sub/project' });
		expect(keyFor(REPO, pr, 'gitlab.example.com')).toEqual({
			host: 'gitlab.example.com',
			owner: 'group/sub',
			name: 'project',
			number: 7
		});
		expect(keyString(keyFor(REPO, pr, 'gitlab.example.com')!)).toBe(
			'gitlab.example.com/group/sub/project#7'
		);
	});

	it('has no key with no forge to put it under', () => {
		expect(keyFor(null, request())).toBeNull();
		expect(keyFor(null, request({ repository: 'nameonly' }), 'github.com')).toBeNull();
	});
});
