// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Assign popover and the two sheets that add agents (2.0), mounted.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { click, render } from '../../testing/mount';
import { aLocal, aRemote, aSnapshot, someRules } from '../../testing/agent-fixtures';

vi.mock('./api', () => ({
	snapshot: vi.fn(),
	list: vi.fn(() => Promise.resolve([])),
	models: vi.fn(() => Promise.resolve(['claude-sonnet-5-5', 'claude-opus-5-5'])),
	testRemote: vi.fn(() => Promise.resolve({ ok: true, said: 'claude-sonnet-5-5 answered', ms: 1200 })),
	saveRemote: vi.fn(() => Promise.resolve({}))
}));

import * as api from './api';
import { agents } from './store.svelte';
import AssignPopover from './AssignPopover.svelte';
import ApiAgentSheet from './ApiAgentSheet.svelte';
import CustomAgentSheet from './CustomAgentSheet.svelte';

const text = (el: Element | null | undefined) => el?.textContent?.replace(/\s+/g, ' ').trim() ?? '';

function type(input: HTMLInputElement, value: string) {
	input.value = value;
	input.dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
}

beforeEach(() => {
	vi.clearAllMocks();
	agents.reset(
		aSnapshot({
			local: [aLocal(), aLocal({ id: 'codex', name: 'Codex' })],
			remote: [aRemote()],
			rules: someRules({ highest: 'signOff' })
		})
	);
});

describe('the Assign popover', () => {
	it('starts from the default agent and level, and assigns what is chosen', () => {
		const onassign = vi.fn();
		const view = render(AssignPopover, { job: 'review' as const, onassign, oncancel: vi.fn() });
		const checked = view.all('[role="radio"][aria-checked="true"]').map(text);
		expect(checked[0]).toContain('Claude Code');
		click(view.all('button').find((b) => text(b) === 'Assign')!);
		expect(onassign).toHaveBeenCalledWith({ agent: 'claude', level: 'stepByStep', note: '', lands: false });
	});

	it('lists a refused agent, disabled, with the reason', () => {
		const view = render(AssignPopover, {
			job: 'review' as const,
			refused: { codex: 'wrote commits here' },
			onassign: vi.fn(),
			oncancel: vi.fn()
		});
		const codex = view.all('[role="radio"]').find((b) => text(b).includes('Codex')) as HTMLButtonElement;
		expect(codex.disabled).toBe(true);
		expect(text(codex)).toContain('wrote commits here');
	});

	it('does not offer a level above the repository’s highest', () => {
		const view = render(AssignPopover, { job: 'review' as const, onassign: vi.fn(), oncancel: vi.fn() });
		const unattended = view.all('button').find((b) => text(b) === 'Unattended') as HTMLButtonElement;
		expect(unattended.disabled).toBe(true);
		expect(unattended.title).toBe('Capped at Sign off here');
	});

	it('says what leaves the machine for a remote agent', () => {
		const view = render(AssignPopover, { job: 'review' as const, onassign: vi.fn(), oncancel: vi.fn() });
		click(view.all('[role="radio"]').find((b) => text(b).includes('Anthropic'))!);
		expect(view.text()).toContain('Code you assign it is sent to Anthropic.');
	});

	it('hands over landing only at a level that can land', () => {
		const onassign = vi.fn();
		const view = render(AssignPopover, { job: 'merge' as const, onassign, oncancel: vi.fn() });
		const land = () => view.all('button').find((b) => text(b) === 'Resolve, check and land') as HTMLButtonElement;
		expect(land().disabled).toBe(true);
		click(view.all('button').find((b) => text(b) === 'Sign off')!);
		expect(land().disabled).toBe(false);
		click(land());
		const note = view.get('input[aria-label="Note for the agent"]') as HTMLInputElement;
		type(note, 'main is the one that matters');
		click(view.all('button').find((b) => text(b) === 'Assign')!);
		expect(onassign).toHaveBeenCalledWith({ agent: 'claude', level: 'signOff', note: 'main is the one that matters', lands: true });
	});

	it('cancels on Escape', () => {
		const oncancel = vi.fn();
		const view = render(AssignPopover, { job: 'review' as const, onassign: vi.fn(), oncancel });
		view.get('[role="dialog"]').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
		expect(oncancel).toHaveBeenCalled();
	});
});

