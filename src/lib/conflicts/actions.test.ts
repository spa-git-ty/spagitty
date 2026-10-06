// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The question the Conflicts screen asks (FEAT-016).
 *
 * Most of the screen is safe — taking a side leaves all three stages in the
 * index, so a wrong click costs another click. Abort is not, and it is
 * asserted on its wording as much as on the gate: it puts something back, and
 * *what* differs by operation. The one people get wrong is cherry-pick, where
 * aborting does not undo the commits it already made.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

// Declared inside the factory: `vi.mock` is hoisted above every top-level
// binding in the file, so a store built out here would not exist yet.
vi.mock('$lib/conflicts/store.svelte', () => ({
	conflicts: {
		writeError: null as string | null,
		abort: vi.fn(() => Promise.resolve(true)),
		continue: vi.fn(() => Promise.resolve(true))
	}
}));
vi.mock('$lib/ui/notice.svelte', () => ({ notice: { ok: vi.fn(), failed: vi.fn() } }));

import { conflicts } from '$lib/conflicts/store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import { abortBody, abortOperation } from './actions';

const store = conflicts as unknown as {
	writeError: string | null;
	abort: ReturnType<typeof vi.fn>;
	continue: ReturnType<typeof vi.fn>;
};

beforeEach(() => {
	vi.clearAllMocks();
	dialog.dismiss();
	store.writeError = null;
});

describe('what aborting puts back', () => {
	it('says a merge touches nothing already committed', () => {
		expect(abortBody('merge')).toContain('Nothing that was already committed is touched');
	});

	it('names ORIG_HEAD for a rebase, and warns about the resolutions', () => {
		const body = abortBody('rebase');
		expect(body).toContain('ORIG_HEAD');
		expect(body).toContain('every conflict resolved since');
	});

	it('says an interactive rebase the same way as a plain one', () => {
		expect(abortBody('rebaseInteractive')).toBe(abortBody('rebase'));
	});

	it('is honest that a cherry-pick keeps the commits it already made', () => {
		// The one people get wrong. "Abort" sounds like "undo all of it".
		expect(abortBody('cherryPick')).toContain('Commits it already made stay');
	});

	it('says the same of a revert', () => {
		expect(abortBody('revert')).toContain('Commits it already made stay');
	});
});

describe('the gate on abort', () => {
	it('asks first, and is painted as destructive', async () => {
		const running = abortOperation('merge');

		expect(dialog.question?.title).toBe('Abort the merge');
		expect(dialog.question?.danger).toBe(true);
		expect(store.abort).not.toHaveBeenCalled();

		dialog.accept();
		expect(await running).toBe(true);
		expect(store.abort).toHaveBeenCalled();
	});

	it('names an interactive rebase properly in the title', async () => {
		const running = abortOperation('rebaseInteractive');
		expect(dialog.question?.title).toBe('Abort the interactive rebase');
		dialog.dismiss();
		await running;
	});

	it('does nothing when dismissed', async () => {
		const running = abortOperation('merge');
		dialog.dismiss();

		expect(await running).toBe(false);
		expect(store.abort).not.toHaveBeenCalled();
	});

	it('asks nothing when there is no operation to abort', async () => {
		expect(await abortOperation('none')).toBe(false);
		expect(dialog.question).toBeNull();
	});
});
