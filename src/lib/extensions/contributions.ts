// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Which contributions show where, and why one cannot run (FEAT-096).
 *
 * Pure functions over the host's listing, so the rules are tested as a table
 * rather than by driving screens. Contributions are data: a command is an id,
 * a title, a context and a finite list of predicates, and nothing here
 * evaluates anything an extension wrote.
 *
 * **An extension that is failed, disabled or incompatible contributes
 * nothing.** Its commands leave the palette and its actions leave the screens
 * as a group, so no dead command is ever offered. An extension that is merely
 * idle — enabled but not started — contributes everything: workers start on
 * first use.
 */

import type {
	CommandContribution,
	ContextKind,
	ExtensionView,
	PanelContribution,
	Predicate
} from './types';

/** What is true right now, as far as predicates can ask. */
export interface Facts {
	repositoryOpen: boolean;
	hasWorkingChanges: boolean;
	taskSelected: boolean;
	taskHasCommit: boolean;
	pullRequestSelected: boolean;
	forgeConnected: boolean;
}

export const NO_FACTS: Facts = {
	repositoryOpen: false,
	hasWorkingChanges: false,
	taskSelected: false,
	taskHasCommit: false,
	pullRequestSelected: false,
	forgeConnected: false
};

/** Why each predicate is not met, in a few words. */
const UNMET: Record<Predicate, string> = {
	repositoryOpen: 'Open a repository',
	hasWorkingChanges: 'No changes',
	taskSelected: 'Select a task',
	taskHasCommit: 'The task has no commit',
	pullRequestSelected: 'Select a pull request',
	forgeConnected: 'Connect an account'
};

/** What a predicate needs implicitly from the context a command lives in. */
const IMPLIED: Record<ContextKind, Predicate[]> = {
	global: [],
	workingCopy: ['repositoryOpen'],
	farmTask: ['repositoryOpen', 'taskSelected'],
	pullRequest: ['repositoryOpen', 'pullRequestSelected']
};

/** The id a contribution is registered under: unique across extensions. */
export function qualified(extension: string, id: string): string {
	return `${extension}/${id}`;
}

/** Whether an extension contributes anything right now. */
export function contributes(extension: ExtensionView): boolean {
	return (
		extension.enabled &&
		extension.compatibility.compatible &&
		['installed', 'starting', 'active', 'stopping'].includes(extension.state)
	);
}

export function commandsOf(extension: ExtensionView): CommandContribution[] {
	return extension.manifest.contributes?.commands ?? [];
}

export function contextOf(command: CommandContribution): ContextKind {
	return command.context ?? 'global';
}

/** Why `command` cannot run now, or null when it can. */
export function unavailable(
	extension: ExtensionView,
	command: CommandContribution,
	facts: Facts
): string | null {
	if (!contributes(extension)) return extension.stateReason ?? 'Not enabled here';
	const reported = extension.unavailable.find((u) => u.id === command.id);
	if (reported) return reported.reason;
	if (extension.state === 'stopping') return 'Stopping';
	const needed = [...IMPLIED[contextOf(command)], ...(command.when ?? [])];
	for (const predicate of needed) {
		if (!facts[predicate]) return UNMET[predicate];
	}
	return null;
}

export interface Placed {
	extension: ExtensionView;
	command: CommandContribution;
	key: string;
}

/** Commands shown on a screen for `context`, in a stable order. */
export function actionsFor(extensions: ExtensionView[], context: ContextKind): Placed[] {
	const out: Placed[] = [];
	for (const extension of extensions) {
		if (!contributes(extension)) continue;
		for (const command of commandsOf(extension)) {
			if (contextOf(command) !== context) continue;
			if (command.menu === false) continue;
			out.push({ extension, command, key: qualified(extension.id, command.id) });
		}
	}
	return out.sort((a, b) => a.command.title.localeCompare(b.command.title));
}

/** Commands the palette offers. */
export function paletteCommands(extensions: ExtensionView[]): Placed[] {
	const out: Placed[] = [];
	for (const extension of extensions) {
		if (!contributes(extension)) continue;
		for (const command of commandsOf(extension)) {
			if (command.palette === false) continue;
			out.push({ extension, command, key: qualified(extension.id, command.id) });
		}
	}
	return out;
}

export interface PlacedPanel {
	extension: ExtensionView;
	panel: PanelContribution;
	key: string;
	/** The review provider whose history a findings panel shows. */
	provider: string | null;
}

/** Where a panel goes when its manifest does not say — mirrors `Panel::placed`. */
export function placement(panel: PanelContribution): ContextKind {
	if (panel.location) return panel.location;
	if (panel.renderer === 'reviewStatus') return 'pullRequest';
	if (panel.renderer === 'reviewFindings') return 'workingCopy';
	return 'global';
}

export function panelsFor(extensions: ExtensionView[], location: ContextKind): PlacedPanel[] {
	const out: PlacedPanel[] = [];
	for (const extension of extensions) {
		if (!contributes(extension)) continue;
		const providers = extension.manifest.contributes?.reviewProviders ?? [];
		for (const panel of extension.manifest.contributes?.panels ?? []) {
			if (placement(panel) !== location) continue;
			const provider =
				panel.provider ?? (panel.renderer === 'reviewFindings' && providers.length === 1 ? providers[0].id : null);
			out.push({ extension, panel, key: qualified(extension.id, panel.id), provider });
		}
	}
	return out;
}

/** The words for an extension's state on its card. */
export const STATE_LABELS: Record<ExtensionView['state'], string> = {
	installed: 'Ready',
	disabled: 'Off',
	starting: 'Starting',
	active: 'Running',
	stopping: 'Stopping',
	failed: 'Stopped',
	incompatible: "Can't run here"
};

export const PROVENANCE_LABELS: Record<ExtensionView['provenance'], string> = {
	bundled: 'Official',
	local: 'Installed from a file',
	development: 'Development'
};
