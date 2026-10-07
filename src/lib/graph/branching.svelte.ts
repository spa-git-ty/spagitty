// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A branch named where HEAD is (FEAT-104).
 *
 * The toolbar's Branch opens a name field on the graph, in the row HEAD points
 * at, the way GitKraken does — no dialog, no other screen. Enter creates the
 * branch there and checks it out; Escape or leaving the field puts it away.
 *
 * Only the commit the field is on is kept here: the field itself belongs to
 * the row that draws it, and the row may not exist until the graph scrolls to
 * it.
 */

let at = $state<string | null>(null);

/**
 * What a typed name becomes: spaces turn into dashes, as GitKraken does, and
 * the ends are trimmed. What git itself refuses is left for git to say.
 */
export function branchName(typed: string): string {
	return typed.trim().replace(/\s+/g, '-');
}

export const branching = {
	/** The commit the name field is on, or null when there is none. */
	get at(): string | null {
		return at;
	},
	start(id: string): void {
		at = id;
	},
	cancel(): void {
		at = null;
	}
};
