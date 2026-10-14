// SPDX-License-Identifier: GPL-3.0-or-later

/** The Hooks screen's typed command boundary (FEAT-137). */
import { invoke } from '@tauri-apps/api/core';
import type { Builder, Definition, Run, Stage, Tried } from './builder-types';

export const load = (): Promise<Builder> => invoke('hooks_builder');
export const save = (definition: Definition, install: boolean, imported: boolean): Promise<void> => invoke('hooks_save', { definition, install, imported });
export const importManager = (): Promise<Definition> => invoke('hooks_import');
export const matches = (paths: string[]): Promise<string[]> => invoke('hooks_matches', { paths });
export const runs = (): Promise<Run[]> => invoke('hooks_runs');
export const run = (id: string): Promise<Run> => invoke('hooks_run', { id });
export const log = (id: string, block: string): Promise<string> => invoke('hooks_log', { id, block });
export const dryRun = (definition: Definition, stage: Stage): Promise<Tried> => invoke('hooks_dry_run', { definition, stage });
export const tryMessage = (definition: Definition, message: string): Promise<Tried> => invoke('hooks_try_message', { definition, message });
export const stop = (id: string): Promise<void> => invoke('hooks_stop', { id });
export const undoRestage = (id: string): Promise<void> => invoke('hooks_undo_restage', { id });
