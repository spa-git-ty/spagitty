// SPDX-License-Identifier: GPL-3.0-or-later
import type { Job, LiveJob, Workflow } from './types';
/** Match names only when unique. An ambiguous host name has no guessed edges. */
export function liveGraph(rows: LiveJob[], workflow: Workflow | null): Job[] {
	const byId = new Map<string, string[]>(), matching = new Map<number, Job>();
	for (const row of rows) {
		const candidates = workflow?.jobs.filter((job) => job.name === row.name || (job.name === job.id && row.name.startsWith(`${job.id} (`)) || job.matrix.some((matrix) => {
			if (!matrix || typeof matrix !== 'object') return false;
			const name = job.name.replace(/\$\{\{\s*matrix\.([A-Za-z_][A-Za-z0-9_]*)\s*\}\}/g, (_, key: string) => String((matrix as Record<string, unknown>)[key] ?? `unknown:${key}`));
			return name === row.name;
		})) ?? [];
		if (candidates.length === 1) { const job = candidates[0]; matching.set(row.id, job); byId.set(job.id, [...byId.get(job.id) ?? [], String(row.id)]); }
	}
	const stages = [...new Set(rows.map((j) => j.stage))];
	return rows.map((row): Job => {
		const job = matching.get(row.id);
		return { id: String(row.id), name: row.name, stage: row.stage || job?.stage || '', depth: job?.depth ?? Math.max(0, stages.indexOf(row.stage)), needs: job?.needs.flatMap((id) => byId.get(id) ?? []) ?? [], state: row.state === 'completed' ? row.result : row.state, reason: row.result || row.state, matrix: [], runsOn: [row.runner], steps: [], condition: '', environment: '', services: [], timeout: '', secrets: [], file: '', line: 0 };
	});
}
