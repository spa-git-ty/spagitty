// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Typed wrappers around the recipes' Tauri commands (FEAT-133).
 *
 * The only recipes module that calls `invoke`, as `$lib/agents/api.ts` is for
 * agents: a command rename is a one-file change.
 */

import { invoke } from '@tauri-apps/api/core';
import type { Check, Choice, Listed, Proposed, Recipe, RestorePoint, RunView } from './types';

/** A run changed: a step ran, it stopped, it ended. Carries the whole run. */
export const RUN_EVENT = 'recipe-run';
/** A line was added to a run's output, as it happened. */
export const LINE_EVENT = 'recipe-line';

export function list(): Promise<Listed> {
	return invoke('recipes_list');
}

/** Save a recipe. `was` is the name it was saved under before a rename. */
export function save(recipe: Recipe, was: string | null): Promise<Recipe[]> {
	return invoke('recipes_save', { recipe, was });
}

export function remove(name: string): Promise<Recipe[]> {
	return invoke('recipes_delete', { name });
}

/** A recipe from the JSON it was exported as. */
export function importFrom(text: string): Promise<Recipe> {
	return invoke('recipes_import', { text });
}

/** Validate a stack against the repository as it is, and say what it is. */
export function check(recipe: Recipe): Promise<Check> {
	return invoke('recipes_check', { recipe });
}

/** Ask the recipe's agent for a stack of blocks. Nothing of it runs. */
export function propose(recipe: Recipe, sentence: string): Promise<Proposed> {
	return invoke('recipes_propose', { recipe, sentence });
}

export function start(recipe: Recipe): Promise<RunView> {
	return invoke('recipe_run_start', { recipe });
}

/** The person's choice at a stop. Rejects when it cannot be taken yet. */
export function answer(choice: Choice): Promise<RunView> {
	return invoke('recipe_run_answer', { choice });
}

export function stop(): Promise<void> {
	return invoke('recipe_run_stop');
}

/** *Undo the whole run*. */
export function undo(): Promise<RunView> {
	return invoke('recipe_run_undo');
}

/** The run on this repository: the one going, or the last one's record. */
export function current(): Promise<RunView | null> {
	return invoke('recipe_run_current');
}

/** Put a run that is over away. */
export function close(): Promise<void> {
	return invoke('recipe_run_close');
}

export function restorePoints(): Promise<RestorePoint[]> {
	return invoke('restore_points');
}

export function restoreTo(id: string): Promise<void> {
	return invoke('restore_point_undo', { id });
}

export function dropRestorePoint(id: string): Promise<RestorePoint[]> {
	return invoke('restore_point_drop', { id });
}
