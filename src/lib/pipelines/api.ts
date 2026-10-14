// SPDX-License-Identifier: GPL-3.0-or-later
import { invoke } from '@tauri-apps/api/core';
import type { Hosted, Jobs, Page, Preferences, Preview, Read, Simulation, Watches, Workflow, Write } from './types';
export const remotes = (): Promise<Hosted[]> => invoke('issues_remotes');
export const preview = (simulation: Simulation, mode: string, from: string, to: string): Promise<Preview> => invoke('pipelines_preview', { simulation, mode, from, to });
export const source = (file: string): Promise<string> => invoke('pipelines_source', { file });
export const preferences = (remote: string, branches?: string[]): Promise<Preferences> => invoke('pipelines_preferences', { remote, branches });
export const watches = (remote: string): Promise<Read<Watches>> => invoke('pipelines_watches', { remote });
export const cached = (remote: string, branch: string): Promise<Page | null> => invoke('pipelines_cached', { remote, branch });
export const runs = (remote: string, branch: string): Promise<Read<Page>> => invoke('pipelines_runs', { remote, branch });
export const jobs = (remote: string, id: number, page: number): Promise<Read<Jobs>> => invoke('pipelines_jobs', { remote, id, page });
export const log = (remote: string, id: number): Promise<string> => invoke('pipelines_log', { remote, id });
export const graph = (remote: string, id: number): Promise<Read<Workflow>> => invoke('pipelines_graph', { remote, id });
export const write = (remote: string, action: Write): Promise<unknown> => invoke('pipelines_write', { remote, action, confirmed: true });
