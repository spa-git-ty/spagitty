// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What a launch resumes, as a sequence a test can watch.
 *
 * The shell used to hold this in `onMount`: read the launch path, open it, or
 * else reopen the tab the last session was on, then land on its route and its
 * selected commit. BUG-013 was one missing call in that run — the tab strip
 * came back with a repository's name across the top and nothing had told the
 * backend to open it — and no test could have caught it, because `src/routes/**`
 * is outside the coverage scope on purpose.
 *
 * Nothing about the order was hard. It was only unreachable. So the order lives
 * here, behind a port of five operations, and the shell supplies the real ones.
 * The failure that matters is a call that does not happen, which is why the
 * tests assert the sequence rather than a return value.
 */

import type { Place } from './workspace.svelte';

/** Where a launch lands (FEAT-140). */
export const HOME_ROUTE = '/home';

/**
 * The operations a resume performs. Everything here is asynchronous or a
 * side effect; none of it is a decision, and that is the split.
 */
export interface ResumePort {
	/** A path given on the command line, or `null` when there was none. */
	launchPath(): Promise<string | null>;
	/** Opens a repository. `false` means it did not resolve — moved or deleted. */
	open(path: string): Promise<boolean>;
	/** The tab the last session left active, or `null` for an empty strip. */
	active(): string | null;
	/** The route and selection that tab was left on. */
	placeOf(path: string): Place | null;
	/** The route the window is on now, so an unchanged route is not navigated. */
	route(): string;
	/** Navigates. Only called when the stored route differs from the current one. */
	goto(route: string): Promise<void>;
	/** Holds a commit id across a walk that has not reached it yet. */
	want(id: string): void;
	/** True once the shell has been torn down; every await checks it. */
	cancelled(): boolean;
}

/**
 * Opens whatever this launch is supposed to open.
 *
 * A path on the command line wins: it is what the person just asked for, and it
 * is more specific than what they were doing last time. Otherwise the last
 * session's active tab comes back, and with it the commit it was left on; the
 * screen is Home.
 *
 * A tab that fails to open stops the sequence but keeps the tab: `repo.error`
 * says what happened and All repositories marks it missing, because silently
 * dropping somebody's workspace is worse than showing them a repository they
 * need to find again.
 */
export async function resumeSession(port: ResumePort): Promise<void> {
	if (port.cancelled()) return;

	/*
	 * Every launch lands on Home (FEAT-140), which the author chose as the screen
	 * the window opens on. The tab and its selected commit still come back; the
	 * stored route is still where a tab switch goes, but not where a launch does.
	 */
	const land = async () => {
		if (!port.cancelled() && port.route() !== HOME_ROUTE) await port.goto(HOME_ROUTE);
	};

	const launch = await port.launchPath();
	if (launch) {
		await port.open(launch);
		await land();
		return;
	}

	const resume = port.active();
	if (!resume) {
		await land();
		return;
	}
	if (port.cancelled()) return;

	const place = port.placeOf(resume);
	const opened = await port.open(resume);
	if (port.cancelled()) return;

	await land();
	// After the route: the graph store holds a wanted id across a walk it cannot
	// see the end of, the same way a tab switch does.
	if (opened && place?.selected) port.want(place.selected);
}
