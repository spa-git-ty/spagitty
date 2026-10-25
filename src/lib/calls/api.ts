// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Typed wrappers around Calls' Tauri commands (FEAT-160).
 *
 * The only Calls module that calls `invoke`, as `$lib/recipes/api.ts` is for
 * recipes. Nothing here reaches a network: a send is made in Rust and the
 * answer arrives as an event.
 */

import { invoke } from '@tauri-apps/api/core';
import type { Call, Collection, HistoryEntry, Loaded, PersonState } from './types';

/** A piece of an event stream, as it arrived. */
export const CHUNK_EVENT = 'calls-chunk';
/** A send ended: answered, failed or cancelled. */
export const DONE_EVENT = 'calls-done';

export function load(): Promise<Loaded> {
	return invoke('calls_load');
}

/** Save a collection; answers with the file stem it was written under. */
export function saveCollection(collection: Collection, stem: string | null, was: string | null): Promise<string> {
	return invoke('calls_save_collection', { collection, stem, was });
}

export function deleteCollection(stem: string): Promise<void> {
	return invoke('calls_delete_collection', { stem });
}

export function saveState(person: PersonState): Promise<void> {
	return invoke('calls_save_state', { person });
}

/**
 * Build and start sending `call`. Rejects at once, with nothing sent, when a
 * variable has no value; otherwise the answer is a `calls-done` event.
 */
export function send(ask: { sendId: string; stem: string | null; call: Call; environment: string | null }): Promise<void> {
	return invoke('calls_send', { ask });
}

export function cancel(sendId: string): Promise<void> {
	return invoke('calls_cancel', { sendId });
}

/** Write a response's bytes, as they arrived, to `path`. */
export function saveBody(sendId: string, path: string): Promise<void> {
	return invoke('calls_save_body', { sendId, path });
}

export function history(): Promise<HistoryEntry[]> {
	return invoke('calls_history');
}

export function historyBody(id: string): Promise<string | null> {
	return invoke('calls_history_body', { id });
}

export function clearHistory(): Promise<void> {
	return invoke('calls_history_clear');
}