describe('Add an API agent', () => {
	it('takes a provider, a key and a model, tests it, and adds it', async () => {
		const onclose = vi.fn();
		const view = render(ApiAgentSheet, { onclose });
		expect(view.text()).toContain('Code you assign it is sent to Anthropic. Each repository asks once.');
		type(view.get('#api-key') as HTMLInputElement, 'sk-ant-secret-1234');
		type(view.get('#api-model') as HTMLInputElement, 'claude-sonnet-5-5');
		click(view.all('button').find((b) => text(b) === 'Test')!);
		await vi.waitFor(() => expect(view.text()).toContain('1.2 s'));
		expect(api.testRemote).toHaveBeenCalledWith(
			expect.objectContaining({ provider: 'anthropic', model: 'claude-sonnet-5-5', key: 'sk-ant-secret-1234' })
		);
		click(view.all('button').find((b) => text(b) === 'Add')!);
		await vi.waitFor(() => expect(onclose).toHaveBeenCalled());
		expect(api.saveRemote).toHaveBeenCalledWith(
			expect.objectContaining({ name: 'Anthropic', provider: 'anthropic', tokensPerRun: 40000, tokensPerDay: 400000, key: 'sk-ant-secret-1234' })
		);
	});

	it('an endpoint on this machine needs no key and sends nothing anywhere', () => {
		const view = render(ApiAgentSheet, { onclose: vi.fn() });
		click(view.all('button').find((b) => text(b) === 'OpenAI-compatible')!);
		expect((view.get('#api-name') as HTMLInputElement).value).toBe('OpenAI-compatible');
		type(view.get('#api-base') as HTMLInputElement, 'http://localhost:11434/v1');
		type(view.get('#api-model') as HTMLInputElement, 'qwen3-coder:30b');
		expect(view.text()).toContain('Everything stays on this machine.');
		const add = view.all('button').find((b) => text(b) === 'Add') as HTMLButtonElement;
		expect(add.disabled).toBe(false);
	});

	it('editing keeps the stored key unless a new one is typed', async () => {
		const view = render(ApiAgentSheet, { editing: aRemote(), onclose: vi.fn() });
		expect((view.get('#api-key') as HTMLInputElement).placeholder).toBe('stored, ends 7Qx2');
		click(view.all('button').find((b) => text(b) === 'Save')!);
		await vi.waitFor(() => expect(api.saveRemote).toHaveBeenCalledWith(expect.objectContaining({ id: 'api-1', key: null })));
	});

	it('a failed test says the provider’s own words', async () => {
		vi.mocked(api.testRemote).mockResolvedValueOnce({ ok: false, said: 'Anthropic: invalid x-api-key', ms: 0 });
		const view = render(ApiAgentSheet, { onclose: vi.fn() });
		type(view.get('#api-key') as HTMLInputElement, 'nope-nope-nope');
		type(view.get('#api-model') as HTMLInputElement, 'm');
		click(view.all('button').find((b) => text(b) === 'Test')!);
		await vi.waitFor(() => expect(view.text()).toContain('Anthropic: invalid x-api-key'));
	});
});

describe('Add a command-line agent', () => {
	it('takes a name, a command and where the prompt goes', async () => {
		const onsave = vi.fn(() => Promise.resolve(true));
		const onclose = vi.fn();
		const view = render(CustomAgentSheet, { onsave, onclose });
		type(view.get('#custom-name') as HTMLInputElement, 'Review Bot');
		type(view.get('#custom-command') as HTMLInputElement, '~/tools/review-bot');
		type(view.get('#custom-args') as HTMLInputElement, '--diff');
		click(view.all('button').find((b) => text(b) === 'Add')!);
		await vi.waitFor(() => expect(onclose).toHaveBeenCalled());
		expect(onsave).toHaveBeenCalledWith(
			expect.objectContaining({
				id: 'custom-review-bot',
				displayName: 'Review Bot',
				provider: 'custom',
				inputMode: 'cliPrompt',
				extraArgs: ['--diff', '{prompt}']
			})
		);
	});

	it('on standard input, the prompt is not an argument', async () => {
		const onsave = vi.fn(() => Promise.resolve(true));
		const view = render(CustomAgentSheet, { onsave, onclose: vi.fn() });
		type(view.get('#custom-name') as HTMLInputElement, 'bot');
		type(view.get('#custom-command') as HTMLInputElement, 'bot');
		click(view.all('button').find((b) => text(b) === 'on standard input')!);
		click(view.all('button').find((b) => text(b) === 'Add')!);
		await vi.waitFor(() => expect(onsave).toHaveBeenCalledWith(expect.objectContaining({ inputMode: 'stdin', extraArgs: [] })));
	});
});
