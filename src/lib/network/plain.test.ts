// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * git's last line about a fetch or a push, in words (BUG-091).
 */

import { describe, expect, it } from 'vitest';
import { plainSummary } from './plain';

describe('plainSummary', () => {
	it('names the branch a push moved, not the two hashes', () => {
		expect(plainSummary('push', '   75b242b47..880b35657  chore/fix-sonar-scan -> chore/fix-sonar-scan')).toBe(
			'Pushed chore/fix-sonar-scan'
		);
	});

	it('names the remote branch a fetch moved', () => {
		expect(plainSummary('fetch', '   75b242b47..880b35657  main       -> origin/main')).toBe(
			'Fetched: origin/main moved on'
		);
	});

	it('says new, forced, deleted and refused in words', () => {
		expect(plainSummary('push', ' * [new branch]      feat/x -> feat/x')).toBe('Pushed new branch feat/x');
		expect(plainSummary('fetch', ' * [new tag]         v3.2.0     -> v3.2.0')).toBe('Fetched new tag v3.2.0');
		expect(plainSummary('push', ' + 1a2b3c4...5d6e7f8 main -> main (forced update)')).toBe(
			'Pushed main, replacing what was there'
		);
		expect(plainSummary('fetch', ' - [deleted]         (none)     -> origin/old')).toBe('Deleted origin/old');
		expect(plainSummary('push', ' ! [rejected]        main -> main (fetch first)')).toBe(
			'main was refused: fetch first'
		);
	});

	it('says up to date in words', () => {
		expect(plainSummary('push', 'Everything up-to-date')).toBe('Already up to date');
		expect(plainSummary('fetch', ' = [up to date]      main -> origin/main')).toBe('Already up to date');
	});

	it('passes on what it does not recognise, and nothing as nothing', () => {
		expect(plainSummary('fetch', 'From github.com:spa-git-ty/spagitty')).toBe('From github.com:spa-git-ty/spagitty');
		expect(plainSummary('push', null)).toBeNull();
		expect(plainSummary('push', '   ')).toBeNull();
	});
});
