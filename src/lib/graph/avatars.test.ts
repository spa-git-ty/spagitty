// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AvatarAnswer } from '$lib/types';

const avatarCall = vi.fn<(email: string) => Promise<AvatarAnswer>>();

vi.mock('$lib/api', () => ({
	avatar: (email: string) => avatarCall(email)
}));

import { avatars } from './avatars.svelte';

/**
 * A picture that decodes.
 *
 * `Image` in this environment does not fetch or decode anything, so `onload`
 * has to be driven by hand — which is closer to the truth than it looks: the
 * store's whole reason for existing is that a picture arrives after the frame
 * that asked for it.
 */
function decodesImmediately() {
	class FakeImage {
		onload: (() => void) | null = null;
		onerror: (() => void) | null = null;
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
}

/** Let every queued microtask and promise settle. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
	vi.clearAllMocks();
	vi.unstubAllGlobals();
	avatars.reset();
	avatars.setEnabled(false);
	avatarCall.mockResolvedValue({ handle: null, picture: null });
});

describe('before the preference has been read', () => {
	it('asks for nothing', async () => {
		// The window between the first paint and the preference landing. A
		// request made in it would be one the user never agreed to.
		avatars.lookup('ada@example.com', 'Ada Lovelace');
		await settle();

		expect(avatarCall).not.toHaveBeenCalled();
	});

	it('still answers, with nothing', () => {
		expect(avatars.lookup('ada@example.com', 'Ada')).toEqual({ image: null, handle: null });
	});
});

describe('asking', () => {
	beforeEach(() => avatars.setEnabled(true));

	it('asks once for an address, however many rows carry it', async () => {
		// A repository with one author and forty thousand commits makes one
		// call, not forty thousand.
		for (let i = 0; i < 50; i++) avatars.lookup('ada@example.com', 'Ada Lovelace');
		await settle();

		expect(avatarCall).toHaveBeenCalledTimes(1);
	});

	it('asks again for a different address', async () => {
		avatars.lookup('ada@example.com', 'Ada');
		avatars.lookup('grace@example.com', 'Grace');
		await settle();

		expect(avatarCall).toHaveBeenCalledTimes(2);
	});

	it('treats two spellings of one address as one person', async () => {
		avatars.lookup('Ada@Example.com', 'Ada');
		avatars.lookup('ada@example.com', 'Ada Lovelace');
		await settle();

		expect(avatarCall).toHaveBeenCalledTimes(1);
	});

	it('does not ask about a commit with no author at all', async () => {
		avatars.lookup('', '');
		await settle();

		expect(avatarCall).not.toHaveBeenCalled();
	});

	it('never asks twice, even when the answer was nothing', async () => {
		// The expensive case: an address with no picture anywhere. Asking it
		// again on every scroll is most of the traffic this could generate.
		avatars.lookup('ada@example.com', 'Ada');
		await settle();
		avatars.lookup('ada@example.com', 'Ada');
		await settle();

		expect(avatarCall).toHaveBeenCalledTimes(1);
	});

	it('keeps going after one address fails', async () => {
		avatarCall.mockRejectedValueOnce(new Error('offline'));
		avatars.lookup('ada@example.com', 'Ada');
		await settle();

		avatars.lookup('grace@example.com', 'Grace');
		await settle();

		expect(avatarCall).toHaveBeenCalledTimes(2);
		expect(avatars.drawable('ada@example.com', 'Ada')).toBe(null);
	});
});

describe('an answer arriving', () => {
	beforeEach(() => avatars.setEnabled(true));

	it('makes the picture drawable, and only once it has decoded', async () => {
		decodesImmediately();
		avatarCall.mockResolvedValue({ handle: null, picture: 'data:image/png;base64,AAA' });

		avatars.lookup('ada@example.com', 'Ada');
		await settle();

		const image = avatars.drawable('ada@example.com', 'Ada');
		expect(image).not.toBe(null);
		expect(image?.src).toBe('data:image/png;base64,AAA');
	});

	it('bumps the version, so a canvas that had painted repaints', async () => {
		decodesImmediately();
		avatarCall.mockResolvedValue({ handle: null, picture: 'data:image/png;base64,AAA' });
		const before = avatars.version;

		avatars.lookup('ada@example.com', 'Ada');
		await settle();

		expect(avatars.version).toBeGreaterThan(before);
	});

	it('carries a handle even when there is no picture', async () => {
		// A no-reply address spells the account out, and that is worth having
		// on its own: it is what the hover says.
		avatarCall.mockResolvedValue({ handle: 'octocat', picture: null });

		avatars.lookup('1+octocat@users.noreply.github.com', 'Octo');
		await settle();

		expect(avatars.lookup('1+octocat@users.noreply.github.com', 'Octo').handle).toBe('octocat');
	});

	it('leaves the picture undrawable when it will not decode', async () => {
		class BrokenImage {
			onload: (() => void) | null = null;
			onerror: (() => void) | null = null;
			set src(_value: string) {
				queueMicrotask(() => this.onerror?.());
			}
		}
		vi.stubGlobal('Image', BrokenImage);
		avatarCall.mockResolvedValue({ handle: null, picture: 'data:image/png;base64,not-a-png' });

		avatars.lookup('ada@example.com', 'Ada');
		await settle();

		expect(avatars.drawable('ada@example.com', 'Ada')).toBe(null);
	});
});

describe('turning the preference off', () => {
	it('drops what was already fetched, rather than going on drawing it', async () => {
		// A picture still on screen after somebody opted out is a preference
		// that did not take effect.
		decodesImmediately();
		avatars.setEnabled(true);
		avatarCall.mockResolvedValue({ handle: null, picture: 'data:image/png;base64,AAA' });

		avatars.lookup('ada@example.com', 'Ada');
		await settle();
		expect(avatars.drawable('ada@example.com', 'Ada')).not.toBe(null);

		avatars.setEnabled(false);

		expect(avatars.drawable('ada@example.com', 'Ada')).toBe(null);
	});

	it('stops asking', async () => {
		avatars.setEnabled(true);
		avatars.setEnabled(false);

		avatars.lookup('ada@example.com', 'Ada');
		await settle();

		expect(avatarCall).not.toHaveBeenCalled();
	});
});
