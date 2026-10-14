// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * How old a blamed line is, said two ways (FEAT-131).
 *
 * In the gutter there is room for a few characters, so an age is a number and
 * a unit: `2 d`, `7 mo`. The peek has room for `relativeTime`'s sentence.
 * Beside every line there is also a bar, stronger the newer the line is
 * against the rest of the file, so where a file was touched lately can be
 * seen without reading a single age.
 */

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** An age in the fewest characters that still say it. */
export function shortAge(unixSeconds: number, now = Date.now()): string {
	const seconds = Math.max(0, Math.floor(now / 1000) - unixSeconds);
	if (seconds < 45) return 'now';
	if (seconds < HOUR) return `${Math.max(1, Math.round(seconds / MINUTE))} min`;
	if (seconds < DAY) return `${Math.round(seconds / HOUR)} h`;
	const days = Math.round(seconds / DAY);
	if (days < 30) return `${days} d`;
	if (days < 365) return `${Math.max(1, Math.round(days / 30))} mo`;
	return `${Math.max(1, Math.round(days / 365))} y`;
}

/** The faintest and the strongest a bar is drawn. Never nothing: every line has an age. */
export const BAR_MIN = 0.16;
export const BAR_MAX = 0.9;

/**
 * How strongly each commit's bar is drawn, by its age among the file's own
 * commits: the oldest at `BAR_MIN`, the newest at `BAR_MAX`. Measured against
 * the file rather than against today, so a file nobody has touched in three
 * years still shows which of its lines came last.
 */
export function strengths(times: readonly number[]): number[] {
	if (times.length === 0) return [];
	const oldest = Math.min(...times);
	const newest = Math.max(...times);
	const span = newest - oldest;
	return times.map((time) =>
		span === 0 ? BAR_MAX : BAR_MIN + ((time - oldest) / span) * (BAR_MAX - BAR_MIN)
	);
}
