// SPDX-License-Identifier: GPL-3.0-or-later
export type { Hosted, Read } from '$lib/issues/types';
export interface Asset { id: string; name: string; url: string; size: number | null; downloads: number | null; removable: boolean }
export interface Release {
	id: string; tag: string; target: string; title: string; body: string;
	draft: boolean; prerelease: boolean; latest: boolean; author: string;
	created: string; url: string; assets: Asset[]; local: boolean;
}
export interface Page { releases: Release[]; page: number; more: boolean; etag: string | null }
export interface Preferences { hidden: string[]; drafts: Release[] }
export interface Progress { sent: number; total: number }
export interface Artifact { id: string; name: string; size: number | null; run: string; commit: string }
export interface Download { id: string; sent: number; total: number; busy: boolean; error: string; path: string | null }
export interface FileTransfer {
	key: number; path: string; name: string; sent: number; total: number;
	state: 'queued' | 'uploading' | 'uploaded' | 'error'; error: string; asset: Asset | null;
}
export type Write =
	| { kind: 'save'; release: Release; publish: boolean }
	| { kind: 'delete'; id: string; tag: string }
	| { kind: 'deleteTag'; tag: string }
	| { kind: 'removeAsset'; releaseId: string; tag: string; assetId: string }
	| { kind: 'linkAsset'; tag: string; name: string; url: string }
	| { kind: 'generateNotes'; tag: string; target: string; since: string };
export const empty = (target = ''): Release => ({ id: '', tag: '', target, title: '', body: '', draft: true, prerelease: false, latest: false, author: '', created: '', url: '', assets: [], local: false });
