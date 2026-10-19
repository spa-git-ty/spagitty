// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The check gate 2 runs on the workspace's `rust-version` (TASK-072).
 */

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { aheadOf, declaredIn, newer } from './rust-version.mjs';

const metadata = {
	workspace_members: ['path+file:///w/core#spagitty-core@3.3.0'],
	packages: [
		{ id: 'path+file:///w/core#spagitty-core@3.3.0', name: 'spagitty-core', version: '3.3.0', rust_version: '1.99' },
		{ id: 'tree-sitter', name: 'tree-sitter', version: '0.27.1', rust_version: '1.90' },
		{ id: 'notify-rust', name: 'notify-rust', version: '4.18.2', rust_version: '1.89.0' },
		{ id: 'old', name: 'old', version: '1.0.0', rust_version: null }
	]
};

describe('rust-version', () => {
	it('compares versions of either length', () => {
		expect(newer('1.90', '1.89.0')).toBe(true);
		expect(newer('1.90.0', '1.90')).toBe(false);
		expect(newer('1.77', '1.90')).toBe(false);
		expect(newer('1.100', '1.99')).toBe(true);
	});

	it('names a dependency that needs a newer Rust than the workspace says', () => {
		expect(aheadOf(metadata, '1.77')).toEqual(['notify-rust 4.18.2 needs 1.89.0', 'tree-sitter 0.27.1 needs 1.90']);
		expect(aheadOf(metadata, '1.90')).toEqual([]);
	});

	it('leaves the crates of the workspace itself out', () => {
		expect(aheadOf(metadata, '1.90').some((line) => line.startsWith('spagitty-core'))).toBe(false);
	});

	it('reads the line the workspace declares', () => {
		expect(declaredIn('[workspace.package]\nedition = "2021"\nrust-version = "1.90"\n')).toBe('1.90');
		expect(declaredIn(readFileSync('Cargo.toml', 'utf8'))).toBe('1.90');
	});
});
