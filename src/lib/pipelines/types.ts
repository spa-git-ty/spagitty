// SPDX-License-Identifier: GPL-3.0-or-later
/** Mirrors spagitty_core::pipelines, forge::pipelines and the thin commands. */
export type { Hosted, Read } from '$lib/issues/types';
export interface Simulation { event: string; reference: string; base: string; defaultBranch: string; changed: string[]; changesKnown: boolean }
export interface Problem { file: string; line: number; message: string }
export interface Step { name: string; command: string; condition: string }
export interface Job { id: string; name: string; stage: string; needs: string[]; condition: string; matrix: unknown[]; environment: string; services: string[]; steps: Step[]; runsOn: string[]; timeout: string; secrets: string[]; file: string; line: number; state: string; reason: string; depth: number }
export interface Workflow { file: string; name: string; kind: string; triggers: string[]; stages: string[]; jobs: Job[]; problems: Problem[]; sourceFiles: string[] }
export interface Preview { workflows: Workflow[]; changed: string[]; changesWarning: string | null; simulation: Simulation }
export interface Run { id: number; name: string; branch: string; commit: string; event: string; state: string; result: string; url: string; started: number; ended: number; duration: number | null }
export interface LiveStep { number: number; name: string; state: string; result: string; started: number; ended: number }
export interface LiveJob { id: number; name: string; stage: string; state: string; result: string; runner: string; started: number; ended: number; duration: number | null; url: string; steps: LiveStep[] }
export interface Page { runs: Run[]; etag: string | null; more: boolean }
export interface Jobs { jobs: LiveJob[]; etag: string | null; more: boolean; page: number }
export interface Watches { defaultBranch: string; branches: string[]; requests: string[] }
export interface Preferences { branches: string[] }
export type Write = { kind: 'cancel' | 'rerunFailed'; id: number } | { kind: 'lint'; content: string };
