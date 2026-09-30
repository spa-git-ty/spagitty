// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The External Tools section on screen (FEAT-068, mounted under FEAT-072).
 *
 * `tools-section.test.ts` beside this one tests the api calls the section
 * makes; it never mounts the component, which is how the section came to be
 * themed against three tokens that do not exist without anything failing. These
 * tests mount it, so what the section actually draws is under test at all.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from '../../testing/mount';
import { flushSync } from 'svelte';
import type { ExternalToolsConfig } from '$lib/types';

vi.mock('$lib/api', () => ({
	externalToolsConfig: vi.fn(),
	setExternalTool: vi.fn(() => Promise.resolve())
}));

import * as api from '$lib/api';
import ExternalToolsSection from './ExternalToolsSection.svelte';

const externalToolsConfig = vi.mocked(api.externalToolsConfig);
const setExternalTool = vi.mocked(api.setExternalTool);

function aConfig(overrides: Partial<ExternalToolsConfig> = {}): ExternalToolsConfig {
	return {
		diffTool: null,
		mergeTool: null,
		availableDiffTools: [
			{ id: 'meld', name: 'Meld', command: 'meld $LOCAL $REMOTE', isInstalled: true },
			{ id: 'bc', name: 'Beyond Compare', command: 'bcompare', isInstalled: false }
		],
		availableMergeTools: [
			{ id: 'meld', name: 'Meld', command: 'meld $LOCAL $BASE $REMOTE -o $MERGED', isInstalled: true }
		],
		...overrides
	};
}

/** Mount and let the `onMount` load settle. */
async function mounted(config: ExternalToolsConfig = aConfig()) {
	externalToolsConfig.mockResolvedValue(config);
	const view = render(ExternalToolsSection, {});
	await vi.waitFor(() => expect(view.find('select')).not.toBeNull());
	flushSync();
	return view;
}

beforeEach(() => {
	vi.clearAllMocks();
});

describe('what the section says before it knows anything', () => {
	it('says it is looking rather than showing an empty list', async () => {
		// A never-resolving read: the screen has to have something to say in
		// the meantime, and "no tools" would be a lie.
		externalToolsConfig.mockReturnValue(new Promise(() => {}));
		const view = render(ExternalToolsSection, {});

		expect(view.text()).toContain('Reading');
		expect(view.find('select')).toBeNull();

		view.destroy();
	});

	it('offers a retry when the read fails', async () => {
		externalToolsConfig.mockRejectedValue(new Error('no git here'));
		const view = render(ExternalToolsSection, {});
		await vi.waitFor(() => expect(view.text()).not.toContain('Reading'));

		expect(view.find('select')).toBeNull();
		expect(view.text()).toContain('Could not load external tools configuration');
		expect(view.text()).toContain('Try again');

		view.destroy();
	});
});

describe('what is configured now', () => {
	it('names the built-in when nothing is configured', async () => {
		const view = await mounted();

		expect(view.text()).toContain('Built-in');

		view.destroy();
	});

	it('selects the configured tool in the picker rather than the first option', async () => {
		const view = await mounted(aConfig({ diffTool: 'meld' }));

		expect((view.all('select')[0] as HTMLSelectElement).value).toBe('meld');
		expect((view.all('select')[1] as HTMLSelectElement).value).toBe('');

		view.destroy();
	});
});

describe('the pickers', () => {
	it('offers the built-in as a real choice, not only the tools found', async () => {
		const view = await mounted();

		const options = [...(view.all('select')[0] as HTMLSelectElement).options].map((o) => o.value);
		expect(options[0]).toBe('');
		expect(options).toContain('meld');

		view.destroy();
	});

	it('offers a tool that is not installed, and says so', async () => {
		// Removing it would leave a git config someone already has looking like
		// an invalid value; the answer is to show it and mark it.
		const view = await mounted();

		const text = view.text();
		expect(text).toContain('Beyond Compare');
		expect(text).toContain('(not installed)');
		expect(text).not.toContain('(detected)');

		view.destroy();
	});

	it('writes the chosen diff tool and re-reads the result', async () => {
		const view = await mounted();
		externalToolsConfig.mockResolvedValue(aConfig({ diffTool: 'meld' }));

		const picker = view.all('select')[0] as HTMLSelectElement;
		picker.value = 'meld';
		picker.dispatchEvent(new Event('change', { bubbles: true }));
		await vi.waitFor(() => expect(setExternalTool).toHaveBeenCalled());

		expect(setExternalTool).toHaveBeenCalledWith('diff', 'meld', false);
		await vi.waitFor(() => expect(externalToolsConfig).toHaveBeenCalledTimes(2));

		view.destroy();
	});

	it('writes the chosen merge tool against the merge setting', async () => {
		const view = await mounted();

		const picker = view.all('select')[1] as HTMLSelectElement;
		picker.value = 'meld';
		picker.dispatchEvent(new Event('change', { bubbles: true }));
		await vi.waitFor(() => expect(setExternalTool).toHaveBeenCalled());

		expect(setExternalTool).toHaveBeenCalledWith('merge', 'meld', false);

		view.destroy();
	});

	it('sends null rather than an empty string when the built-in is chosen', async () => {
		// `diff.tool = ""` is not the same as no `diff.tool` at all, and the
		// backend is the one that knows how to unset it.
		const view = await mounted(aConfig({ diffTool: 'meld' }));

		const picker = view.all('select')[0] as HTMLSelectElement;
		picker.value = '';
		picker.dispatchEvent(new Event('change', { bubbles: true }));
		await vi.waitFor(() => expect(setExternalTool).toHaveBeenCalled());

		expect(setExternalTool).toHaveBeenCalledWith('diff', null, false);

		view.destroy();
	});

	it('survives a write that fails, and leaves the pickers usable', async () => {
		const view = await mounted();
		setExternalTool.mockRejectedValueOnce(new Error('config is read-only'));

		const picker = view.all('select')[0] as HTMLSelectElement;
		picker.value = 'meld';
		picker.dispatchEvent(new Event('change', { bubbles: true }));
		await vi.waitFor(() => expect(view.all('select')[0].hasAttribute('disabled')).toBe(false));

		view.destroy();
	});
});

describe('which git config is written', () => {
	it('writes to the repository by default', async () => {
		const view = await mounted();

		expect((view.get('.scope-toggle input') as HTMLInputElement).checked).toBe(false);

		view.destroy();
	});

	it('writes to the global config once that is asked for', async () => {
		const view = await mounted();

		const toggle = view.get('.scope-toggle input') as HTMLInputElement;
		toggle.checked = true;
		toggle.dispatchEvent(new Event('change', { bubbles: true }));
		flushSync();

		const picker = view.all('select')[0] as HTMLSelectElement;
		picker.value = 'meld';
		picker.dispatchEvent(new Event('change', { bubbles: true }));
		await vi.waitFor(() => expect(setExternalTool).toHaveBeenCalled());

		expect(setExternalTool).toHaveBeenCalledWith('diff', 'meld', true);

		view.destroy();
	});
});
