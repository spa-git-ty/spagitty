// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The sounds (FEAT-072).
 *
 * Synthesised, not sampled. Every noise Spagitty makes is a few oscillators and
 * a gain envelope built here at play time, which is a deliberate trade: nobody
 * ships an audio asset, nothing is decoded at startup, the bundle does not
 * grow, and a sound can be retuned by changing a number instead of by opening
 * a wave editor. The cost is that these are clicks and tones rather than
 * recordings, and everything below is written to suit that — short, mechanical,
 * and closer to a switch than to a fanfare.
 *
 * # Three rules this file will not break
 *
 * **Silence is the default.** `off` is what a fresh install has, and nothing
 * here creates an `AudioContext` until somebody has chosen otherwise. A
 * developer tool that makes a noise nobody asked for is a developer tool that
 * gets muted at the operating system, which loses the good sounds along with
 * the bad.
 *
 * **Never block, never throw.** A webview can refuse audio outright, and an
 * autoplay policy can leave the context suspended until a real gesture. Both
 * are ordinary, neither is worth a message, and a git client must not fail an
 * operation because it could not make a click.
 *
 * **Rarity is audible.** A legendary badge has a sound nothing else has, so
 * somebody across the room knows what happened without looking. That only works
 * if the common ones stay small.
 *
 * # The one that has to wear well
 *
 * The commit is heard more than everything else here put together — several
 * times an hour, for as long as somebody uses Spagitty. It is the only cue
 * chosen against that: pleasant on the ten-thousandth hearing beats striking on
 * the first, which is why it is a short rise rather than the mechanical click it
 * started as. The click moved to recovery, where it is rare enough to stay a
 * pleasure.
 */

import type { Rarity } from './badges';

/** How loud, if at all. `subtle` is the same set at a third of the level. */
export type SoundLevel = 'off' | 'subtle' | 'full';

/** What can be played. One entry per moment worth marking. */
export type Cue =
	| 'commit'
	| 'merge'
	| 'rebase'
	| 'conflict'
	| 'recovery'
	| 'notification'
	| Rarity;

/**
 * The context, built on first use and kept.
 *
 * One per process: contexts are a limited resource, and a browser that has run
 * out will refuse to make more — which would mean the sound working until it
 * quietly stopped.
 */
let context: AudioContext | null = null;
/** Swapped by the tests. Keeps this file free of a test-only branch. */
let makeContext: () => AudioContext | null = () => {
	const Ctor =
		typeof window === 'undefined'
			? undefined
			: (window.AudioContext ??
				(window as unknown as { webkitAudioContext?: typeof AudioContext })
					.webkitAudioContext);
	return Ctor ? new Ctor() : null;
};

/** Point the module at another context factory. Tests only. */
export function useContextFactory(factory: () => AudioContext | null): void {
	makeContext = factory;
	context = null;
}

function audio(): AudioContext | null {
	if (context) return context;
	try {
		context = makeContext();
	} catch {
		// No audio on this host. Everything below turns into a no-op.
		context = null;
	}
	return context;
}

/** Peak gain per level. `off` never reaches here. */
const LEVEL: Record<Exclude<SoundLevel, 'off'>, number> = {
	subtle: 0.06,
	full: 0.18
};

interface Tone {
	/** Hz. */
	from: number;
	/** Hz to glide to, or the same value for a flat tone. */
	to: number;
	/** Seconds from now that this tone starts. */
	delay: number;
	/** Seconds it lasts. */
	length: number;
	type: OscillatorType;
	/** Multiplier on the level's peak gain. */
	gain: number;
}

/**
 * What each cue is made of.
 *
 * The commit is the one to get right — it is the sound the application is meant
 * to be recognised by. It is two parts: a short bright click and a low body
 * under it, which together read as something mechanical locking into place
 * rather than as a beep.
 */
