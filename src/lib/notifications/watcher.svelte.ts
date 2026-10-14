// SPDX-License-Identifier: GPL-3.0-or-later

import * as api from '$lib/api';
import { play } from '$lib/delight/sound';
import { settings } from '$lib/settings/store.svelte';
import { describe, notice } from '$lib/ui/notice.svelte';
import type { Watched } from '$lib/types';
import { changes, type News } from './changes';

/**
 * The pull request watcher (FEAT-114).
 *
 * Reads every connected account's pull requests on a timer while Spagitty is
 * open, compares the answer with the last one, and says what changed: in the
 * corner, in the operating system's notification centre when asked, and with
 * Spagitty's own `notification` cue at the chosen sound level.
 *
 * The last answer is kept across restarts, so a pull request merged while
 * Spagitty was closed is announced on the next start. It is forgotten when the
 * watcher is switched off, so switching it back on starts from now.
 */

const SEEN = 'spagitty.notifications.seen';
/** Past this, the rest are one summary rather than a wall of notifications. */
const MAX_DESKTOP = 3;

let timer: ReturnType<typeof setInterval> | null = null;
let reading = false;
let lastError = $state<string | null>(null);
let lastChecked = $state<number | null>(null);
let notificationEvery: number | null = null;
let nextNotification = 0;
const subscribers = new Map<symbol, { every: number; next: number; read: () => void | Promise<void> }>();

/** One forge clock, also used by open forge screens. Turning notifications
 * off stops their reads while subscribed screens retain their own refresh. */
function schedule(): void {
	if (timer) clearInterval(timer);
	timer = null;
	const periods = [...subscribers.values()].map((subscriber) => subscriber.every);
	if (notificationEvery !== null) periods.push(notificationEvery);
	if (!periods.length) return;
	timer = setInterval(() => {
		if (typeof document !== 'undefined' && document.visibilityState === 'hidden') return;
		const now = Date.now();
		if (notificationEvery !== null && now >= nextNotification) {
			nextNotification = now + notificationEvery; void check();
		}
		for (const subscriber of subscribers.values()) {
			if (now >= subscriber.next) {
				subscriber.next = now + subscriber.every;
				void Promise.resolve().then(subscriber.read).catch(() => {});
			}
		}
	}, Math.min(...periods));
}

function readSeen(): Record<string, Watched> | null {
	try {
		const text = localStorage.getItem(SEEN);
		return text ? (JSON.parse(text) as Record<string, Watched>) : null;
	} catch {
		return null;
	}
}

function writeSeen(rows: Watched[]): void {
	try {
		localStorage.setItem(SEEN, JSON.stringify(Object.fromEntries(rows.map((r) => [r.key, r]))));
	} catch {
		// Storage refused. The next start compares with nothing and stays quiet,
		// which is the safe way to be wrong.
	}
}

/** Say it: one notice, one sound, and the operating system when asked. */
function announce(news: News[]): void {
	if (!news.length) return;
	const latest = news[news.length - 1];
	const more = news.length > 1 ? ` (+${news.length - 1} more)` : '';
	notice.ok(`${latest.title}${more}`, latest.body);
	play('notification', settings.settings.sound);

	if (!settings.settings.notifyDesktop) return;
	for (const item of news.slice(0, MAX_DESKTOP)) {
		void api.notifyDesktop(item.title, item.body).catch(() => {});
	}
	if (news.length > MAX_DESKTOP) {
		const rest = news.length - MAX_DESKTOP;
		void api
			.notifyDesktop(`${rest} more pull request ${rest === 1 ? 'update' : 'updates'}`, 'In Spagitty')
			.catch(() => {});
	}
}

/** Read once, compare, announce. One read at a time. */
async function check(): Promise<News[]> {
	if (reading) return [];
	reading = true;
	try {
		const now = await api.watchPullRequests();
		const found = changes(readSeen(), now, settings.settings);
		writeSeen(now);
		lastError = null;
		lastChecked = Date.now();
		announce(found);
		return found;
	} catch (error) {
		// Offline, rate limited, a token gone: said on the Settings section,
		// never in the corner on a timer.
		lastError = describe(error);
		return [];
	} finally {
		reading = false;
	}
}

function stop(): void {
	notificationEvery = null;
	schedule();
}

export const watcher = {
	get lastError(): string | null {
		return lastError;
	},
	get lastChecked(): number | null {
		return lastChecked;
	},

	/** Look now, then every `minutes`. Restarting replaces the timer. */
	start(minutes: number): void {
		notificationEvery = Math.max(1, minutes) * 60_000;
		nextNotification = Date.now() + notificationEvery;
		void check();
		schedule();
	},

	subscribe(read: () => void | Promise<void>, every = 60_000): () => void {
		const key = Symbol(); every = Math.max(5000, every);
		subscribers.set(key, { read, every, next: Date.now() + every }); schedule();
		return () => { subscribers.delete(key); schedule(); };
	},

	stop,

	/** Switched off: stop, and forget the last answer so switching on starts from now. */
	forget(): void {
		stop();
		lastError = null;
		lastChecked = null;
		try {
			localStorage.removeItem(SEEN);
		} catch {
			// Nothing to forget.
		}
	},

	check,

	/** What a notification looks and sounds like, from the Settings section. */
	test(): void {
		announce([
			{
				kind: 'merged',
				pr: {} as Watched,
				title: 'Your pull request was merged',
				body: 'This is how Spagitty will tell you'
			}
		]);
	}
};
