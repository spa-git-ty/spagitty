// SPDX-License-Identifier: GPL-3.0-or-later
import { invoke } from '@tauri-apps/api/core';
import type { Hosted, Metadata, Page, Read, Thread, Write } from './types';
export const remotes = (): Promise<Hosted[]> => invoke('issues_remotes');
export const cached = (remote: string, query: string, page: number): Promise<Page | null> => invoke('issues_cached', { remote, query, page });
export const page = (remote: string, query: string, page: number): Promise<Read<Page>> => invoke('issues_page', { remote, query, page });
export const metadata = (remote: string, refresh = false): Promise<Read<Metadata>> => invoke('issues_metadata', { remote, refresh });
export const thread = (remote: string, number: number, commentsPage = 1, eventsPage = 1): Promise<Thread> => invoke('issues_thread', { remote, number, commentsPage, eventsPage });
export const write = (remote: string, action: Write): Promise<unknown> => invoke('issues_write', { remote, action, confirmed: true });
export const branch = (remote: string, name: string): Promise<void> => invoke('issues_branch', { remote, name, confirmed: true });
