// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Open a file in Files from any screen.
 *
 * A row that names a file says so with `data-file-path`, and the revision
 * it is showing with `data-file-rev`. The window's right-click menu
 * (`FileMenu.svelte`) reads them and offers **Open in Files** at that
 * revision. A file Files cannot read, one not in a commit yet, carries
 * `data-file-absent` with the reason, and the entry is shown with it.
 *
 * A screen whose rows already have a menu of their own puts the same entry
 * in it with `openInFiles`.
 */

import { goto } from '$app/navigation';

export interface FileTarget {
	path: string;
	/** Any revision git can read. Files' own revision when left out. */
	rev?: string;
	line?: number;
}

/** The address that opens `path` in Files. */
export function filesHref({ path, rev, line }: FileTarget): string {
	const query = new URLSearchParams({ path });
	if (rev) query.set('rev', rev);
	if (line !== undefined && line > 0) query.set('line', String(line));
	return `/files?${query}`;
}

export function openInFiles(target: FileTarget): Promise<void> {
	return goto(filesHref(target));
}

/**
 * The revision a commit's file is read at. A file the commit deleted is not
 * in it, so it is read as the commit's parent left it.
 */
export function revOfChange(commit: string, status: string): string {
	return status === 'deleted' ? `${commit}^` : commit;
}

/**
 * A pull request's file, as the review room has it: at the fetched head, or
 * where the head left the target when the pull request deletes it.
 */
export function pullFileAttrs(
	path: string,
	status: string,
	head: { head: string; mergeBase: string } | null
) {
	if (!head) return fileAttrs(path, null, 'Not fetched');
	return fileAttrs(path, status === 'deleted' ? head.mergeBase : head.head);
}

/**
 * The attributes that make a row openable in Files: spread onto the row.
 * Without a revision, `absent` says why there is nothing to read.
 */
export function fileAttrs(path: string, rev: string | null | undefined, absent?: string) {
	return {
		'data-file-path': path,
		'data-file-rev': rev ?? undefined,
		'data-file-absent': absent || (rev ? undefined : 'Not in a commit')
	};
}
