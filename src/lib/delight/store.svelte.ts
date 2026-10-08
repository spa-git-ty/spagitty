// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The delight layer's state (FEAT-072).
 *
 * Sits between the pure engine and everything that can be seen or heard:
 *
 * ```
 *   actions ──► delight.record(event) ──► engine ──► unlocked
 *                      │                                │
 *                      ├──► sound cue                   ▼
 *                      └──► pulse                    queue ──► RewardOverlay
 * ```
 *
 * # Why it holds no reference to the repository store
 *
 * It is *told* which repository it belongs to, by the shell, rather than
 * reading `repo`. Almost every module that reports a git operation is reachable
 * from `repo.svelte` in one direction or another, and importing it back from
 * here would close a cycle through half the frontend — the kind that works
 * until the day module evaluation order changes and a store is `undefined` at
 * first use.
 *
 * # Why the record is per repository, in `localStorage`
 *
 * Per repository because that is the only scope in which the numbers mean
 * anything: "which agent does well here" is a question about *this* codebase,
 * and a global tally would average a Rust workspace against somebody's dotfiles
 * and call the result a reputation.
 *
 * In `localStorage` because the delight layer must stay isolated from the git
 * layers (see the design document's engine section), and reaching into
 * `src-tauri` for a preferences file would put a badge write on the same path
 * as a settings write. It sits beside the theme and the column layouts, which
 * are stored the same way for the same reason.
 *
 * A record that is lost is a record of badges, not of work. That is the trade,
 * and it is the right way round: nothing here is ever the authority on what
 * happened in a repository — git is.
 */

import { BADGES, badge, titleable, type Badge } from './badges';
import { emptyRecord, record as fold, type ActorRecord } from './engine';
import type { ActorRef, DelightEvent } from './events';
import { play, type Cue } from './sound';
import { settings } from '$lib/settings/store.svelte';
import type { Personality } from '$lib/types';
import { notice } from '$lib/ui/notice.svelte';

const PREFIX = 'spagitty.delight:';

/** The stored shape. Versioned so a later change can migrate rather than guess. */
interface Stored {
	version: 1;
	actors: Record<string, ActorRecord>;
}

/** Who a git operation is attributed to when nothing says otherwise. */
const YOU: ActorRef = { id: 'you', kind: 'human', name: 'You' };

/** Which noise an event makes, before the level is consulted. */
const CUES: Partial<Record<DelightEvent['kind'], Cue>> = {
	commit: 'commit',
	merge: 'merge',
	rebase: 'rebase',
	conflict: 'conflict',
	recovery: 'recovery'
};

let path = $state<string | null>(null);
let actors = $state<Record<string, ActorRecord>>({});
/** The human at the keyboard, once the identity is known. */
let me = $state<ActorRef>(YOU);
/** Unlocks waiting to be shown, oldest first. */
let queue = $state<{ badge: Badge; who: string }[]>([]);
/** What the overlay is showing now, if anything. */
let showing = $state<{ badge: Badge; who: string } | null>(null);
/**
 * Bumped on every event that deserves a visible acknowledgement.
 *
 * The overlay watches it rather than the event, so the signature commit pulse
 * is one number changing instead of a second event channel.
 */
let pulse = $state(0);

/** The personality, honouring the default before the settings read lands. */
function personality(): Personality {
	return settings.settings.personality;
}

function persist(): void {
	if (path === null) return;
	try {
		const stored: Stored = { version: 1, actors };
		localStorage.setItem(PREFIX + path, JSON.stringify(stored));
	} catch {
		// Storage full, or disabled. Losing a badge is not worth an error in
		// front of somebody who was doing something else.
	}
}

/** Read a stored record, ignoring anything that is not one. */
function read(key: string): Record<string, ActorRecord> {
	try {
		const raw = localStorage.getItem(PREFIX + key);
		if (!raw) return {};
		const parsed = JSON.parse(raw) as Partial<Stored>;
		if (parsed?.version !== 1 || typeof parsed.actors !== 'object' || !parsed.actors) return {};
		return parsed.actors;
	} catch {
		return {};
	}
}

/** The record for an actor, created empty the first time they do anything. */
function recordFor(actor: ActorRef): ActorRecord {
	const existing = actors[actor.id];
	if (existing) return existing;
	return emptyRecord(actor);
}

export const delight = {
	get repository(): string | null {
		return path;
	},

	/** Every actor with a record here, the person first and then the agents. */
	get list(): ActorRecord[] {
		return Object.values(actors).sort((a, b) => {
			if (a.kind === 'human' && b.kind !== 'human') return -1;
			if (b.kind === 'human' && a.kind !== 'human') return 1;
			return b.stats.commits + b.stats.tasks - (a.stats.commits + a.stats.tasks);
		});
	},

	/** The human at the keyboard. */
	get me(): ActorRecord {
		return recordFor(me);
	},

	get showing(): { badge: Badge; who: string } | null {
		return showing;
	},

	get waiting(): number {
		return queue.length;
	},

	get pulse(): number {
		return pulse;
	},

	/**
	 * Whether the layer shows anything at all (TASK-045).
	 *
	 * Off is the default. Badges are still recorded while it is off, so turning
	 * it on shows what was earned in the meantime rather than starting from
	 * nothing.
	 */
	get on(): boolean {
		return personality() !== 'off';
	},

	/** The Hall of Shame is off in Off and Professional; badges are still recorded. */
	get showsShame(): boolean {
		return this.on && personality() !== 'professional';
	},

	/** A reward moment interrupts nothing, but Off and Professional do not want one. */
	get showsRewardMoment(): boolean {
		return this.on && personality() !== 'professional';
	},

	/** Jokes, easter eggs and shame notices. The top level only. */
	get showsJokes(): boolean {
		return personality() === 'fullSpagitty';
	},

	get(id: string): ActorRecord | null {
		return actors[id] ?? null;
	},

	/**
	 * Point the layer at a repository, loading its record.
	 *
	 * Called by the shell when the open repository changes. Re-binding to the
	 * same path is a no-op rather than a reload, so a refresh does not throw
	 * away a queue that has not been shown yet.
	 */
	bind(next: string | null): void {
		if (next === path) return;
		path = next;
		actors = next === null ? {} : read(next);
		queue = [];
		showing = null;
	},

	/**
	 * Say who the person at the keyboard is.
	 *
	 * Their git email is the key, so the record follows the identity rather than
	 * the machine — someone switching profiles (FEAT-069) switches record too,
	 * which is the honest behaviour when the profiles exist to separate work.
	 */
	identify(name: string | null, email: string | null): void {
		me = email
			? { id: email.toLowerCase(), kind: 'human', name: name?.trim() || email }
			: YOU;
	},

	/**
	 * Count something that happened, and award whatever it earns.
	 *
	 * Never throws. Every caller is a line after a successful git operation, and
	 * an achievement engine that could fail a commit would be the single worst
	 * thing this feature could do.
	 */
	record(event: DelightEvent, actor: ActorRef = me): Badge[] {
		try {
			return this.apply(event, actor);
		} catch {
			return [];
		}
	},

	/** The body of `record`, split out so the guard above stays one line. */
	apply(event: DelightEvent, actor: ActorRef): Badge[] {
		const before = recordFor(actor);
		const { record: after, unlocked } = fold(before, event, Date.now());

		actors = { ...actors, [actor.id]: after };
		persist();

		// Sound is its own setting: the personality decides what is shown, not
		// what is heard, so Off with sound chosen still plays the cue.
		const cue = CUES[event.kind];
		if (cue) play(cue, settings.settings.sound);
		if (cue && this.on) pulse += 1;

		for (const found of unlocked) this.announce(found, after.name);

		return unlocked;
	},

	/**
	 * Put an unlock in front of the user, in the way this personality allows.
	 *
	 * A shame badge never gets a reward moment. Celebrating somebody for
	 * committing to main with the same animation that celebrates a recovery is
	 * the joke landing on the wrong person; at the top level it gets a one-line
	 * notice, and below that it is recorded and left for the badge screen.
	 */
	announce(found: Badge, who: string): void {
		// Off: recorded, and that is all.
		if (!this.on) return;

		if (found.shame) {
			if (this.showsJokes) notice.ok(`${found.emoji} ${found.name}`, found.line);
			return;
		}

		if (!this.showsRewardMoment) {
			// Professional: said once, in the corner, with everything else.
			notice.ok(`${found.emoji} ${found.name}`, `${found.rarity} badge earned`);
			return;
		}

		queue = [...queue, { badge: found, who }];
	},

	/** Take the next unlock off the queue. The overlay drives this. */
	advance(): void {
		const [next, ...rest] = queue;
		showing = next ?? null;
		queue = rest;
		if (next) play(next.badge.rarity, settings.settings.sound);
	},

	/** Put the current reward moment away. */
	dismiss(): void {
		showing = null;
	},

	/**
	 * Equip a badge as an actor's title.
	 *
	 * Refused for a badge they have not earned and for a shame badge — a title
	 * is what somebody chooses to be known by, and the Hall of Shame is not a
	 * wardrobe.
	 */
	equip(actorId: string, badgeId: string | null): void {
		const current = actors[actorId];
		if (!current) return;
		if (badgeId !== null) {
			const allowed = titleable(current.earned.map((entry) => entry.id));
			if (!allowed.some((found) => found.id === badgeId)) return;
		}
		actors = { ...actors, [actorId]: { ...current, title: badgeId } };
		persist();
	},

	/** What an actor's equipped title draws as, or null. */
	title(actorId: string): Badge | null {
		const current = actors[actorId];
		return current?.title ? badge(current.title) : null;
	},

	// --- God mode ----------------------------------------------------------
	//
	// A testing surface, and it is kept here rather than in the section that
	// draws it for one reason: these are the only writes in the application
	// that bypass the engine. Anything that can award a badge nobody earned
	// belongs beside the code that awards the ones people did, where it can be
	// read against it — not scattered through a settings component.

	/**
	 * Show a badge's reward moment without earning it.
	 *
	 * Deliberately ignores the personality. The personality governs *automatic*
	 * announcements — the ones that arrive while somebody is working. Pressing a
	 * button labelled "preview this badge" is asking for the card, and a preview
	 * that silently did nothing at Professional would look broken.
	 */
	preview(id: string): void {
		const found = badge(id);
		if (!found) return;
		queue = [...queue, { badge: found, who: this.me.name }];
	},

	/** Award a badge outright, engine or no engine. */
	grant(id: string, actorId: string = me.id): void {
		if (!badge(id)) return;
		const current = actors[actorId] ?? emptyRecord(me);
		if (current.earned.some((entry) => entry.id === id)) return;

		actors = {
			...actors,
			[actorId]: { ...current, earned: [...current.earned, { id, at: Date.now() }] }
		};
		persist();
	},

	/** Take a badge back. The one operation the engine itself will never do. */
	revoke(id: string, actorId: string = me.id): void {
		const current = actors[actorId];
		if (!current) return;

		actors = {
			...actors,
			[actorId]: {
				...current,
				earned: current.earned.filter((entry) => entry.id !== id),
				// A title pointing at a badge that is gone would draw as nothing.
				title: current.title === id ? null : current.title
			}
		};
		persist();
	},

	/** Award the whole catalogue, so the badge screen can be seen full. */
	grantEvery(actorId: string = me.id): void {
		for (const found of BADGES) this.grant(found.id, actorId);
	},

	/**
	 * Put three agents in this repository with a plausible history.
	 *
	 * Fed as real events through the real engine rather than written as stats,
	 * so what the standings table then shows is what the standings table would
	 * show — a seeded number that the rules could never have produced would
	 * make this a worse testing tool than no testing tool.
	 */
	seedAgents(): void {
		const agents: { actor: ActorRef; tasks: number; clean: number; green: number }[] = [
			{ actor: { id: 'claude', kind: 'claude', name: 'Claude' }, tasks: 34, clean: 31, green: 33 },
			{ actor: { id: 'gpt', kind: 'gpt', name: 'GPT' }, tasks: 29, clean: 27, green: 28 },
			{ actor: { id: 'codex', kind: 'codex', name: 'Codex' }, tasks: 41, clean: 36, green: 40 }
		];

		for (const { actor, tasks, clean, green } of agents) {
			for (let i = 0; i < tasks; i += 1) {
				const first = i < clean;
				this.record(
					{
						kind: 'agentTask',
						testsPassed: i < green,
						approved: first,
						corrections: first ? 0 : 2,
						difficulty: i % 7 === 0 ? 'hard' : 'routine',
						handoff: i % 11 === 0,
						failedElsewhere: false
					},
					actor
				);
			}
		}
	},

	/** Forget this repository's record. Asked for from the badge screen. */
	forget(): void {
		actors = {};
		queue = [];
		showing = null;
		if (path !== null) {
			try {
				localStorage.removeItem(PREFIX + path);
			} catch {
				// Nothing to do about it, and nothing worth saying.
			}
		}
	},

	/** Forget everything, without touching storage. For tests and teardown. */
	clear(): void {
		path = null;
		actors = {};
		me = YOU;
		queue = [];
		showing = null;
		pulse = 0;
	}
};
