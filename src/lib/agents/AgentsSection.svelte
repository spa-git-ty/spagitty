<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import { repo } from '$lib/repo.svelte';
	import * as api from './api';
	import { agents } from './store.svelte';
	import { LEVELS, perRun } from './levels';
	import ApiAgentSheet from './ApiAgentSheet.svelte';
	import CustomAgentSheet from './CustomAgentSheet.svelte';
	import OmpSettingsSheet from './OmpSettingsSheet.svelte';
	import type { AgentDefinition } from '$lib/farm/types';
	import type { Defaults, Jobs, Level, LocalAgent, Notify, RemoteAgent, RepoRules, Tested } from './types';

	/**
	 * Settings › Agents (2.0): where agents are attached, once, for the
	 * machine, and the open repository's rules for them.
	 *
	 * Built like the other sections — labelled rows and chip rows — and
	 * telling nobody they need an agent: with nothing set up it lists what
	 * detection found and the two ways to add one. Review and Merger say
	 * nothing about agents until one is set up.
	 */

	let adding = $state<'custom' | 'api' | null>(null);
	let editing = $state<RemoteAgent | null>(null);
	let editingOmp = $state(false);
	let tests = $state<Record<string, Tested | 'testing'>>({});

	const snapshot = $derived(agents.snapshot);
	const path = $derived(repo.info?.path ?? null);

	onMount(() => {
		void agents.load(repo.info?.path ?? null);
	});

	async function act(what: string, run: () => Promise<unknown>): Promise<boolean> {
		try {
			await run();
			await agents.load(path);
			return true;
		} catch (cause) {
			notice.failed(what, agents.failure(cause).message);
			return false;
		}
	}

	function stateLine(agent: LocalAgent): string {
		const found = agent.availability;
		if (found.state === 'available') {
			return [found.version, found.path].filter(Boolean).join(' · ');
		}
		if (found.state === 'broken') return `Doesn’t run · ${found.reason}`;
		return 'Not installed';
	}

	function flip(id: string, jobs: Jobs, job: keyof Jobs) {
		void act('Not changed', () => api.setJobs(id, { ...jobs, [job]: !jobs[job] }));
	}

	async function test(id: string, run: () => Promise<Tested>) {
		tests[id] = 'testing';
		try {
			tests[id] = await run();
		} catch (cause) {
			tests[id] = { ok: false, said: agents.failure(cause).message, ms: 0 };
		}
	}

	async function removeAgent(id: string, name: string) {
		const sure = await dialog.confirm({
			title: `Remove ${name}?`,
			body: 'It is no longer offered in Review or Merger. A stored key is removed from the keychain.',
			confirmLabel: 'Remove',
			danger: true
		});
		if (sure) await act('Not removed', () => api.remove(id));
	}

	function setDefaults(next: Partial<Defaults>, notify?: Partial<Notify>) {
		if (!snapshot) return;
		void act('Not changed', () =>
			api.setDefaults({ ...snapshot.defaults, ...next }, { ...snapshot.notify, ...notify })
		);
	}

	function setRules(next: Partial<RepoRules>) {
		if (!snapshot?.rules || !path) return;
		const rules = { ...snapshot.rules, ...next };
		void act('Not changed', () => api.setRules(path, rules));
	}

	async function addBranch() {
		const pattern = await dialog.prompt({
			title: 'Never land unattended',
			body: 'A branch, or a pattern such as release/*.',
			label: 'Branch',
			confirmLabel: 'Add',
			placeholder: 'release/*'
		});
		if (pattern && snapshot?.rules && !snapshot.rules.neverUnattended.includes(pattern)) {
			setRules({ neverUnattended: [...snapshot.rules.neverUnattended, pattern] });
		}
	}

	function allowed(id: string): boolean {
		return snapshot?.rules?.agents?.includes(id) ?? true;
	}

	function toggleAllowed(id: string) {
		if (!snapshot?.rules) return;
		const every = [...snapshot.local.filter((l) => l.availability.state === 'available').map((l) => l.id), ...snapshot.remote.map((r) => r.id)];
		const now = snapshot.rules.agents ?? every;
		const next = now.includes(id) ? now.filter((a) => a !== id) : [...now, id];
		setRules({ agents: next.length === every.length ? null : next });
	}

	const PROVIDER_NAMES: Record<string, string> = {
		anthropic: 'Anthropic',
		openai: 'OpenAI',
		google: 'Google',
		compatible: 'OpenAI-compatible endpoints'
	};

	const usableNames = $derived(
		snapshot
			? [
					...snapshot.local.filter((l) => l.availability.state === 'available'),
					...snapshot.remote
				].map((a) => ({ id: a.id, name: a.name }))
			: []
	);

	async function takeOffer(take: AgentDefinition[]) {
		await act('Not added', () => api.takeOffer(take));
	}
