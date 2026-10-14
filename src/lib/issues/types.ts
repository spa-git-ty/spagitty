// SPDX-License-Identifier: GPL-3.0-or-later

/** Mirrors forge::issues and src-tauri's forge_views. */
import type { ForgeRepo } from '$lib/types';
export interface Hosted { remote: string; repo: ForgeRepo; user: string | null }
export interface Label { name: string; color: string }
export interface Milestone { id: string; title: string; open: number | null; closed: number | null }
export interface Issue { id: string; number: number; title: string; body: string; state: string; author: string; created: string; updated: string; url: string; replies: number; labels: Label[]; assignees: string[]; milestone: Milestone | null; locked: boolean; branchSuggestion: string }
export interface IssueComment { id: string; author: string; body: string; at: string; url: string }
export interface IssueEvent { id: string; kind: string; author: string; at: string; title: string; url: string; branch: string | null }
export interface Page { issues: Issue[]; page: number; more: boolean; etag: string | null; canWrite: boolean | null }
export interface Thread { issue: Issue; comments: IssueComment[]; events: IssueEvent[]; commentsMore: boolean; eventsMore: boolean }
export interface Metadata { defaultBranch: string; open: number; closed: number; labels: Label[]; milestones: Milestone[] }
export interface Read<T> { data: T; cacheWarning: string | null }
export type Write = { kind: 'create'; title: string; body: string } | { kind: 'comment'; number: number; body: string } | { kind: 'close'; number: number };
