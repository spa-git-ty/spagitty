// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merge with an agent (2.0): the agent's resolutions, as the resolver's own
 * choices.
 *
 * Spagitty names each resolution in the resolver's words before it gets
 * here, so what the agent proposes is a choice the person already reads:
 * Take A, Both, Pick lines, Edit. Proposed, it waits on the conflict card
 * with *Accept*; applied at *Sign off* or *Unattended*, it is the region's
 * choice, with the agent as who chose it. The person choosing another way
 * is a choice like any other, and the engine is told so the checks run what
 * will land.
 */

import { agents } from '$lib/agents/store.svelte';
import { choiceWords, consentSlug, isLive } from '$lib/agents/levels';
import type { Assigned, Assignment, MergeFile, Proposal, StartRequest } from '$lib/agents/types';
import type { AgentProposal, Choice, ResolverFile } from '$lib/resolver/model';
import { repo } from '$lib/repo.svelte';
import { notice } from '$lib/ui/notice.svelte';
import { resolving } from './resolve.svelte';
import { merger } from './store.svelte';

type Resolution = Proposal & { body: { kind: 'resolution' } };

function isResolution(p: Proposal): p is Resolution {
	return p.body.kind === 'resolution';
}

/** The assignment on the merge Merger has open, if one is. */
export function current(): Assignment | null {
	if (!merger.a || !merger.b) return null;
	const found = agents.forMerge(merger.a, merger.b);
	return found && found.repo === repo.info?.path ? found : null;
}

/** The latest standing proposal for each region: not superseded, not dismissed. */
export function standing(a: Assignment): Map<string, Resolution> {
	const out = new Map<string, Resolution>();
	for (const p of a.proposals) {
		if (!isResolution(p) || p.state === 'superseded' || p.state === 'dismissed') continue;
		out.set(`${p.body.path}#${p.body.region}`, p);
	}
	return out;
}

/** What the conflict cards show: proposals still waiting for the person. */
export function proposalsFor(
	a: Assignment | null,
	files: ResolverFile[],
	names: { a: string; b: string }
): Record<string, (AgentProposal | null)[]> {
	if (!a) return {};
	const by = standing(a);
	const out: Record<string, (AgentProposal | null)[]> = {};
	for (const file of files) {
		out[file.path] = file.regions.map((region) => {
			const p = by.get(`${file.path}#${region.index}`);
			if (!p || p.state !== 'proposed') return null;
			return {
				agent: a.agent.name,
				words: choiceWords(p.body.choice, names),
				why: p.body.why,
				sure: p.sure,
				asked: p.why
			};
		});
	}
	return out;
}

/** The conflict the agent is on, while it is on one. */
export function workingOn(a: Assignment | null): { path: string; index: number } | null {
	if (!a || !isLive(a)) return null;
	const last = a.steps.at(-1);
	if (!last || last.kind.kind !== 'conflict' || (last.state !== 'running' && last.state !== 'waiting')) return null;
	return { path: last.kind.path, index: last.kind.region };
}

/**
 * Bring the resolver up to an assignment: each resolution the agent applied
 * becomes the region's choice, unless the person has chosen there already.
 */
export function apply(a: Assignment): void {
	if (!a || a.target.kind !== 'merge' || a.target.a !== merger.a || a.target.b !== merger.b) return;
	if (!resolving.files.length) return;
	for (const p of standing(a).values()) {
		if (p.state !== 'applied') continue;
		const { path, region } = p.body;
		const now = resolving.choices[path]?.[region] ?? null;
		const by = resolving.authors[path]?.[region] ?? null;
		const mine = now !== null && by === null;
		if (mine) continue;
		if (now && by && JSON.stringify(now) === JSON.stringify(p.body.choice)) continue;
		resolving.choose(path, region, p.body.choice as Choice, { agent: a.agent.name, decided: 'agent' });
	}
}

/** Accept what the agent proposes for a region: the person's decision. */
export function accept(a: Assignment, path: string, index: number): void {
	const p = standing(a).get(`${path}#${index}`);
	if (!p) return;
	resolving.choose(path, index, p.body.choice as Choice, { agent: a.agent.name, decided: 'person' });
	if (isLive(a)) void agents.control(a.id, { kind: 'decide', proposal: p.id, state: 'accepted' });
}

/** Accept every resolution waiting at a gate. */
export function acceptAll(a: Assignment, proposals: Proposal[]): void {
	for (const p of proposals) {
		if (isResolution(p)) accept(a, p.body.path, p.body.region);
	}
}