const CUES: Record<Cue, Tone[]> = {
	/*
	 * The commit: something faint arriving and steadying.
	 *
	 * This was the recovery cue, and the two were swapped because the commit is
	 * the sound the application is heard making most — several times an hour,
	 * for years — and a mechanical click wears out long before a rising figure
	 * does. A rise also matches what a commit *is*: not a latch closing, but a
	 * thing that now exists and did not before.
	 */
	commit: [
		{ from: 220, to: 440, delay: 0, length: 0.18, type: 'sine', gain: 0.5 },
		{ from: 660, to: 660, delay: 0.16, length: 0.12, type: 'sine', gain: 0.7 }
	],
	/*
	 * The merge: two lines converging, and the moment they touch.
	 *
	 * The first attempt was three tones in a row, which is what it sounded like
	 * — a little tune, and one that arrived after the merge rather than with it.
	 * A merge is not a sequence, it is a *meeting*, so this one is built the way
	 * the graph draws it: a voice climbing from below and a voice falling from
	 * above, both landing on the same note at the same instant, with a short
	 * bright tick and a low body exactly where they meet.
	 *
	 * The convergence is what carries it. Both glides end at 494Hz, the tick
	 * sits at that moment rather than after it, and the body underneath is what
	 * turns a chime into a *snap* — the difference between two notes agreeing
	 * and two objects closing.
	 */
	merge: [
		{ from: 294, to: 494, delay: 0, length: 0.1, type: 'triangle', gain: 0.55 },
		{ from: 831, to: 494, delay: 0, length: 0.1, type: 'triangle', gain: 0.45 },
		// The contact: a tick at the top and a body underneath, together.
		{ from: 1760, to: 1320, delay: 0.098, length: 0.018, type: 'square', gain: 0.3 },
		{ from: 165, to: 124, delay: 0.098, length: 0.13, type: 'sine', gain: 0.85 },
		// And the note they agreed on, held just long enough to be the answer.
		{ from: 494, to: 494, delay: 0.1, length: 0.16, type: 'sine', gain: 0.5 }
	],
	// A short rise that ends in the commit's own click: the history landed.
	rebase: [
		{ from: 300, to: 480, delay: 0, length: 0.06, type: 'triangle', gain: 0.5 },
		{ from: 480, to: 720, delay: 0.055, length: 0.06, type: 'triangle', gain: 0.55 },
		{ from: 1900, to: 1400, delay: 0.12, length: 0.02, type: 'square', gain: 0.35 }
	],
	// Tension coming off, not a success chime.
	conflict: [
		{ from: 700, to: 300, delay: 0, length: 0.16, type: 'sine', gain: 0.8 },
		{ from: 1400, to: 1400, delay: 0.14, length: 0.03, type: 'sine', gain: 0.3 }
	],
	/*
	 * The recovery: click — thunk. Something lost, put back where it belongs.
	 *
	 * The old commit sound, and it suits this better than it suited that. A
	 * latch closing is exactly the feeling of getting work back out of the
	 * reflog — the thing is *in place* again — and it is rare enough here that
	 * its mechanical edge stays a pleasure rather than becoming a tic.
	 */
	recovery: [
		{ from: 2100, to: 1500, delay: 0, length: 0.02, type: 'square', gain: 0.35 },
		{ from: 180, to: 110, delay: 0.012, length: 0.11, type: 'sine', gain: 1 }
	],

	/*
	 * A pull request has news (FEAT-114): a knock from somewhere else.
	 *
	 * Two soft notes a fifth apart, the second answering the first, then the
	 * commit's own 660Hz settling under them — so it reads as Spagitty from
	 * across the room, but as somebody else's doing rather than your own.
	 */
	notification: [
		{ from: 988, to: 988, delay: 0, length: 0.09, type: 'sine', gain: 0.55 },
		{ from: 1480, to: 1480, delay: 0.1, length: 0.14, type: 'sine', gain: 0.6 },
		{ from: 660, to: 660, delay: 0.1, length: 0.22, type: 'triangle', gain: 0.35 }
	],

	// Badges, by rarity. Tiny to unmistakable.
	common: [{ from: 1200, to: 1200, delay: 0, length: 0.03, type: 'sine', gain: 0.5 }],
	uncommon: [
		{ from: 900, to: 900, delay: 0, length: 0.04, type: 'triangle', gain: 0.6 },
		{ from: 1350, to: 1350, delay: 0.045, length: 0.05, type: 'triangle', gain: 0.6 }
	],
	rare: [
		{ from: 880, to: 880, delay: 0, length: 0.06, type: 'triangle', gain: 0.7 },
		{ from: 1320, to: 1320, delay: 0.06, length: 0.08, type: 'triangle', gain: 0.7 },
		{ from: 2640, to: 2640, delay: 0.07, length: 0.05, type: 'sine', gain: 0.25 }
	],
	epic: [
		{ from: 660, to: 660, delay: 0, length: 0.1, type: 'sine', gain: 0.8 },
		{ from: 990, to: 990, delay: 0.02, length: 0.12, type: 'sine', gain: 0.6 },
		{ from: 1320, to: 1320, delay: 0.11, length: 0.14, type: 'triangle', gain: 0.7 },
		{ from: 1980, to: 1980, delay: 0.13, length: 0.1, type: 'sine', gain: 0.3 }
	],
	// The one that should be recognised without looking at the screen.
	legendary: [
		{ from: 110, to: 110, delay: 0, length: 0.5, type: 'sine', gain: 0.9 },
		{ from: 440, to: 660, delay: 0.05, length: 0.3, type: 'triangle', gain: 0.5 },
		{ from: 880, to: 880, delay: 0.22, length: 0.24, type: 'sine', gain: 0.6 },
		{ from: 1320, to: 1320, delay: 0.34, length: 0.26, type: 'triangle', gain: 0.5 },
		{ from: 1760, to: 1760, delay: 0.46, length: 0.34, type: 'sine', gain: 0.4 }
	]
};

