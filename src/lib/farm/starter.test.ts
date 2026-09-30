// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it, vi } from 'vitest';
import { click, fire, press, render } from '../../testing/mount';
import Starter from './components/Starter.svelte';
import type { AgentStatus } from './types';

/**
 * The starter page (FEAT-073).
 *
 * What is worth asserting here is not the prose — that is in the component,
 * where it can be reworded without breaking a test — but the three promises the
 * screen makes: a farm can be started from this page without navigating
 * anywhere, a goal with no title cannot start one, and each readiness row tells
 * the truth about the machine it is describing.
 */

function agent(name: string): AgentStatus {
	return {
		definition: {
			id: name.toLowerCase(),
			provider: 'claudeCode',
			displayName: name,
			executable: `/usr/bin/${name.toLowerCase()}`,
			capabilities: ['coding'],
			inputMode: 'cliPrompt',
			traits: {
				resumableSessions: true,
				streaming: true,
				structuredOutput: true,
				toolUse: true,
				headless: true
			},
			role: 'general',
			extraArgs: [],
			enabled: true
		},
		availability: { state: 'available', path: `/usr/bin/${name.toLowerCase()}`, version: '1.0' }
	};
}

function props(overrides: Partial<Parameters<typeof Starter>[1]> = {}) {
	return {
		ready: [],
		undetected: [],
		policySources: [],
		verificationCount: 0,
		busy: false,
		onstart: vi.fn(),
		ondetect: vi.fn(),
		onwritePolicy: vi.fn(),
		onsettings: vi.fn(),
		...overrides
	} as never;
}

/** A row's words, with the markup's line breaks flattened out of them. */
function words(element: HTMLElement): string {
	return (element.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** Type into a bound field the way a person does. */
function type(field: HTMLElement, value: string): void {
	(field as HTMLInputElement).value = value;
	fire(field, 'input');
}

describe('the starter page', () => {
	it('explains the loop before asking for anything', () => {
		const view = render(Starter, props());

		// Four steps, because that is the whole loop: goal, tasks, parallel
		// work, verify and merge.
		expect(view.all('.step')).toHaveLength(4);
		expect(view.find('.lede')).toBeNull();

		view.destroy();
	});

	it('starts a farm from this page, with no navigation', () => {
		const onstart = vi.fn();
		const view = render(Starter, props({ onstart }));

		type(view.get('.goal-title'), '  Add dark mode  ');
		type(view.get('textarea'), '  no new dependencies  ');
		click(view.get('.actions button'));

		// Trimmed, because a goal with trailing spaces is the same goal.
		expect(onstart).toHaveBeenCalledWith('Add dark mode', 'no new dependencies');

		view.destroy();
	});

	it('refuses to start a farm with no goal', () => {
		const onstart = vi.fn();
		const view = render(Starter, props({ onstart }));

		const start = view.get('.actions button') as HTMLButtonElement;
		expect(start.disabled).toBe(true);

		// Whitespace is not a goal either.
		type(view.get('.goal-title'), '   ');
		expect((view.get('.actions button') as HTMLButtonElement).disabled).toBe(true);

		type(view.get('.goal-title'), 'Ship it');
		expect((view.get('.actions button') as HTMLButtonElement).disabled).toBe(false);

		view.destroy();
	});

	it('starts on Enter in the goal field', () => {
		// The field holds one short sentence, so Enter is what a person does
		// with it — and reaching for the mouse to submit one line is a defect.
		const onstart = vi.fn();
		const view = render(Starter, props({ onstart }));

		type(view.get('.goal-title'), 'Ship it');
		press(view.get('.goal-title'), 'Enter');

		expect(onstart).toHaveBeenCalledWith('Ship it', '');

		view.destroy();
	});

	it('does not start on Enter while the goal is empty', () => {
		const onstart = vi.fn();
		const view = render(Starter, props({ onstart }));

		press(view.get('.goal-title'), 'Enter');

		expect(onstart).not.toHaveBeenCalled();

		view.destroy();
	});

	it('says which agents could take a task, and offers to look again', () => {
		const ondetect = vi.fn();
		const view = render(Starter, props({ ready: [agent('Claude Code')], ondetect }));

		const row = view.all('.check')[0];
		expect(row.dataset.state).toBe('ok');
		expect(words(row)).toContain('1 ready — Claude Code');

		click(row.querySelector('button') as HTMLElement);
		expect(ondetect).toHaveBeenCalled();

		view.destroy();
	});

	it('names what it looked for when it found nothing', () => {
		const view = render(Starter, props({ undetected: ['codex', 'cursor'] }));

		const row = view.all('.check')[0];
		expect(row.dataset.state).toBe('warn');
		expect(words(row)).toContain('Codex, Cursor');

		view.destroy();
	});

	it('offers to write AGENTS.md only when there is none', () => {
		const onwritePolicy = vi.fn();
		const missing = render(Starter, props({ onwritePolicy }));

		const row = missing.all('.check')[1];
		expect(row.dataset.state).toBe('warn');
		click(row.querySelector('button') as HTMLElement);
		expect(onwritePolicy).toHaveBeenCalled();
		missing.destroy();

		const present = render(Starter, props({ policySources: ['AGENTS.md'] }));
		const found = present.all('.check')[1];
		expect(found.dataset.state).toBe('ok');
		expect(words(found)).toContain('Read from AGENTS.md');
		expect(found.querySelector('button')).toBeNull();
		present.destroy();
	});

	it('says plainly that nothing checks the work when nothing does', () => {
		const onsettings = vi.fn();
		const none = render(Starter, props({ onsettings }));

		const row = none.all('.check')[2];
		expect(row.dataset.state).toBe('warn');
		expect(words(row)).toContain('checked by nobody');
		click(row.querySelector('button') as HTMLElement);
		expect(onsettings).toHaveBeenCalled();
		none.destroy();

		// Singular and plural, because "1 commands run" is the kind of thing
		// that makes a person distrust the rest of the screen.
		const one = render(Starter, props({ verificationCount: 1 }));
		expect(words(one.all('.check')[2])).toContain('1 command runs');
		one.destroy();

		const two = render(Starter, props({ verificationCount: 2 }));
		expect(words(two.all('.check')[2])).toContain('2 commands run');
		two.destroy();
	});

	it('disables everything it can while an action is in flight', () => {
		const view = render(Starter, props({ busy: true, verificationCount: 1 }));

		for (const button of view.all('button')) {
			expect((button as HTMLButtonElement).disabled, button.textContent ?? '').toBe(true);
		}

		view.destroy();
	});
});
