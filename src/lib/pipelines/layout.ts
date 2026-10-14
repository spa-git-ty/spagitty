// SPDX-License-Identifier: GPL-3.0-or-later
import { PIPELINE_CARD_HEIGHT as H, PIPELINE_CARD_WIDTH as W, PIPELINE_COLUMN_PITCH as X, PIPELINE_ROW_PITCH as Y } from '$lib/metrics';
import type { Job } from './types';
export interface Card { key: string; job: Job; x: number; y: number; subtitle: string; count: number }
export interface Edge { key: string; path: string; skipped: boolean; x1: number; x2: number; y1: number; y2: number }
export function layout(jobs: Job[], stages: string[], mode: 'needs' | 'stages', expanded: Set<string>, collapse: boolean) {
	const columns = new Map<number, number>(), cards: Card[] = [], byId = new Map<string, Card[]>();
	const groups = collapse ? [...new Set(jobs.map((j) => j.stage || `Depth ${j.depth + 1}`))].map((stage) => {
		const members = jobs.filter((j) => (j.stage || `Depth ${j.depth + 1}`) === stage);
		return { ...members[0], id: `stage:${stage}`, name: stage, needs: [], matrix: [], reason: `${members.length} jobs`, state: members.some((j) => j.state === 'running') ? 'running' : members.every((j) => j.state === 'skipped') ? 'skipped' : 'host' };
	}) : jobs;
	for (const job of groups) {
		const stage = stages.indexOf(job.stage), column = mode === 'stages' && stage >= 0 ? stage : job.depth;
		const values = expanded.has(job.id) && job.matrix.length ? job.matrix : [null];
		const placed: Card[] = [];
		for (let index = 0; index < values.length; index++) {
			const row = columns.get(column) ?? 0; columns.set(column, row + 1);
			const value = values[index]; const card: Card = { key: `${job.id}/${index}`, job, x: column * X, y: row * Y + 32, subtitle: value ? JSON.stringify(value) : collapse ? job.reason : job.runsOn.join(', ') || job.condition || job.steps[0]?.command || job.reason, count: job.matrix.length };
			cards.push(card); placed.push(card);
		}
		byId.set(job.id, placed);
	}
	const edges: Edge[] = [];
	for (const card of cards) for (const name of card.job.needs) for (const start of byId.get(name) ?? []) {
		const x1 = start.x + W, x2 = card.x, y1 = start.y + H / 2, y2 = card.y + H / 2, middle = (x1 + x2) / 2;
		edges.push({ key: `${start.key}>${card.key}`, path: `M${x1} ${y1} C${middle} ${y1},${middle} ${y2},${x2} ${y2}`, skipped: card.job.state === 'skipped', x1, x2, y1, y2 });
	}
	const width = Math.max(1, ...cards.map((c) => c.x + W)), height = Math.max(160, ...cards.map((c) => c.y + H + 20));
	const labels = [...columns.keys()].sort((a, b) => a - b).map((column) => ({ column, x: column * X, text: mode === 'stages' ? stages[column] ?? `Stage ${column + 1}` : `Depth ${column + 1}` }));
	return { cards, edges, width, height, labels };
}
