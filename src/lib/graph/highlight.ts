// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Which rows the Author column's filter keeps bright.
 *
 * A question about *the rows already loaded*, not about the repository, and
 * that is deliberate: the graph's rows already carry what is needed, so the
 * answer costs no round trip. It stops at the end of what has been walked,
 * which is the honest answer anyway — a highlight cannot mean anything about
 * rows that are not on screen to be highlighted.
 *
 * This file used to hold three more functions, answering hover questions:
 * `ancestry`, `ghostPath` and `rowOfRef`. FEAT-023 removed the hover effect
 * they served — dimming most of the screen as the pointer crosses it makes the
 * graph flicker — and they went with it, to
 * `~/claudetrashbin/spagitty-FEAT-023/` rather than into a delete.
 *
 * FEAT-081 brought one hover question back, in a form that does not flicker:
 * `branchOf`, which names the branch a bare commit belongs to so the Branch/Tag
 * gutter can show it faintly while that one row is hovered. It dims nothing and
 * draws on no other row, which is the whole of the difference from what
 * FEAT-023 took out.
 *
 * Everything here is a pure function over a row accessor, so it is testable
 * without a repository and without a DOM.
 */

import type { GraphRow, RefChip } from '../types';

export type RowAt = (index: number) => GraphRow | undefined;

/** Rows whose author matches, for the Author column's filter. */
export function byAuthor(
	needle: string,
	row: RowAt,
	first: number,
	last: number
): Set<number> | null {
	const wanted = needle.trim().toLowerCase();
	if (wanted === '') return null;

	const found = new Set<number>();
	for (let i = first; i <= last; i++) {
		const current = row(i);
		if (!current) continue;
		if (current.authorName.toLowerCase().includes(wanted)) found.add(i);
	}
	return found;
}

/**
 * How far above a commit its children are looked for, in rows.
 *
 * A child is always newer, so always above, and in date order it is usually a
 * few rows up — but a long-lived branch can leave a gap of hundreds while other
 * work lands in between. Past this the answer is "no branch named", which the
 * gutter shows as nothing at all.
 */
const CHILD_WINDOW = 600;

/** Total rows a single `branchOf` may read before it gives up. */
const BUDGET = 6000;

/** The name a row's refs give its branch: local first, then the checked-out one. */
function branchChip(refs: RefChip[]): RefChip | null {
	let best: RefChip | null = null;
	for (const chip of refs) {
		if (chip.kind === 'tag') continue;
		const rank = (chip.kind === 'branch' ? 2 : 0) + (chip.current ? 1 : 0);
		const bestRank = best === null ? -1 : (best.kind === 'branch' ? 2 : 0) + (best.current ? 1 : 0);
		if (rank > bestRank) best = chip;
	}
	return best;
}

/**
 * The branch a commit is on, by walking up the history to the nearest ref.
 *
 * The recording this reproduces shows a faint branch name beside some hovered
 * rows and does not show how it was chosen, so the rule is taken from git's own
 * topology rather than from the picture: follow the commit's **first-parent
 * children** upward until one carries a branch. That is the line `git log
 * --first-parent` walks down from a branch tip, so a commit is named for the
 * branch whose own history it is part of — not for a branch that merely merged
 * it, which is what following any child would give.
 *
 * Where a commit has several first-parent children it is a fork, and the child
 * in the **same lane** wins: the lane is the line the graph draws straight
 * through the commit, so the name agrees with what a person sees continuing
 * upward. A different lane's child is the fallback, and a child that took this
 * commit as a *second* parent — a merge whose branch has since been deleted —
 * is the last resort, which then continues up the branch it was merged into.
 * That is `git name-rev`'s answer for a commit whose own branch is gone.
 *
 * Only the loaded rows are read, and within a window and a budget, because this
 * runs on hover. A commit whose branch tip is beyond them gets no name, which is
 * honest: the gutter says nothing rather than guessing.
 *
 * Returns null for a row that carries a branch of its own. Its chip is already
 * in the gutter, and naming it a second time in the faint style would read as a
 * different branch.
 */
export function branchOf(index: number, row: RowAt): string | null {
	const start = row(index);
	if (!start || branchChip(start.refs) !== null) return null;

	let current = start;
	let budget = BUDGET;

	while (budget > 0) {
		let sameLane: GraphRow | null = null;
		let otherLane: GraphRow | null = null;
		let merged: GraphRow | null = null;

		const floor = Math.max(0, current.index - CHILD_WINDOW);
		for (let i = current.index - 1; i >= floor && budget > 0; i--, budget--) {
			const candidate = row(i);
			if (!candidate) continue;
			const at = candidate.parents.indexOf(current.id);
			if (at === 0) {
				if (candidate.lane === current.lane) {
					sameLane = candidate;
					break;
				}
				otherLane ??= candidate;
			} else if (at > 0) {
				merged ??= candidate;
			}
		}

		const next = sameLane ?? otherLane ?? merged;
		if (next === null) return null;

		const chip = branchChip(next.refs);
		if (chip !== null) return chip.name;
		current = next;
	}

	return null;
}
