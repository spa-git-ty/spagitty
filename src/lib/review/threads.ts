// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A pull request's review comments as threads (FEAT-091).
 *
 * Both hosts hand comments over as a flat list in which a reply names what it
 * answers. GitHub chains replies one to another; GitLab's notes all name the
 * first note of their discussion. Either way a thread is everything that leads
 * back to one comment that answers nothing.
 */

import type { PullRequestComment } from '../types';

export interface Thread {
	/** The first comment's id. */
	id: number;
	path: string;
	/** The line the thread is on, or null when the host no longer places it. */
	line: number | null;
	side: 'LEFT' | 'RIGHT';
	resolved: boolean;
	/** How the host names it to resolve (FEAT-093); null when it cannot be. */
	threadId: string | null;
	/** On the pull request as a whole rather than on a line (FEAT-093). */
	general: boolean;
	/** Oldest first. */
	comments: PullRequestComment[];
}

/** Every thread, in the order their first comments were written. */
export function threadsOf(comments: PullRequestComment[]): Thread[] {
	const byId = new Map(comments.map((comment) => [comment.id, comment]));

	// The comment a reply ultimately answers. A chain that names a comment not
	// in the list ends there; one that loops settles on the loop's oldest
	// comment, so every comment in it agrees on one thread.
	const rootOf = (comment: PullRequestComment): PullRequestComment => {
		const path = [comment];
		let at = comment;
		while (at.inReplyTo !== null) {
			const parent = byId.get(at.inReplyTo);
			if (!parent) break;
			const looped = path.indexOf(parent);
			if (looped >= 0) {
				return path.slice(looped).reduce((oldest, item) => (item.id < oldest.id ? item : oldest));
			}
			path.push(parent);
			at = parent;
		}
		return at;
	};

	const threads = new Map<number, Thread>();
	for (const comment of comments) {
		const root = rootOf(comment);
		let thread = threads.get(root.id);
		if (!thread) {
			thread = {
				id: root.id,
				path: root.path,
				line: root.line,
				side: root.side === 'LEFT' ? 'LEFT' : 'RIGHT',
				resolved: root.resolved,
				threadId: root.threadId ?? null,
				general: root.path === '',
				comments: []
			};
			threads.set(root.id, thread);
		}
		thread.comments.push(comment);
		// A reply posted from here comes back without the thread's id.
		thread.threadId ??= comment.threadId ?? null;
	}
	for (const thread of threads.values()) {
		thread.comments.sort((a, b) => a.createdAt - b.createdAt || a.id - b.id);
	}
	return [...threads.values()].sort(
		(a, b) => a.comments[0].createdAt - b.comments[0].createdAt || a.id - b.id
	);
}

/** Where a thread sits, as a map key: `path`, side and line. */
export function placeOf(path: string, side: 'LEFT' | 'RIGHT', line: number): string {
	return `${path}\n${side}\n${line}`;
}

/** Threads by where they sit. Threads the host no longer places are left out. */
export function threadsByPlace(threads: Thread[]): Map<string, Thread[]> {
	const out = new Map<string, Thread[]>();
	for (const thread of threads) {
		if (thread.line === null) continue;
		const place = placeOf(thread.path, thread.side, thread.line);
		const list = out.get(place);
		if (list) list.push(thread);
		else out.set(place, [thread]);
	}
	return out;
}

/** `name.rs:20`, `name.rs` when the thread has no line, or `whole PR`. */
export function whereOf(thread: Thread): string {
	if (thread.general) return 'whole PR';
	const name = thread.path.slice(thread.path.lastIndexOf('/') + 1);
	return thread.line === null ? name : `${name}:${thread.line}`;
}
