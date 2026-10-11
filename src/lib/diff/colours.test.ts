// SPDX-License-Identifier: GPL-3.0-or-later

// An addition is green and a removal red, whatever is drawn on top (BUG-069).
// These read the stylesheets, because a colour rule is the thing under test.

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync('src/app.css', 'utf8');
const room = readFileSync('src/lib/review/RoomDiff.svelte', 'utf8');

/** The declarations of the first rule whose selector is exactly `selector`. */
function rule(source: string, selector: string): string {
	const start = source.indexOf(selector + ' {');
	expect(start, selector).toBeGreaterThanOrEqual(0);
	return source.slice(start, source.indexOf('}', start));
}

describe('the colours of a diff', () => {
	it('lays the ruler and the comment range over the line, never in place of it', () => {
		for (const selector of ['.line.focused', '.line.covered']) {
			const body = rule(room, selector);
			expect(body, selector).not.toMatch(/background(-color)?:/);
			expect(body, selector).toContain('background-image');
		}
	});

	it('paints an added line green and a removed line red from the line itself', () => {
		expect(rule(room, '.line.added')).toContain('--diff-add-bg');
		expect(rule(room, '.line.removed')).toContain('--diff-del-bg');
		expect(css).toMatch(/--diff-add-bg: color-mix\(in srgb, var\(--ok\)/);
		expect(css).toMatch(/--diff-del-bg: color-mix\(in srgb, var\(--danger\)/);
	});

	it('does not tint the comment range, or the comment button, with the accent', () => {
		expect(css).toMatch(/--covered: color-mix\(in srgb, var\(--ink\)/);
		expect(rule(room, '.plus')).not.toContain('--accent');
	});

	it('gives no syntax colour the red of a removal', () => {
		const tokens = css.slice(css.indexOf('.tok-keyword'));
		expect(tokens).not.toContain('--lane-1');
		expect(tokens).not.toContain('--danger');
		expect(tokens).not.toContain('#c23b22');
	});

	it('makes an addition strong enough to be read as green', () => {
		const share = Number(/--diff-add-bg: color-mix\(in srgb, var\(--ok\) (\d+)%/.exec(css)?.[1]);
		expect(share).toBeGreaterThanOrEqual(14);
	});
});
