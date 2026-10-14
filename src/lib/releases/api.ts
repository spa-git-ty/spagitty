// SPDX-License-Identifier: GPL-3.0-or-later
import { Channel, invoke } from '@tauri-apps/api/core';
import type { Artifact, Asset, Hosted, Page, Preferences, Progress, Read, Release, Write } from './types';
export const remotes = (): Promise<Hosted[]> => invoke('issues_remotes');
export const preferences = (remote: string): Promise<Preferences> => invoke('releases_preferences', { remote });
export const hide = (remote: string, id: string, hidden: boolean): Promise<Preferences> => invoke('releases_hide', { remote, id, hidden });
export const cached = (remote: string, page: number): Promise<Page | null> => invoke('releases_cached', { remote, page });
export const page = (remote: string, page: number): Promise<Read<Page>> => invoke('releases_page', { remote, page });
export const write = (remote: string, action: Write): Promise<unknown> => invoke('releases_write', { remote, action, confirmed: true });
export const notes = (since: string, target: string): Promise<string> => invoke('releases_notes', { since, target });
export const target = (remote: string, revision: string): Promise<string> => invoke('releases_target', { remote, revision });
export const changelog = (remote: string, branch: string, expected: string, tag: string, body: string): Promise<string> => invoke('releases_changelog', { remote, branch, expected, tag, body, confirmed: true });
export const pushTarget = (remote: string, branch: string, expected: string, commit: string): Promise<void> => invoke('releases_push_target', { remote, branch, expected, commit, confirmed: true });
export const upload = (remote: string, release: Release, path: string, update: (value: Progress) => void): Promise<Asset> => {
	const progress = new Channel<Progress>(); progress.onmessage = update;
	return invoke('releases_upload', { remote, release, path, confirmed: true, progress });
};
export const artifacts = (remote: string, commit: string): Promise<Artifact[]> => invoke('releases_artifacts', { remote, commit });
export const download = (remote: string, artifact: Artifact, update: (value: Progress) => void): Promise<string> => {
	const progress = new Channel<Progress>(); progress.onmessage = update;
	return invoke('releases_download_artifact', { remote, artifact, progress });
};
