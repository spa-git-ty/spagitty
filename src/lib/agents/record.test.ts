// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { KIND_LABELS, blocksOf, lineCount } from './record';
import { aStep } from '../../testing/agent-fixtures';
import type { StepEvent } from './types';

describe('a step’s record, ready to draw (FEAT-118)', () => {
	it('keeps each kind as itself and joins adjacent reply lines into one reply', () => {
		const record: StepEvent[] = [
			{ kind: 'reply', text: 'First line.' },
			{ kind: 'reply', text: 'Second line.' },
			{ kind: 'read', text: 'a.rs', path: 'a.rs' },
			{ kind: 'reply', text: 'After the read.' },
			{ kind: 'command', text: 'cargo test' }
		];
		expect(blocksOf(aStep({ record }))).toEqual([
			{ kind: 'reply', text: 'First line.\nSecond line.', path: undefined },
			{ kind: 'read', text: 'a.rs', path: 'a.rs' },
			{ kind: 'reply', text: 'After the read.', path: undefined },
			{ kind: 'command', text: 'cargo test', path: undefined }
		]);
	});

	it('is empty for a step recorded before kinds were kept', () => {
		expect(blocksOf({ record: undefined as unknown as StepEvent[] })).toEqual([]);
	});

	it('names every kind once', () => {
		expect(Object.keys(KIND_LABELS).sort()).toEqual(['command', 'error', 'read', 'reply', 'result', 'sent', 'tool']);
	});

	it('counts the lines of a prompt', () => {
		expect(lineCount('')).toBe(0);
		expect(lineCount('one')).toBe(1);
		expect(lineCount('one\ntwo\nthree')).toBe(3);
	});
});
