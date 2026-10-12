// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What Spagitty is doing to the repository right now (FEAT-121).
 *
 * A pull, a stash or a checkout used to run with nothing on screen until it
 * ended: the button looked the same as before it was pressed, and pressing it
 * again started a second git that fought the first for `index.lock`. A fetch
 * and a push did say what they were doing, in a line of their own that
 * nothing else could use.
 *
 * This is the one place that knows. Whatever takes time says so here, and the
 * chrome reads it: the toolbar names what is running, the control that
 * started it shows the strands, and what would collide with it waits, dimmed,
 * with the reason. Nothing else is held: the graph scrolls, a diff opens and
 * another screen can be walked to while git works.
 *
 * What collides is decided by lane rather than by "anything is running". An
 * operation holds the lanes it writes to:
 *
 * - `repository`: the working copy, the index and the refs. A checkout, a
 *   stash, a reset, a commit.
 * - `network`: the one worker that talks to a remote. A fetch, a push.
 *
 * A pull holds both. So a fetch does not stop a stash, and a stash does not
 * stop a push, and two things that would both take `index.lock` cannot start.
 */

export type Lane = 'repository' | 'network';

/** What kind of thing is running, for the control that starts that kind. */
export type ActivityKind =
	| 'pull'
	| 'fetch'
	| 'push'
	| 'stash'
	| 'checkout'
	| 'branch'
	| 'commit'
	| 'merge'
	| 'other';

interface Running {
	id: number;
	kind: ActivityKind;
	/** What it is, with no trailing dots: `Pulling`, `Switching to main`. */
	what: string;
	/** git's own progress line, when it has given one. */
	detail: string | null;
	lanes: readonly Lane[];
}

/** One running operation, held by whoever started it. */
export interface Step {
	/** It has moved on to another part: `Putting your changes back`. */
	say(what: string): void;
	/** git's own words about where it has got to, or null for none. */
	progress(detail: string | null): void;
	/** It is over, however it went. Safe to call twice. */
	end(): void;
}

/** Refused because something that would collide is still running. */
export class ActivityBusy extends Error {
	constructor(reason: string) {
		super(reason);
		this.name = 'ActivityBusy';
	}

	/** The sentence alone: stores show a failure as `String(error)`. */
	override toString(): string {
		return this.message;
	}
}

const REPOSITORY: readonly Lane[] = ['repository'];

let running = $state<Running[]>([]);
let nextId = 0;

function holder(lanes: readonly Lane[]): Running | null {
	return running.find((one) => one.lanes.some((lane) => lanes.includes(lane))) ?? null;
}

export const activity = {
	/** Something is running. */
	get running(): boolean {
		return running.length > 0;
	},

	/**
	 * What to show while it runs: git's progress when there is some, the name
	 * of the operation otherwise. The newest, when two lanes are both taken.
	 */
	get label(): string {
		const latest = running[running.length - 1];
		if (!latest) return '';
		return latest.detail ?? `${latest.what}…`;
	},

	/** Whether an operation of this kind is the one running. */
	is(kind: ActivityKind): boolean {
		return running.some((one) => one.kind === kind);
	},

	/**
	 * Why something that writes to `lanes` has to wait, or null when it can
	 * start. The sentence names what is in the way.
	 */
	blocked(...lanes: Lane[]): string | null {
		const inTheWay = holder(lanes.length > 0 ? lanes : REPOSITORY);
		return inTheWay ? `${inTheWay.what} is still running` : null;
	},

	/**
	 * Take the lanes and say what is running. Null when one of them is taken:
	 * the caller says why with `blocked`, and starts nothing.
	 */
	begin(kind: ActivityKind, what: string, lanes: readonly Lane[] = REPOSITORY): Step | null {
		if (holder(lanes)) return null;

		const id = ++nextId;
		running = [...running, { id, kind, what, detail: null, lanes }];

		const change = (patch: Partial<Running>) => {
			running = running.map((one) => (one.id === id ? { ...one, ...patch } : one));
		};

		return {
			say: (next) => change({ what: next, detail: null }),
			progress: (detail) => change({ detail }),
			end: () => {
				running = running.filter((one) => one.id !== id);
			}
		};
	},

	/**
	 * Run `work` as a named operation. The lanes are let go however it ends.
	 * Rejects with `ActivityBusy`, and runs nothing, when a lane is taken.
	 */
	async run<T>(
		kind: ActivityKind,
		what: string,
		work: () => Promise<T>,
		lanes: readonly Lane[] = REPOSITORY
	): Promise<T> {
		const step = this.begin(kind, what, lanes);
		if (!step) throw new ActivityBusy(this.blocked(...lanes) ?? 'Something else is still running');
		try {
			return await work();
		} finally {
			step.end();
		}
	},

	/** For tests: nothing is running. */
	reset(): void {
		running = [];
	}
};
