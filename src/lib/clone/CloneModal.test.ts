// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, press, render } from '../../testing/mount';
import type { ClonePlan } from '$lib/types';

vi.mock('$lib/api', () => ({
	inTauri: vi.fn(() => true),
	clonePlan: vi.fn(),
	cloneStart: vi.fn(),
	cloneRelease: vi.fn(() => Promise.resolve())
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

const listeners = new Map<string, (event: { payload: unknown }) => void>();
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((name: string, handler: (event: { payload: unknown }) => void) => {
		listeners.set(name, handler);
		return Promise.resolve(() => listeners.delete(name));
	})
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import * as api from '$lib/api';
import CloneModal from './CloneModal.svelte';
import { clone } from './store.svelte';

const clonePlan = vi.mocked(api.clonePlan);
const cloneStart = vi.mocked(api.cloneStart);

function aPlan(overrides: Partial<ClonePlan> = {}): ClonePlan {
	return {
		name: 'project',
		destination: '/work/project',
		createsDestination: true,
		problem: null,
		message: null,
		...overrides
	};
}

async function planned(): Promise<void> {
	clone.setUrl('https://example.com/owner/project.git');
	await vi.waitFor(() => expect(clone.plan.destination).not.toBeNull());
	flushSync();
}

beforeEach(async () => {
	vi.clearAllMocks();
	listeners.clear();
	clone.clear();
	clonePlan.mockResolvedValue(aPlan());
	await clone.attach();
});

describe('CloneModal', () => {
	it('is not in the document until it is opened', () => {
		const mounted = render(CloneModal, {});

		expect(mounted.find('[role="dialog"]')).toBeNull();

		mounted.destroy();
	});

	it('shows the exact path the clone will land at', async () => {
		// Criterion 2, on screen: the path itself, not a description of it.
		clone.show();
		const mounted = render(CloneModal, {});
		await planned();

		expect(mounted.text()).toContain('/work/project');

		mounted.destroy();
	});

	it('cannot start until there is a destination and nothing wrong with it', async () => {
		clone.show();
		const mounted = render(CloneModal, {});
		const start = () =>
			mounted.all('button').find((button) => button.textContent?.trim() === 'Clone');

		expect((start() as HTMLButtonElement).disabled).toBe(true);
		await planned();
		expect((start() as HTMLButtonElement).disabled).toBe(false);

		mounted.destroy();
	});

	it('shows the reason a refused plan was refused', async () => {
		clonePlan.mockResolvedValue(
			aPlan({
				problem: { kind: 'destinationNotEmpty', detail: '/work/project' },
				message: '/work/project already exists and is not empty. Nothing has been changed.'
			})
		);
		clone.show();
		const mounted = render(CloneModal, {});
		await planned();

		expect(mounted.text()).toContain('already exists and is not empty');

		mounted.destroy();
	});

	it('starts the clone and then offers to stop it', async () => {
		clone.show();
		const mounted = render(CloneModal, {});
		await planned();
		cloneStart.mockResolvedValueOnce(7);

		const start = mounted.all('button').find((b) => b.textContent?.trim() === 'Clone');
		click(start as HTMLElement);
		await vi.waitFor(() => expect(clone.running).toBe(true));
		flushSync();

		expect(mounted.text()).toContain('Stop');
		expect(mounted.find('.bar')).not.toBeNull();

		mounted.destroy();
	});

	it('shows the phase and the percentage git reported', async () => {
		clone.show();
		const mounted = render(CloneModal, {});
		await planned();
		cloneStart.mockResolvedValueOnce(7);
		click(mounted.all('button').find((b) => b.textContent?.trim() === 'Clone') as HTMLElement);
		await vi.waitFor(() => expect(clone.running).toBe(true));

		listeners.get('clone-progress')?.({
			payload: {
				token: 7,
				phase: 'Receiving objects',
				percent: 57,
				line: 'Receiving objects:  57% (123/456)'
			}
		});
		flushSync();

		expect(mounted.text()).toContain('Receiving objects');
		expect(mounted.text()).toContain('57%');
		expect(mounted.get('.fill').style.width).toBe('57%');

		mounted.destroy();
	});

	it('offers to open what was cloned once it finishes', async () => {
		clone.show();
		const mounted = render(CloneModal, {});
		await planned();
		cloneStart.mockResolvedValueOnce(7);
		click(mounted.all('button').find((b) => b.textContent?.trim() === 'Clone') as HTMLElement);
		await vi.waitFor(() => expect(clone.running).toBe(true));

		listeners.get('clone-done')?.({
			payload: { token: 7, ok: true, cancelled: false, error: null, path: '/work/project' }
		});
		flushSync();

		expect(mounted.text()).toContain('Open it');
		expect(mounted.text()).toContain('Cloned into');

		mounted.destroy();
	});

	it('asks for no password, and says nothing about not asking (TASK-050)', async () => {
		clone.show();
		const mounted = render(CloneModal, {});

		expect(mounted.find('input[type="password"]')).toBeNull();
		expect(mounted.text()).not.toContain('never asks for a password');

		mounted.destroy();
	});

	it('closes on Escape without stopping anything', () => {
		clone.show();
		const mounted = render(CloneModal, {});

		press(mounted.get('[role="dialog"]'), 'Escape');

		expect(clone.open).toBe(false);
		mounted.destroy();
	});
});
