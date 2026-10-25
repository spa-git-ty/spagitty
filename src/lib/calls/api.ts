// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Typed wrappers around Calls' Tauri commands (FEAT-160).
 *
 * The only Calls module that calls `invoke`, as `$lib/recipes/api.ts` is for
 * recipes. Nothing here reaches a network: a send is made in Rust and the
 * answer arrives as an event.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
	Call,
	Collection,
	Environment,
	Environments,
	HistoryEntry,
	Loaded,
	PersonState,
	Private,
	Shared
} from './types';

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

// --- Environments, secrets and kept values (FEAT-161) ---

export function environments(): Promise<Environments> {
	return invoke('calls_environments');
}

/**
 * Save an environment. `was` is its stem before a rename; `copyFrom` the
 * environment it duplicates, whose keychain values it is given.
 */
export function saveEnvironment(
	environment: Environment,
	was: string | null,
	copyFrom: string | null = null
): Promise<Environments> {
	return invoke('calls_save_environment', { environment, was, copyFrom });
}

export function deleteEnvironment(stem: string): Promise<Environments> {
	return invoke('calls_delete_environment', { stem });
}

export function saveShared(shared: Shared): Promise<Environments> {
	return invoke('calls_save_shared', { shared });
}

/** An environment's kept values, and which secrets have a value; `null` is the shared variables. */
export function privateOf(environment: string | null): Promise<Private> {
	return invoke('calls_private', { environment });
}

/** One secret's value, while the eye is held. */
export function reveal(environment: string | null, name: string): Promise<string | null> {
	return invoke('calls_reveal', { environment, name });
}

/** Set a secret's value, or remove it with `null`. */
export function setPrivate(environment: string | null, name: string, value: string | null): Promise<void> {
	return invoke('calls_set_private', { environment, name, value });
}

export function keep(ask: {
	environment: string | null;
	var: string;
	value: string;
	callId: string;
	call: string;
	method: string;
	from: string;
}): Promise<Environments> {
	return invoke('calls_keep', { ask });
}

export function forgetKept(environment: string | null, name: string): Promise<Environments> {
	return invoke('calls_forget_kept', { environment, name });
}
