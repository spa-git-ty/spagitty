// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A Farm task is merged in Merger like anything else (FEAT-117): the Farm
 * opens Merger on the task's branch and the branch it was meant for, and is
 * told when the merge is written.
 */

import { goto } from '$app/navigation';
import * as api from './api';
import { merger } from '../merger/store.svelte';
import { repo } from '../repo.svelte';
import { notice } from '../ui/notice.svelte';
import type { Task } from './types';

/** Where the task's work is meant to go: the branch it recorded, else the one checked out. */
export function mergeTargetOf(task: Task): string | null {
	return task.mergeTarget ?? repo.info?.head.branch ?? null;
}

/**
 * Open Merger on this task. False when there is nothing to merge, nowhere to
 * put it, or the Farm would not merge it now.
 *
 * The Farm is asked first (BUG-077): Merger makes the merge, and a task that
 * is still running, or still owes a required supplemental review, is refused
 * here with the Farm's own reason and not after the merge is written.
 */
export async function openInMerger(task: Task): Promise<boolean> {
	const target = mergeTargetOf(task);
	if (!task.branch || !target) {
		notice.failed('Could not open Merger', 'This task has no branch to merge, or nowhere recorded to merge it to.');
		return false;
	}
	try {
		await api.mergeCheck(task.id);
	} catch (e) {
		notice.failed('Could not open Merger', String(e));
		return false;
	}
	merger.presentFarmTask({ id: task.id, branch: task.branch, target, title: task.title });
	void goto('/merge');
	return true;
}
