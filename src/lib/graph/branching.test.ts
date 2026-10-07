// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { branchName, branching } from './branching.svelte';

describe('branching', () => {
	it('turns spaces into dashes and trims the ends', () => {
		expect(branchName('  my new  feature ')).toBe('my-new-feature');
		expect(branchName('feature/a')).toBe('feature/a');
		expect(branchName('   ')).toBe('');
	});

	it('holds one commit at a time and lets it go', () => {
		branching.start('abc');
		expect(branching.at).toBe('abc');
		branching.start('def');
		expect(branching.at).toBe('def');
		branching.cancel();
		expect(branching.at).toBeNull();
	});
});
