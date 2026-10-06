// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { click, render } from '../testing/mount';
import { codeStack, DEFAULTS, normalise, reading } from './reading.svelte';
import { scale } from './scale.svelte';
import ReadingSection from './settings/ReadingSection.svelte';

const root = () => document.documentElement;
const token = (name: string) => root().style.getPropertyValue(name);

beforeEach(() => {
	localStorage.clear();
	scale.init();
	reading.init();
});

afterEach(() => {
	reading.reset();
	localStorage.clear();
});

describe('normalise', () => {
	it('reads nothing as the defaults', () => {
		expect(normalise(null)).toEqual(DEFAULTS);
		expect(normalise('nonsense')).toEqual(DEFAULTS);
	});

	it('keeps each good field and replaces each bad one on its own', () => {
		const read = normalise({
			codeFont: 'jetbrains-mono',
			uiFont: 'comic-sans',
			size: 99,
			lineHeight: 1.6,
			letterSpacing: -1,
			ruler: 'chunk',
			colours: 'neon',
			words: 'yes'
		});
		expect(read).toEqual({
			...DEFAULTS,
			codeFont: 'jetbrains-mono',
			size: 20,
			lineHeight: 1.6,
			letterSpacing: 0,
			ruler: 'chunk'
		});
	});
});

describe('reading', () => {
	it('publishes the defaults as tokens on the first read', () => {
		expect(token('--code-font')).toBe(codeStack('atkinson-mono'));
		expect(token('--code-lh')).toBe('1.85');
		expect(token('--code-ls')).toBe('0.02em');
		expect(token('--fs-code')).toBe('15px');
		expect(root().dataset.diff).toBe('calm');
		expect(root().dataset.words).toBe('on');
		// The interface keeps the desktop's face until one is chosen.
		expect(token('--font-ui')).toBe('');
	});

	it('changes every reading token at once, and keeps the choice', () => {
		reading.set({
			codeFont: 'opendyslexic-mono',
			uiFont: 'lexend',
			size: 17,
			lineHeight: 2,
			letterSpacing: 0.05,
			colours: 'classic',
			words: false
		});
		expect(token('--code-font')).toContain('OpenDyslexic Mono');
		expect(token('--font-ui')).toContain('Lexend');
		expect(token('--fs-code')).toBe('17px');
		expect(token('--code-lh')).toBe('2');
		expect(token('--code-ls')).toBe('0.05em');
		expect(root().dataset.diff).toBe('classic');
		expect(root().dataset.words).toBe('off');

		// A fresh start reads it back.
		reading.reset();
		localStorage.setItem(
			'spagitty.reading',
			JSON.stringify({ ...DEFAULTS, codeFont: 'jetbrains-mono', size: 13 })
		);
		reading.init();
		expect(reading.current.codeFont).toBe('jetbrains-mono');
		expect(token('--fs-code')).toBe('13px');
	});

	it('scales code with the zoom it composes with', () => {
		reading.set({ size: 16 });
		scale.setZoom(1.5);
		expect(token('--fs-code')).toBe('24px');
		scale.setZoom(1);
	});

	it('gives the interface back to the desktop when System is chosen again', () => {
		reading.set({ uiFont: 'atkinson' });
		expect(token('--font-ui')).toContain('Atkinson Hyperlegible');
		reading.set({ uiFont: 'system' });
		expect(token('--font-ui')).toBe('');
	});
});

describe('ReadingSection', () => {
	const chip = (view: ReturnType<typeof render>, label: string) =>
		view.all('button.chip').find((button) => button.textContent?.trim() === label)!;

	it('offers every face, ruler and colour, each chip in its own face', () => {
		const view = render(ReadingSection, {});
		const labels = view.all('button.chip').map((button) => button.textContent?.trim());
		expect(labels).toEqual([
			'Atkinson Hyperlegible Mono',
			'OpenDyslexic Mono',
			'Lexend',
			'JetBrains Mono',
			'System monospace',
			'System',
			'Atkinson Hyperlegible',
			'Lexend',
			'Off',
			'One line',
			'Whole chunk',
			'Calm',
			'Classic',
			'Highlight changed words'
		]);
		expect(view.text()).toContain('b d p q and I l 1 all have different shapes');
		expect(view.text()).toContain('Move it with j and k.');
		view.destroy();
	});

	it('changes the preferences from its chips and sliders', () => {
		const view = render(ReadingSection, {});

		click(chip(view, 'JetBrains Mono'));
		expect(reading.current.codeFont).toBe('jetbrains-mono');
		expect(view.text()).toContain('A common coding font.');

		click(chip(view, 'Whole chunk'));
		expect(reading.current.ruler).toBe('chunk');
		expect(view.all('.line.faded').length).toBeGreaterThan(0);

		click(chip(view, 'Classic'));
		expect(reading.current.colours).toBe('classic');
		click(chip(view, 'Highlight changed words'));
		expect(reading.current.words).toBe(false);

		const size = view.all('input[type="range"]')[0] as HTMLInputElement;
		size.value = '18';
		size.dispatchEvent(new Event('input', { bubbles: true }));
		expect(reading.current.size).toBe(18);
		view.destroy();
	});

	it('marks the changed words in its preview', () => {
		const view = render(ReadingSection, {});
		expect(view.all('.line .word').length).toBeGreaterThan(0);
		view.destroy();
	});
});
