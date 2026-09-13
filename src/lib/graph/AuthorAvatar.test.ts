// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * FEAT-081: one author mark for the Author column and the commit detail.
 *
 * Both surfaces mount this component, and the lane canvas reads the same
 * `avatars.drawable` it does, so proving the component switches from the
 * generated face to the decoded picture by address is proving they switch
 * together.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, render } from '../../testing/mount';
import type { AvatarAnswer } from '$lib/types';

const avatarCall = vi.fn<(email: string, commit: string | null) => Promise<AvatarAnswer>>();
vi.mock('$lib/api', () => ({
	avatar: (email: string, commit: string | null) => avatarCall(email, commit)
}));

import { avatars } from './avatars.svelte';
import AuthorAvatar from './AuthorAvatar.svelte';

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
	class FakeImage {
		onload: (() => void) | null = null;
		#src = '';
		get src(): string {
			return this.#src;
		}
		set src(value: string) {
			this.#src = value;
			queueMicrotask(() => this.onload?.());
		}
	}
	vi.stubGlobal('Image', FakeImage);
	avatars.reset();
	avatars.setEnabled(true);
});

afterEach(() => {
	vi.unstubAllGlobals();
	avatars.setEnabled(false);
});

describe('AuthorAvatar', () => {
	it('draws the generated initials until a picture arrives, then the picture on every copy', async () => {
		avatarCall.mockResolvedValue({ handle: 'thisisgm', picture: 'data:image/png;base64,AAA', retry: false });

		// The Author column's copy and the detail pane's copy of one person,
		// spelled differently the way git lets two commits spell one address.
		const column = render(AuthorAvatar, { email: 'gm@icloud.com', name: 'Gianmarco', letters: 'GM' });
		const detail = render(AuthorAvatar, { email: 'GM@icloud.com ', name: 'Gianmarco Morales' });

		expect(column.get('.avatar .letters').textContent).toBe('GM');
		expect(detail.get('.avatar .letters').textContent).toBe('GM');

		avatars.lookup('gm@icloud.com', 'Gianmarco', 'a'.repeat(40));
		await settle();
		flushSync();

		for (const view of [column, detail]) {
			const mark = view.get('.avatar');
			expect(mark.classList.contains('photo')).toBe(true);
			expect(mark.getAttribute('style')).toContain('data:image/png;base64,AAA');
			expect(view.find('.letters')).toBe(null);
		}
		// And the canvas draws the very same decoded image.
		expect(avatars.drawable('gm@icloud.com')?.src).toBe('data:image/png;base64,AAA');

		column.destroy();
		detail.destroy();
	});

	it('is one fixed square size, taken from the graph node', async () => {
		const css = await import('node:fs').then((fs) =>
			fs.readFileSync('src/lib/graph/AuthorAvatar.svelte', 'utf8')
		);
		expect(css).toMatch(/inline-size:\s*var\(--avatar-d, 22px\)/);
		expect(css).toMatch(/block-size:\s*var\(--avatar-d, 22px\)/);
		expect(css).toMatch(/aspect-ratio:\s*1/);
		expect(css).toMatch(/flex:\s*none/);

		// No surface keeps a size of its own any more.
		for (const file of ['CommitRows.svelte', 'CommitDetail.svelte']) {
			const source = await import('node:fs').then((fs) =>
				fs.readFileSync(`src/lib/graph/${file}`, 'utf8')
			);
			expect(source, file).not.toMatch(/\.avatar\s*\{/);
		}
	});
});