/** The person chose for a region where the agent proposed something else. */
export function personChose(path: string, index: number, choice: Choice | null): void {
	const a = current();
	if (!a || !isLive(a)) return;
	const p = standing(a).get(`${path}#${index}`);
	if (!p || !choice) return;
	if (JSON.stringify(choice) === JSON.stringify(p.body.choice)) return;
	void agents.control(a.id, { kind: 'decide', proposal: p.id, state: 'dismissed', choice });
}

export function askWhy(a: Assignment, path: string, index: number): void {
	const p = standing(a).get(`${path}#${index}`);
	if (p) void agents.control(a.id, { kind: 'askWhy', proposal: p.id });
}

/** `Co-authored-by` for each agent whose choice stands, which Spagitty already
 * reads to attribute agent work on the graph. */
export function trailers(names: string[]): string {
	return names.map((name) => `Co-authored-by: ${name} <agent@spagitty.invalid>`).join('\n');
}

/** The commit message with the agents' trailers, once. */
export function withTrailers(message: string, names: string[]): string {
	const lines = trailers(names.filter((name) => !message.includes(`Co-authored-by: ${name} `)));
	if (!lines) return message;
	return `${message.trimEnd()}\n\n${lines}`;
}

const acted = new Set<string>();

/** The last act at *Unattended*: land, as the commit dialog would. */
async function lastAct(a: Assignment): Promise<void> {
	if (!a.lastAct || a.lastAct.kind !== 'land' || acted.has(a.id)) return;
	acted.add(a.id);
	const ours = a.target.kind === 'merge' && a.target.a === merger.a && a.target.b === merger.b && a.repo === repo.info?.path;
	if (!ours || !resolving.ready) {
		await agents.control(a.id, {
			kind: 'acted',
			ok: false,
			message: ours ? 'A conflict was left unresolved, so nothing was landed.' : 'Merger is no longer on this merge.'
		});
		return;
	}
	resolving.markUnattended(a.agent.name);
	merger.setMessage(withTrailers(merger.message, resolving.agentsInResult));
	const landed = await resolving.commit();
	await agents.control(
		a.id,
		landed ? { kind: 'acted', ok: true } : { kind: 'acted', ok: false, message: merger.landError ?? 'It did not land.' }
	);
	if (landed) notice.ok(`${a.agent.name} landed the merge`, `${a.target.kind === 'merge' ? a.target.b : ''} into ${merger.roles?.targetName ?? ''}`);
}

let unsubscribe: (() => void) | null = null;

/** Keep the resolver in step with the agents' work. Safe to call twice. */
export function follow(): void {
	if (unsubscribe) return;
	unsubscribe = agents.subscribe((a) => {
		if (a.job !== 'merge') return;
		apply(a);
		void lastAct(a);
	});
}

/** The conflicts, as the engine is handed them. */
export function workOf(files: ResolverFile[]): MergeFile[] {
	return files.map((file) => ({
		path: file.path,
		merged: file.merged,
		eol: file.eol,
		whole: file.whole,
		regions: file.regions.map((region) => ({
			index: region.index,
			start: region.start,
			end: region.end,
			a: region.a,
			b: region.b,
			base: region.base,
			aFrom: region.aFrom,
			bFrom: region.bFrom
		}))
	}));
}

/** Assign an agent to the merge Merger has planned. The conflicts are read
 * first, as *Resolve* reads them. */
export async function assign(chosen: Assigned): Promise<Assignment | null> {
	const path = repo.info?.path;
	const plan = merger.forecast;
	if (!path || !plan || !merger.a || !merger.b) return null;
	if (!resolving.files.length) await resolving.open(path, null);
	if (!resolving.files.length) {
		notice.failed('The agent was not assigned', resolving.error ?? 'There are no conflicts to resolve.');
		return null;
	}
	const request: StartRequest = {
		repo: path,
		agent: chosen.agent,
		level: chosen.level,
		note: chosen.note,
		target: {
			kind: 'merge',
			a: merger.a,
			b: merger.b,
			aTip: plan.a.tip,
			bTip: plan.b.tip,
			base: plan.base,
			strategy: merger.strategy,
			into: merger.roles?.targetName ?? merger.a
		},
		work: { job: 'merge', files: workOf(resolving.files) },
		lands: chosen.lands
	};
	const remote = agents.snapshot?.remote.find((r) => r.id === chosen.agent) ?? null;
	return agents.start(request, remote?.providerLabel ?? null, remote ? consentSlug(remote) : null);
}

/** For the tests. */
export function forgetActed(): void {
	acted.clear();
	unsubscribe?.();
	unsubscribe = null;
}
