// SPDX-License-Identifier: GPL-3.0-or-later
import { PIPELINE_CARD_HEIGHT as H, PIPELINE_CARD_WIDTH as W, PIPELINE_COLUMN_PITCH as X, PIPELINE_ROW_PITCH as Y } from '$lib/metrics';
import type { Job } from './types';

export interface Card { key: string; job: Job; x: number; y: number; subtitle: string; count: number }
export interface Edge { key: string; from: string; to: string; path: string; skipped: boolean; x1: number; x2: number; y1: number; y2: number }
export interface Label { column: number; x: number; text: string }

/** Room above the first row for the column labels. */
const TOP = 32;

const groupName = (job: Job) => job.stage || `Depth ${job.depth + 1}`;

/** Over 200 jobs, one card per stage (or depth) stands for its members. */
function collapsed(jobs: Job[]): Job[] {
	return [...new Set(jobs.map(groupName))].map((name) => {
		const members = jobs.filter((job) => groupName(job) === name);
		const state = members.some((job) => job.state === 'running')
			? 'running'
			: members.every((job) => job.state === 'skipped')
				? 'skipped'
				: 'host';
		return { ...members[0], id: `stage:${name}`, name, needs: [], matrix: [], implicit: false, reason: `${members.length} jobs`, state };
	});
}

function subtitle(job: Job, value: unknown, collapse: boolean): string {
	if (value) return Object.values(value as Record<string, unknown>).join(' · ');
	if (collapse) return job.reason;
	return job.runsOn.join(', ') || job.environment || job.condition || job.steps[0]?.command || job.reason;
}

/**
 * Place every card once. Columns follow `needs` depth, or the GitLab stage in
 * Stages; empty columns close up. Within a column a card sits near the cards
 * it needs, so edges cross less. GitLab's stage order (`implicit`) is shown by
 * the columns and not drawn as edges: every job to every job of the stage
 * before is a tangle, not information.
 */
export function layout(jobs: Job[], stages: string[], mode: 'needs' | 'stages', expanded: Set<string>, collapse: boolean) {
	const groups = collapse ? collapsed(jobs) : jobs;
	const wanted = (job: Job) => {
		const stage = stages.indexOf(job.stage);
		return mode === 'stages' && stage >= 0 ? stage : job.depth;
	};
	const used = [...new Set(groups.map(wanted))].sort((a, b) => a - b);
	const columnOf = new Map(used.map((value, index) => [value, index]));

	const cards: Card[] = [];
	const byId = new Map<string, Card[]>();
	const rowOf = new Map<string, number>();
	for (const [column] of used.entries()) {
		const members = groups
			.map((job, order) => ({ job, order }))
			.filter(({ job }) => columnOf.get(wanted(job)) === column)
			.map(({ job, order }) => {
				const parents = job.implicit ? [] : job.needs.map((id) => rowOf.get(id)).filter((row): row is number => row !== undefined);
				const weight = parents.length ? parents.reduce((a, b) => a + b, 0) / parents.length : Number.POSITIVE_INFINITY;
				return { job, order, weight };
			})
			.sort((a, b) => a.weight - b.weight || a.order - b.order);
		let row = 0;
		for (const { job } of members) {
			const values = expanded.has(job.id) && job.matrix.length ? job.matrix : [null];
			const placed: Card[] = [];
			values.forEach((value, index) => {
				const card: Card = {
					key: `${job.id}/${index}`,
					job,
					x: column * X,
					y: TOP + row * Y,
					subtitle: subtitle(job, value, collapse),
					count: job.matrix.length
				};
				row++;
				cards.push(card);
				placed.push(card);
			});
			rowOf.set(job.id, (placed[0].y - TOP) / Y);
			byId.set(job.id, placed);
		}
	}

	const edges: Edge[] = [];
	for (const card of cards) {
		if (card.job.implicit) continue;
		for (const name of card.job.needs) {
			for (const start of byId.get(name) ?? []) {
				const x1 = start.x + W, x2 = card.x, y1 = start.y + H / 2, y2 = card.y + H / 2, middle = (x1 + x2) / 2;
				edges.push({
					key: `${start.key}>${card.key}`,
					from: start.job.id,
					to: card.job.id,
					path: `M${x1} ${y1} C${middle} ${y1},${middle} ${y2},${x2} ${y2}`,
					skipped: card.job.state === 'skipped',
					x1, x2, y1, y2
				});
			}
		}
	}

	const labels: Label[] = used.map((value, column) => {
		const names = new Set(groups.filter((job) => columnOf.get(wanted(job)) === column).map((job) => job.stage));
		const text = mode === 'stages' ? (stages[value] ?? '') : names.size === 1 ? [...names][0] : '';
		return { column, x: column * X, text };
	});
	const width = Math.max(W, ...cards.map((c) => c.x + W));
	const height = Math.max(160, ...cards.map((c) => c.y + H + 20));
	return { cards, edges, width, height, labels };
}
