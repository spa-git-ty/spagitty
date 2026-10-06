// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Typed wrappers around the extension host's Tauri commands (FEAT-096).
 *
 * The same rule as `$lib/api.ts` and `$lib/farm/api.ts`: this is the only
 * module under `src/lib/extensions/` that calls `invoke`, and no component
 * imports Tauri directly — `extensions.test.ts` reads the directory and fails
 * if one does. A separate bridge because the host is a subsystem with its own
 * backend module, like the farm.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
	Capability,
	Detected,
	Disposition,
	InstallPreview,
	Installed,
	Invocation,
	Listing,
	Manifest,
	ReviewRecord,
	ReviewRequest,
	ScopePreview,
	Started
} from './types';

export function list(workdir: string | null): Promise<Listing> {
	return invoke('extensions_list', { workdir });
}

export function inspect(path: string, workdir: string | null): Promise<InstallPreview> {
	return invoke('extensions_inspect', { path, workdir });
}

export function install(token: string): Promise<Installed> {
	return invoke('extensions_install', { token });
}

export function rollback(id: string): Promise<Installed> {
	return invoke('extensions_rollback', { id });
}

export function uninstall(id: string, keepHistory: boolean, workdir: string | null): Promise<void> {
	return invoke('extensions_uninstall', { id, keepHistory, workdir });
}

export function attach(path: string): Promise<Manifest> {
	return invoke('extensions_attach', { path });
}

export function restart(id: string): Promise<void> {
	return invoke('extensions_restart', { id });
}

export function enable(
	id: string,
	workdir: string,
	optional: Capability[],
	consent: string | null
): Promise<void> {
	return invoke('extensions_enable', { id, workdir, optional, consent });
}

export function disable(id: string, workdir: string, forget: boolean): Promise<void> {
	return invoke('extensions_disable', { id, workdir, forget });
}

export function setGrant(
	id: string,
	workdir: string,
	capability: Capability,
	granted: boolean
): Promise<void> {
	return invoke('extensions_set_grant', { id, workdir, capability, granted });
}

export function setSetting(
	id: string,
	key: string,
	value: unknown,
	workdir: string | null
): Promise<void> {
	return invoke('extensions_set_setting', { id, key, value, workdir });
}

export function chooseExecutable(id: string, tool: string, path: string | null): Promise<Detected> {
	return invoke('extensions_choose_executable', { id, tool, path });
}

export function detectTool(id: string, tool: string): Promise<Detected> {
	return invoke('extensions_detect_tool', { id, tool });
}

export function runCommand(id: string, command: string, invocation: Invocation): Promise<Started> {
	return invoke('extensions_run_command', { id, command, invocation });
}

export function previewReview(
	id: string,
	provider: string,
	request: ReviewRequest,
	workdir: string
): Promise<ScopePreview> {
	return invoke('extensions_preview_review', { id, provider, request, workdir });
}

export function startReview(
	id: string,
	provider: string,
	request: ReviewRequest,
	workdir: string
): Promise<Started> {
	return invoke('extensions_start_review', { id, provider, request, workdir });
}

export function cancel(operation: string): Promise<void> {
	return invoke('extensions_cancel', { operation });
}

export function reviews(id: string, workdir: string): Promise<ReviewRecord[]> {
	return invoke('extensions_reviews', { id, workdir });
}

export function setDisposition(
	id: string,
	workdir: string,
	review: string,
	finding: string,
	disposition: Disposition
): Promise<ReviewRecord> {
	return invoke('extensions_set_disposition', { id, workdir, review, finding, disposition });
}

export function deleteReviews(id: string, workdir: string, review: string | null): Promise<void> {
	return invoke('extensions_delete_reviews', { id, workdir, review });
}

export function panel(id: string, panel: string, invocation: Invocation): Promise<unknown> {
	return invoke('extensions_panel', { id, panel, invocation });
}

export function suggestedBases(workdir: string): Promise<string[]> {
	return invoke('extensions_suggested_bases', { workdir });
}

export function sendFindings(
	id: string,
	workdir: string,
	review: string,
	findings: string[]
): Promise<{ task: string; message: string }> {
	return invoke('extensions_send_findings', { id, workdir, review, findings });
}

export function confirm(id: string, approved: boolean): Promise<void> {
	return invoke('extensions_confirm', { id, approved });
}
