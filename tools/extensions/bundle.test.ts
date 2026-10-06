// SPDX-License-Identifier: GPL-3.0-or-later
import { expect, it } from 'vitest';
import { targetFor } from './bundle';

it('selects only release targets whose native worker is declared', () => {
	expect(targetFor('win32', 'x64')).toBe('x86_64-pc-windows-msvc');
	expect(targetFor('linux', 'x64')).toBe('x86_64-unknown-linux-gnu');
	expect(targetFor('darwin', 'x64')).toBe('x86_64-apple-darwin');
	expect(targetFor('darwin', 'arm64')).toBe('aarch64-apple-darwin');
	expect(() => targetFor('win32', 'arm64')).toThrow();
	expect(() => targetFor('linux', 'arm64')).toThrow();
});