/**
 * Make a noise, or do nothing at all.
 *
 * Answers whether anything was scheduled, which is what the tests assert on —
 * there is no way to hear an oscillator from a test runner, but there is a way
 * to check that a silent setting stayed silent.
 */
export function play(cue: Cue, level: SoundLevel): boolean {
	if (level === 'off') return false;

	const ctx = audio();
	if (!ctx) return false;

	/*
	 * A suspended context has to be resumed *before* anything is scheduled.
	 *
	 * This was a bug, and a quiet one. An autoplay policy leaves a fresh context
	 * suspended, and a suspended context's `currentTime` is frozen at zero — so
	 * scheduling against it put every tone in the window `0…0.1s`. `resume()` is
	 * asynchronous, and by the time it had resolved the clock had moved past
	 * that window: the oscillators were started in the past and stopped in the
	 * past, and the very first sound the application ever tried to make was the
	 * one guaranteed not to be heard.
	 *
	 * So when it is suspended, the schedule waits for the resume and reads the
	 * clock again afterwards. Answering `true` here is honest: the sound has
	 * been arranged, it is simply one turn away.
	 */
	if (ctx.state === 'suspended') {
		// Wrapped, because `resume()` is specified to return a promise and an
		// older WebKit returns nothing at all. A missing return value must not
		// become a `TypeError` inside a click handler.
		Promise.resolve(ctx.resume?.())
			.then(() => schedule(ctx, cue, LEVEL[level]))
			.catch(() => {
				// The host refused to resume. Nothing to do, and nothing worth
				// interrupting somebody over.
			});
		return true;
	}

	return schedule(ctx, cue, LEVEL[level]);
}

/** Lay one cue's tones onto the context's clock, from now. */
function schedule(ctx: AudioContext, cue: Cue, peak: number): boolean {
	const start = ctx.currentTime;

	try {
		for (const tone of CUES[cue]) {
			const oscillator = ctx.createOscillator();
			const gain = ctx.createGain();

			oscillator.type = tone.type;
			oscillator.frequency.setValueAtTime(tone.from, start + tone.delay);
			if (tone.to !== tone.from) {
				oscillator.frequency.exponentialRampToValueAtTime(
					tone.to,
					start + tone.delay + tone.length
				);
			}

			// An envelope rather than a gate: a square wave switched on and off
			// at full gain clicks at both ends, and the click at the end is the
			// one that sounds like a fault.
			const top = peak * tone.gain;
			gain.gain.setValueAtTime(0.0001, start + tone.delay);
			gain.gain.exponentialRampToValueAtTime(top, start + tone.delay + 0.006);
			gain.gain.exponentialRampToValueAtTime(0.0001, start + tone.delay + tone.length);

			oscillator.connect(gain);
			gain.connect(ctx.destination);
			oscillator.start(start + tone.delay);
			oscillator.stop(start + tone.delay + tone.length + 0.02);
		}
		return true;
	} catch {
		// A context that has been closed under us, or a host that refuses.
		return false;
	}
}

/**
 * What the audio device is doing, in words.
 *
 * God mode shows this, and it exists because of what happens on Linux when it
 * is missing. WebKitGTK renders WebAudio through GStreamer, and the sink the
 * pipeline ends in — `autoaudiosink`, from `gst-plugins-good` — is a separate
 * package that a desktop can perfectly well be running without. When it is
 * absent the context reports itself `running`, every oscillator starts and
 * stops exactly as asked, and not one sample reaches the speakers.
 *
 * Nothing in the page can detect that: from JavaScript, silence and success
 * look identical. So this reports what *is* knowable and names the thing worth
 * checking first, rather than leaving somebody clicking a button that has
 * already done its job.
 */
export function status(): { supported: boolean; state: string; note: string } {
	const ctx = audio();

	if (!ctx) {
		return {
			supported: false,
			state: 'unavailable',
			note: 'This webview has no Web Audio at all. Nothing here will make a sound.'
		};
	}

	return {
		supported: true,
		state: ctx.state,
		note:
			ctx.state === 'running'
				? 'Audio is running. If it is still silent, the webview has no output sink — on Linux that is GStreamer: install gst-plugins-good and restart Spagitty.'
				: `The audio device is ${ctx.state}. It usually starts on the first click.`
	};
}

/** Drop the held context. Called when the application is torn down, and by tests. */
export function reset(): void {
	try {
		context?.close?.();
	} catch {
		// Already closed, or never opened.
	}
	context = null;
}
