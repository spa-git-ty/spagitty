// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How the review room's file list is shown (FEAT-120): put away to give the
 * diff the width, and either *detailed*, as it was, or *compact*, a line per
 * file for a pull request with many.
 *
 * Remembered on this machine, like the conversation card's. A convenience, so
 * a private window that cannot keep it still works.
 */

export type FilesSize = 'detailed' | 'compact';

const KEY = 'spagitty.review.files';

interface Saved {
	hidden: boolean;
	size: FilesSize;
}

function read(): Saved {
	try {
		const raw = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<Saved>;
		return {
			hidden: raw.hidden === true,
			size: raw.size === 'compact' ? 'compact' : 'detailed'
		};
	} catch {
		return { hidden: false, size: 'detailed' };
	}
}

const first = read();
let hidden = $state(first.hidden);
let size = $state<FilesSize>(first.size);

function save() {
	try {
		localStorage.setItem(KEY, JSON.stringify({ hidden, size } satisfies Saved));
	} catch {
		// The choice lasts the session.
	}
}

export const filesView = {
	/** The list is put away; an edge tab brings it back. */
	get hidden(): boolean {
		return hidden;
	},
	get size(): FilesSize {
		return size;
	},
	setHidden(next: boolean) {
		hidden = next;
		save();
	},
	toggleHidden() {
		this.setHidden(!hidden);
	},
	setSize(next: FilesSize) {
		size = next;
		save();
	},

	/** For tests: what was saved is read again, as at a restart. */
	restart() {
		const saved = read();
		hidden = saved.hidden;
		size = saved.size;
	},
	/** For tests: back to the defaults, and forgotten. */
	reset() {
		hidden = false;
		size = 'detailed';
		try {
			localStorage.removeItem(KEY);
		} catch {
			// Nothing was kept.
		}
	}
};