</script>

<section class="section" id="agents">
	{#if !snapshot}
		{#if agents.error}
			<p class="note error">{agents.error}</p>
		{:else}
			<Loader label="Looking for agents…" />
		{/if}
	{:else}
		<div class="columns">
			<div class="main">
				{#if snapshot.offer.length}
					<div class="offer">
						<span>
							Found in your repositories’ farms:
							{snapshot.offer.map((d) => d.displayName).join(', ')}
						</span>
						<Btn onclick={() => takeOffer(snapshot.offer)}>Add them</Btn>
						<Chip onclick={() => takeOffer([])}>not now</Chip>
					</div>
				{/if}

				<header>
					<h2 class="heading">On this machine</h2>
					<Btn title="Detect again" busy={agents.loading} onclick={() => agents.load(path)}>
						<Icon name="refresh" size="1em" />
					</Btn>
				</header>
				<ul class="list">
					{#each snapshot.local as agent (agent.id)}
						{@const usable = agent.availability.state === 'available'}
						{@const result = tests[agent.id]}
						<li class="agent" class:absent={!usable}>
							<span class="mark"><Icon name="terminal" size="1.05em" /></span>
							<div class="what">
								<span class="name">{agent.name}</span>
								<span
									class="state mono note"
									class:broken={agent.availability.state === 'broken'}
									title={stateLine(agent)}>{stateLine(agent)}</span
								>
								{#if result && result !== 'testing'}
									<span class="tested note" class:ok={result.ok}>
										{result.ok ? `Answered in ${(result.ms / 1000).toFixed(1)} s` : result.said}
									</span>
								{/if}
								{#if agent.provider === 'ohMyPi' && usable}
									<span class="note">Model: {snapshot.omp.model || 'OMP default'}{snapshot.omp.profile ? ` · Profile: ${snapshot.omp.profile}` : ''}</span>
									<Btn onclick={() => (editingOmp = true)}>Model and profile…</Btn>
								{/if}

								{#if agent.provider === 'codex' && usable}
									<div class="group" role="group" aria-label="Codex access">
										<Chip active={!snapshot.codexFullAccess} onclick={() => act('Not changed', () => api.setCodexFullAccess(false))}>Sandboxed</Chip>
										<Chip active={snapshot.codexFullAccess} onclick={() => act('Not changed', () => api.setCodexFullAccess(true))}>Full Access</Chip>
									</div>
									{#if snapshot.codexFullAccess}
										<span class="note">Codex commands can access files outside the repository and use the network.</span>
									{/if}
								{/if}
								{#if agent.provider === 'agy' && usable}
									<div class="group" role="group" aria-label="agy tool permissions">
										<Chip active={!snapshot.agyAutoApprove} onclick={() => act('Not changed', () => api.setAgyAutoApprove(false))}>Configured rules</Chip>
										<Chip active={snapshot.agyAutoApprove} onclick={() => act('Not changed', () => api.setAgyAutoApprove(true))}>Auto-approve tools</Chip>
										</div>
									{#if snapshot.agyAutoApprove}
										<span class="note">agy can run commands and access files without asking.</span>
									{/if}
								{/if}
							</div>
							{#if usable}
								<div class="jobs" role="group" aria-label="What {agent.name} may do">
									<Chip active={agent.jobs.review} title="Offer {agent.name} for reviews" onclick={() => flip(agent.id, agent.jobs, 'review')}>Review</Chip>
									<Chip active={agent.jobs.merge} title="Offer {agent.name} for merges" onclick={() => flip(agent.id, agent.jobs, 'merge')}>Merge</Chip>
									<Chip active={agent.jobs.farm} title="Offer {agent.name} for the farm" onclick={() => flip(agent.id, agent.jobs, 'farm')}>Farm</Chip>
								</div>
								<Btn busy={result === 'testing'} onclick={() => test(agent.id, () => api.testLocal(agent.id))}>Test</Btn>
							{/if}
							{#if agent.custom}
								<Chip danger title="Remove {agent.name}" onclick={() => removeAgent(agent.id, agent.name)}>remove</Chip>
							{/if}
						</li>
					{/each}
				</ul>
				<div class="add">
					<Btn onclick={() => (adding = 'custom')}><Icon name="plus" size="1em" />Add</Btn>
				</div>

				<header><h2 class="heading">Through an API</h2></header>
				{#if snapshot.remote.length}
					<ul class="list">
						{#each snapshot.remote as agent (agent.id)}
							{@const result = tests[agent.id]}
							<li class="agent">
								<span class="mark" title={agent.local ? 'On this machine' : 'Reached over the network'}>
									<Icon name={agent.local ? 'machine' : 'cloud'} size="1.05em" />
								</span>
								<div class="what">
									<span class="name">{agent.name} <span class="mono note">{agent.model}</span></span>
									<span class="state mono note">
										{agent.local ? agent.base : `${agent.keyEnd ? `••••${agent.keyEnd}` : 'no key'} · ${perRun(agent.tokensPerRun)}`}
									</span>
									{#if result && result !== 'testing'}
										<span class="tested note" class:ok={result.ok}>
											{result.ok ? `${result.said} in ${(result.ms / 1000).toFixed(1)} s` : result.said}
										</span>
									{/if}
								</div>
								<div class="jobs" role="group" aria-label="What {agent.name} may do">
									<Chip active={agent.jobs.review} onclick={() => flip(agent.id, agent.jobs, 'review')}>Review</Chip>
									<Chip active={agent.jobs.merge} onclick={() => flip(agent.id, agent.jobs, 'merge')}>Merge</Chip>
								</div>
								<Btn
									busy={result === 'testing'}
									onclick={() =>
										test(agent.id, () =>
											api.testRemote({ id: agent.id, provider: agent.provider, base: agent.base, model: agent.model })
										)}>Test</Btn
								>
								<Chip title="Change {agent.name}" onclick={() => (editing = agent)}>edit</Chip>
								<Chip danger title="Remove {agent.name}" onclick={() => removeAgent(agent.id, agent.name)}>remove</Chip>
							</li>
						{/each}
					</ul>
				{/if}
				<div class="add">
					<Btn onclick={() => (adding = 'api')}><Icon name="plus" size="1em" />Add</Btn>
				</div>

				<header><h2 class="heading">Defaults</h2></header>
				{#each [{ job: 'review', label: 'Review' }, { job: 'merge', label: 'Merge' }] as row (row.job)}
					{@const job = row.job as 'review' | 'merge'}
					<div class="field-row">
						<span class="label">{row.label}</span>
						<select
							class="field-select"
							aria-label="{row.label}: agent"
							value={(job === 'review' ? snapshot.defaults.review : snapshot.defaults.merge) ?? ''}
							onchange={(event) =>
								setDefaults({ [job]: event.currentTarget.value || null } as Partial<Defaults>)}
						>
							<option value="">The first one offered</option>
							{#each agents.usable(job) as option (option.id)}
								<option value={option.id}>{option.name}</option>
							{/each}
						</select>
						<select
							class="field-select"
							aria-label="{row.label}: level"
							value={job === 'review' ? snapshot.defaults.reviewLevel : snapshot.defaults.mergeLevel}
							onchange={(event) =>
								setDefaults({ [`${job}Level`]: event.currentTarget.value as Level } as Partial<Defaults>)}
						>
							{#each LEVELS as level (level.id)}
								<option value={level.id}>{level.label}</option>
							{/each}
						</select>
					</div>
				{/each}
				<div class="field-row">
					<span class="label">Tell me</span>
					<div class="jobs" role="group" aria-label="Notifications">
						<Chip active={snapshot.notify.waiting} title="When an agent waits for you, while Spagitty is not in front" onclick={() => setDefaults({}, { waiting: !snapshot.notify.waiting })}>waiting for you</Chip>
						<Chip active={snapshot.notify.finished} onclick={() => setDefaults({}, { finished: !snapshot.notify.finished })}>finished</Chip>
						<Chip active={snapshot.notify.stopped} onclick={() => setDefaults({}, { stopped: !snapshot.notify.stopped })}>stopped</Chip>
					</div>
				</div>
			</div>

			{#if snapshot.rules && repo.info}
				{@const rules = snapshot.rules}
				<aside class="card rules" aria-label="This repository">
					<Chip>{repo.info.name}</Chip>

					<span class="note">Highest level</span>
					<div class="levels" role="radiogroup" aria-label="Highest level">
						{#each LEVELS as level (level.id)}
							<Chip active={rules.highest === level.id} title={level.label} onclick={() => setRules({ highest: level.id })}>
								{level.short}
							</Chip>
						{/each}
					</div>

					{#if usableNames.length}
						<span class="note">Agents allowed here</span>
						<div class="levels" role="group" aria-label="Agents allowed here">
							{#each usableNames as agent (agent.id)}
								<Chip active={allowed(agent.id)} onclick={() => toggleAllowed(agent.id)}>{agent.name}</Chip>
							{/each}
						</div>
					{/if}

					{#if rules.consent.length}
						<span class="note">Code may go to</span>
						{#each rules.consent as slug (slug)}
							<div class="pair">
								<span>{PROVIDER_NAMES[slug] ?? slug}</span>
								<Chip danger onclick={() => setRules({ consent: rules.consent.filter((c) => c !== slug) })}>revoke</Chip>
							</div>
						{/each}
					{/if}

					<span class="note">Never land unattended</span>
					<div class="levels">
						{#each rules.neverUnattended as pattern (pattern)}
							<Chip
								title="Remove {pattern}"
								onclick={() => setRules({ neverUnattended: rules.neverUnattended.filter((p) => p !== pattern) })}
							>
								<span class="mono">{pattern}</span>
								<Icon name="close" size="0.85em" />
							</Chip>
						{/each}
						<Chip title="Add a branch" onclick={addBranch}><Icon name="plus" size="0.9em" /></Chip>
					</div>

					<hr class="hr" />
					<div class="pair">
						<span>Agents may approve</span>
						<Chip active={rules.verdicts} title="An agent may Approve or Request changes on the host, rather than only comment" onclick={() => setRules({ verdicts: !rules.verdicts })}>
							{rules.verdicts ? 'on' : 'off'}
						</Chip>
					</div>
					<div class="pair">
						<span>Mark agent comments</span>
						<Chip active={rules.markComments} title="Each comment an agent drafted says so on the host: Drafted with …" onclick={() => setRules({ markComments: !rules.markComments })}>
							{rules.markComments ? 'on' : 'off'}
						</Chip>
					</div>
				</aside>
			{/if}
		</div>
	{/if}
</section>

{#if editingOmp && snapshot}
	<OmpSettingsSheet
		initial={snapshot.omp}
		onclose={() => (editingOmp = false)}
		onsave={(options) => act('Not changed', () => api.setOmpOptions(options))}
	/>
{/if}

{#if adding === 'custom'}
	<CustomAgentSheet
		onclose={() => (adding = null)}
		onsave={(definition) => act('Not added', () => api.saveCustom(definition))}
	/>
{/if}
{#if adding === 'api' || editing}
	<ApiAgentSheet
		{editing}
		onclose={() => {
			adding = null;
			editing = null;
		}}
	/>
{/if}

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.columns {
		display: grid;
		grid-template-columns: minmax(0, 720px) minmax(240px, 340px);
		gap: 24px;
		align-items: start;
	}

	.main {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 0;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin-top: 6px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
	}

	.agent {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
	}

	.agent + .agent {
		border-top: 1px solid var(--soft);
	}

	.agent.absent .name {
		color: var(--muted);
	}

	.mark {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		flex: none;
		border-radius: var(--r-field);
		background: var(--sunken);
		color: var(--muted);
	}

	.what {
		display: flex;
		flex-direction: column;
		min-width: 0;
		flex: 1;
	}

	.state {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.state.broken {
		color: var(--warn);
	}

	.tested {
		color: var(--danger);
	}

	.tested.ok {
		color: var(--ok);
	}

	.jobs,
	.levels {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.add {
		display: flex;
	}

	.offer {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--r-panel);
		background: var(--agent-soft);
	}

	.offer span {
		flex: 1;
	}

	.field-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.field-row .label {
		width: 72px;
		flex: none;
		color: var(--muted);
	}

	.field-select {
		font-size: var(--fs-ui);
	}

	.rules {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 8px;
		padding: 14px;
	}

	.pair {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		align-self: stretch;
	}

	.hr {
		align-self: stretch;
	}

	@media (max-width: 1100px) {
		.columns {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
