// SPDX-License-Identifier: GPL-3.0-or-later

/** Mirrors `spagitty_core::home` (FEAT-140). */

export interface Divergence {
	upstream: string;
	ahead: number;
	behind: number;
}

export interface RecentBranch {
	name: string;
	authorName: string;
	/** Unix seconds. */
	time: number;
	ahead: number | null;
	behind: number | null;
	current: boolean;
}

export interface Unreleased {
	total: number;
	/** `[heading, entries]` in the file's order. */
	sections: [string, number][];
}

export interface Overview {
	remote: string | null;
	licence: string | null;
	version: string | null;
	versionTime: number | null;
	upstream: Divergence | null;
	defaultBranch: string | null;
	branches: RecentBranch[];
	branchCount: number;
	/** Unix seconds, newest first, inside the last sixteen weeks. */
	commitTimes: number[];
	unreleased: Unreleased | null;
}
