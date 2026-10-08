// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Typed wrappers around the agents' Tauri commands (2.0).
 *
 * The only agents module that calls `invoke`, as `$lib/farm/api.ts` is for
 * the farm: a command rename is a one-file change.
 */

import { invoke } from '@tauri-apps/api/core';
import type { AgentDefinition } from '$lib/farm/types';
import type {
	AgentsSnapshot,
	Assignment,
	Control,
	Defaults,
	Jobs,
	Notify,
	OmpOptions,
	Probe,
	RemoteAgent,
	RemoteInput,
	RepoRules,
	StartRequest,
	Tested
} from './types';

export function snapshot(repo: string | null): Promise<AgentsSnapshot> {
	return invoke('agents_snapshot', { repo });
}

export function setJobs(id: string, jobs: Jobs): Promise<void> {
	return invoke('agents_set_jobs', { id, jobs });
}

export function setCodexFullAccess(enabled: boolean): Promise<void> {
	return invoke('agents_set_codex_full_access', { enabled });
}

export function setOmpOptions(options: OmpOptions): Promise<void> {
	return invoke('agents_set_omp_options', { options });
}

export function setAgyAutoApprove(enabled: boolean): Promise<void> {
	return invoke('agents_set_agy_auto_approve', { enabled });
}

export function saveCustom(definition: AgentDefinition): Promise<void> {
	return invoke('agents_save_custom', { definition });
}

export function takeOffer(take: AgentDefinition[]): Promise<void> {
	return invoke('agents_take_offer', { take });
}

export function saveRemote(agent: RemoteInput): Promise<RemoteAgent> {
	return invoke('agents_save_remote', { agent });
}

export function remove(id: string): Promise<void> {
	return invoke('agents_remove', { id });
}

export function setDefaults(defaults: Defaults, notify: Notify): Promise<void> {
	return invoke('agents_set_defaults', { defaults, notify });
}

export function setRules(repo: string, rules: RepoRules): Promise<void> {
	return invoke('agents_set_rules', { repo, rules });
}

export function consent(repo: string, provider: string): Promise<void> {
	return invoke('agents_consent', { repo, provider });
}

export function testLocal(id: string): Promise<Tested> {
	return invoke('agents_test_local', { id });
}

export function models(probe: Probe): Promise<string[]> {
	return invoke('agents_models', { probe });
}

export function testRemote(probe: Probe): Promise<Tested> {
	return invoke('agents_test_remote', { probe });
}

export function start(request: StartRequest): Promise<Assignment> {
	return invoke('assignment_start', { request });
}

export function control(id: string, control: Control): Promise<void> {
	return invoke('assignment_control', { id, control });
}

export function list(repo: string): Promise<Assignment[]> {
	return invoke('assignment_list', { repo });
}

export function transcript(repo: string, id: string): Promise<string> {
	return invoke('assignment_transcript', { repo, id });
}

export function forget(repo: string, id: string): Promise<void> {
	return invoke('assignment_forget', { repo, id });
}
