// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merger's plan (FEAT-100), as plain functions of the forecast.
 *
 * The backend works out one dry run per pair of branches, A merged with B.
 * Everything the plan says about a direction and a strategy — which card
 * receives, which arrow is solid, the sentence in the Result card, which
 * strategies are possible — is derived here, so changing the direction or the
 * strategy never asks git anything again, and the rules can be tested without
 * mounting a screen.
 */

import type { MergerFile, MergerForecast, MergerSide } from '../types';

/** Where the result lands. */
export type Into = 'a' | 'b' | 'new';

/** How it lands. */
export type Strategy = 'merge' | 'squash' | 'rebase' | 'ff';

export type SideKey = 'a' | 'b';

export const STRATEGIES: Strategy[] = ['merge', 'squash', 'rebase', 'ff'];

/** Who gives and who receives, for one direction. */
export interface Roles {
	/** The side whose commits come in. */
	source: SideKey;
	/**
	 * The side the result is built on: the receiving branch, or A when the
	 * result is a new branch, which starts from A.
	 */
	onto: SideKey;
	isNew: boolean;
	/** What the receiving branch is called. */
	targetName: string;
	sourceName: string;
}

export function roles(forecast: MergerForecast, into: Into, newName: string): Roles {
	const onto: SideKey = into === 'b' ? 'b' : 'a';
	const source: SideKey = onto === 'a' ? 'b' : 'a';
	return {
		source,
		onto,
		isNew: into === 'new',
		targetName: into === 'new' ? newName.trim() || 'new-branch' : forecast[onto].name,
		sourceName: forecast[source].name
	};
}

/** `merge/main-tab-drag` for `main` and `feat/tab-drag`: the last part of each. */
export function defaultNewName(a: string, b: string): string {
	const last = (name: string) => name.slice(name.lastIndexOf('/') + 1) || name;
	return `merge/${last(a)}-${last(b)}`;
}

export function plural(count: number, one: string, many = `${one}s`): string {
	return `${count} ${count === 1 ? one : many}`;
}

/**
 * Why the result cannot land where it was asked to, or null.
 *
 * Only a branch here can receive one: a remote-tracking branch moves when it
 * is fetched, and a tag or a commit is not something a merge moves at all.
 */
export function cannotLand(
	forecast: MergerForecast,
	into: Into,
	newName: string,
	existing: readonly string[]
): string | null {
	if (into === 'new') {
		const name = newName.trim();
		if (!name) return 'Name the new branch';
		if (existing.includes(name)) return `${name} already exists`;
		return null;
	}
	const side = forecast[into];
	if (side.kind === 'remote') return `${side.name} is a remote branch; merge into a new branch instead`;
	if (side.kind !== 'local') return `${side.name} is not a branch; merge into a new branch instead`;
	return null;
}

/** Commits that come in from the source, for this direction. */
export function incoming(forecast: MergerForecast, who: Roles): number {
	return forecast[who.source].ahead;
}

/** The receiving side already has everything the source has. */
export function nothingComesIn(forecast: MergerForecast, who: Roles): boolean {
	return incoming(forecast, who) === 0;
}

/** Fast-forward is possible: what it lands on is in the source's history. */
export function canFastForward(forecast: MergerForecast, who: Roles): boolean {
	return who.onto === 'a' ? forecast.bHasA : forecast.aHasB;
}

export interface StrategyChoice {
	id: Strategy;
	label: string;
	description: string;
	/** Why it cannot be chosen, or null. */
	reason: string | null;
}

export function strategies(forecast: MergerForecast, who: Roles): StrategyChoice[] {
	const target = who.targetName;
	const source = who.sourceName;
	const none = nothingComesIn(forecast, who)
		? `Nothing comes in: ${target} already has everything in ${source}.`
		: null;
	return [
		{
			id: 'merge',
			label: 'Merge commit',
			description: 'Keeps both histories and ties them with one new commit.',
			reason: none
		},
		{
			id: 'squash',
			label: 'Squash',
			description: `Everything ${source} changed, as one new commit on ${target}.`,
			reason: none
		},
		{
			id: 'rebase',
			label: 'Rebase, then fast-forward',
			description: `Replays ${source}’s commits on top of ${target}. A straight line, new hashes.`,
			reason: none
		},
		{
			id: 'ff',
			label: 'Fast-forward only',
			description: canFastForward(forecast, who)
				? `${target} moves forward to ${source}. No new commit.`
				: 'Not possible here: both branches have commits the other lacks.',
			reason:
				none ??
				(canFastForward(forecast, who) ? null : 'Both branches have commits the other lacks')
		}
	];
}

/** The strategy to show: the one asked for, or the first that is possible. */
export function effectiveStrategy(choices: StrategyChoice[], asked: Strategy): Strategy {
	const wanted = choices.find((choice) => choice.id === asked);
	if (wanted && !wanted.reason) return asked;
	return choices.find((choice) => !choice.reason)?.id ?? asked;
}

/** The files whose content on the target changes. */
export function changing(forecast: MergerForecast, who: Roles): MergerFile[] {
	return forecast.files.filter(
		(file) => file.touch === 'conflict' || file.touch === 'both' || file.touch === who.source
	);
}

/** The Result card's one plain sentence. */
export function sentence(forecast: MergerForecast, who: Roles, strategy: Strategy): string {
	const target = who.targetName;
	const source = who.sourceName;
	const count = incoming(forecast, who);
	if (count === 0) return `${target} already has everything in ${source}. Nothing to merge.`;
	const commits = plural(count, 'commit');
	const pre = who.isNew ? `A new branch starts from ${forecast.a.name}. ` : '';
	switch (strategy) {
		case 'merge':
			return `${pre}${target} gets ${commits} from ${source}, tied together by one merge commit. ${source} stays as it is.`;
		case 'squash':
			return `${pre}${target} gets one new commit holding everything ${source} changed. Its ${commits} ${count === 1 ? 'is' : 'are'} not carried over.`;
		case 'rebase':
			return `${pre}${source}’s ${commits} ${count === 1 ? 'is' : 'are'} replayed on top of ${target} as new ${count === 1 ? 'commit' : 'commits'}, then ${target} moves forward to ${count === 1 ? 'it' : 'them'}. No merge commit.`;
		case 'ff':
			return `${pre}${target} moves forward to ${source}, taking its ${commits}. No new commit.`;
	}
}

/** The three numbers under the sentence. */
export function stats(
	forecast: MergerForecast,
	who: Roles,
	strategy: Strategy
): { value: number; label: string }[] {
	const count = incoming(forecast, who);
	const files = count === 0 ? 0 : changing(forecast, who).length;
	const filesChange = { value: files, label: files === 1 ? 'file changes' : 'files change' };
	if (count === 0) {
		return [{ value: 0, label: 'commits come in' }, { value: 0, label: 'new commits' }, filesChange];
	}
	switch (strategy) {
		case 'merge':
			return [{ value: count, label: count === 1 ? 'commit comes in' : 'commits come in' }, { value: 1, label: 'merge commit' }, filesChange];
		case 'squash':
			return [{ value: 1, label: 'new commit' }, { value: count, label: count === 1 ? 'commit folded in' : 'commits folded in' }, filesChange];
		case 'rebase':
			return [{ value: count, label: count === 1 ? 'commit replayed' : 'commits replayed' }, { value: 0, label: 'merge commits' }, filesChange];
		case 'ff':
			return [{ value: count, label: count === 1 ? 'commit comes in' : 'commits come in' }, { value: 0, label: 'new commits' }, filesChange];
	}
}

/** The conflict box: its title, the clean count, and where it was measured. */
export function conflictSummary(
	forecast: MergerForecast,
	who: Roles,
	strategy: Strategy
): { title: string; clean: string; note: string; files: number } {
	const files = forecast.files.filter((file) => file.touch === 'conflict').length;
	const clean = forecast.files.length - files;
	const where = `a dry run against ${forecast.baseShort}, where the two branches split`;
	if (forecast.conflicts === 0) {
		return {
			title: 'No conflicts',
			clean: `${plural(clean, 'file')} merge on their own`,
			note: `Found by ${where}.`,
			files
		};
	}
	const stops = Math.max(1, forecast[who.source].touching);
	return {
		title: `${plural(forecast.conflicts, 'conflict')} in ${plural(files, 'file')}`,
		clean: `${plural(clean, 'file')} merge on their own`,
		note:
			strategy === 'rebase'
				? `Rebase applies one commit at a time, so it may stop on up to ${plural(stops, 'commit')}. The regions are the same ${forecast.conflicts}.`
				: `Found by ${where}.`,
		files
	};
}

/**
 * The meter under the conflict title: one segment per file, red for a
 * conflict and green for one that merges, at most `cap` of them, in
 * proportion when there are more files than that.
 */
export function meter(forecast: MergerForecast, cap = 24): boolean[] {
	const total = forecast.files.length;
	if (total === 0) return [];
	const conflicted = forecast.files.filter((file) => file.touch === 'conflict').length;
	const segments = Math.min(total, cap);
	let red = Math.round((conflicted / total) * segments);
	if (conflicted > 0 && red === 0) red = 1;
	return Array.from({ length: segments }, (_, index) => index < red);
}

export type Role = 'lands' | 'comes-in' | 'start' | 'comes-in-new';

/** What a branch card is, in this direction. */
export function cardRole(key: SideKey, who: Roles): { role: Role; label: string } {
	if (who.isNew) {
		return key === 'a'
			? { role: 'start', label: 'Starting point' }
			: { role: 'comes-in-new', label: 'Comes in' };
	}
	return key === who.onto
		? { role: 'lands', label: 'Lands here' }
		: { role: 'comes-in', label: 'Comes in · stays as is' };
}

/** The arrow from a card into the Result. */
export function connector(
	forecast: MergerForecast,
	key: SideKey,
	who: Roles
): { solid: boolean; label: string } {
	if (key === who.source) {
		const count = forecast[key].ahead;
		return { solid: true, label: `${plural(count, 'commit')} ${count === 1 ? 'comes' : 'come'} in` };
	}
	return { solid: false, label: who.isNew ? 'starts from' : 'continues as' };
}

/** `4 commits since they split at 29c36a1`. */
export function sinceSplit(side: MergerSide, baseShort: string): string {
	if (side.ahead === 0) return `Nothing since they split at ${baseShort}`;
	return `${plural(side.ahead, 'commit')} since they split at ${baseShort}`;
}

export type FileTone = 'danger' | 'ok' | 'a' | 'b' | 'muted';

/** One row of What changes: its chip and colour. */
export function fileStatus(
	file: MergerFile,
	forecast: MergerForecast,
	who: Roles
): { label: string; tone: FileTone; dot: FileTone } {
	if (file.touch === 'conflict') {
		return { label: plural(file.conflicts, 'conflict'), tone: 'danger', dot: 'danger' };
	}
	if (file.touch === 'both') return { label: 'Both changed · merges cleanly', tone: 'ok', dot: 'ok' };
	const side = forecast[file.touch];
	const added = file.touch === 'a' ? file.addedByA : file.addedByB;
	const deleted = file.touch === 'a' ? file.deletedByA : file.deletedByB;
	if (file.touch === who.source) {
		const label = added
			? `New file from ${side.name}`
			: deleted
				? `Deleted on ${side.name}`
				: `Comes in from ${side.name}`;
		return { label, tone: file.touch, dot: file.touch };
	}
	return { label: `Already on ${side.name} · untouched`, tone: 'muted', dot: file.touch };
}

/** `7 files touched by either branch · 6 change on main`. */
export function filesLine(forecast: MergerForecast, who: Roles): string {
	const changes = changing(forecast, who).length;
	return `${plural(forecast.files.length, 'file')} touched by either branch · ${changes} ${changes === 1 ? 'changes' : 'change'} on ${who.targetName}`;
}

/** The primary action under the forecast. */
export function primaryAction(
	forecast: MergerForecast,
	strategy: Strategy
): { kind: 'resolve'; label: string } | { kind: 'merge'; label: string } {
	if (forecast.conflicts > 0 && strategy !== 'ff') {
		return { kind: 'resolve', label: `Resolve ${plural(forecast.conflicts, 'conflict')}` };
	}
	return { kind: 'merge', label: 'Merge now' };
}

/** `after the merge`, beside the target's name in the Result card. */
export function afterLabel(strategy: Strategy): string {
	switch (strategy) {
		case 'merge':
			return 'after the merge';
		case 'squash':
			return 'after the squash';
		case 'rebase':
			return 'after the rebase';
		case 'ff':
			return 'after the fast-forward';
	}
}

/** One conflict in the commit dialog, with what was chosen for it. */
export interface SummaryRow {
	where: string;
	label: string;
	badges: SideKey[];
	/** Typed by hand: drawn in the hand-edit colour. */
	mine: boolean;
	/** Who chose it (2.0): `you`, `Codex, accepted by you`, `Codex, unattended`. */
	who?: string;
}

/** Split a path into the folder, drawn muted, and the name. */
export function splitPath(path: string): { dir: string; name: string } {
	const cut = path.lastIndexOf('/') + 1;
	return { dir: path.slice(0, cut), name: path.slice(cut) };
}
