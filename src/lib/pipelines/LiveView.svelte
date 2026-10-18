<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import PipelineGraph from './PipelineGraph.svelte';
	import { liveGraph, stageOrder } from './live';
	import { ago, clock, mark, tone, word } from './states';
	import { pipelines } from './store.svelte';
	import type { Run } from './types';

	// A second's clock for the elapsed time and "updated N s ago". It lives
	// while this view is mounted and nowhere else.
	let now = $state(Date.now());
	$effect(() => {
		const timer = setInterval(() => { now = Date.now(); }, 1000);
		return () => clearInterval(timer);
	});

	const run = $derived(pipelines.run);
	const liveJob = $derived(pipelines.liveJob);
	const gitLab = $derived(pipelines.host?.repo.kind === 'gitLab');
	const hostName = $derived(pipelines.host?.repo.kind === 'gitHub' ? 'GitHub' : pipelines.host?.repo.kind === 'gitLab' ? 'GitLab' : 'host');
	const jobs = $derived(liveGraph(pipelines.jobs, pipelines.graph));
	const stages = $derived(pipelines.graph?.stages.length ? pipelines.graph.stages : stageOrder(pipelines.jobs));
	const strip = $derived(pipelines.runs.slice(0, 12).reverse());
	const longest = $derived(Math.max(1, ...strip.map((r) => r.duration ?? 0)));
	const logLines = $derived(pipelines.log.split('\n').map((text, index) => ({ text, index })));

	const elapsed = (r: Run) => r.duration ?? (r.started ? Math.max(0, now / 1000 - r.started) : null);
	const latest = (name: string) => pipelines.lists[name]?.runs[0] ?? null;
	const runTone = (r: Run | null) => (r ? tone(r.state === 'completed' ? r.result : r.state) : 'muted');
	const updated = $derived(pipelines.updated ? `updated ${Math.max(0, Math.floor((now - pipelines.updated) / 1000))} s ago` : 'waiting for the host');
	const stepAt = $derived.by(() => {
		const steps = liveJob?.steps ?? [];
		const index = steps.findIndex((s) => s.state === 'in_progress');
		return index >= 0 ? { number: index + 1, of: steps.length, name: steps[index].name } : null;
	});
	const failed = (r: Run) => ['failure', 'failed', 'timed_out'].includes(r.result);
</script>

{#snippet watched(name: string, removable: boolean)}
	{@const last = latest(name)}
	<div class="watch" class:on={pipelines.branch === name}>
		<button class="watch-main" onclick={() => pipelines.chooseBranch(name)} title={last ? `${word(last.state, last.result)} · ${last.event}` : 'No runs read yet'}>
			<span class="dot" data-tone={runTone(last)} aria-hidden="true"></span>
			<span class="watch-text">
				<span class="mono watch-name">{name}</span>
				<span class="note watch-run">{last ? `${last.name} #${last.id}` : 'no runs'}</span>
			</span>
			{#if last}<span class="age">{ago(last.started, now)}</span>{/if}
		</button>
		{#if removable}
			<button class="unwatch" aria-label={`Stop watching ${name}`} title="Stop watching" onclick={() => pipelines.watch(name, true)}>×</button>
		{/if}
	</div>
{/snippet}

<div class="body">
	<aside class="pane side" aria-label="Watched branches">
		<span class="k">Branches</span>
		{#each pipelines.branches as name (name)}
			{@render watched(name, name !== pipelines.branches[0])}
		{/each}
		<span class="k">Requests</span>
		{#each pipelines.requests as name (name)}
			{@render watched(name, false)}
		{:else}
			<p class="note side-note">No open requests of yours.</p>
		{/each}
		{#if !pipelines.host}
			<p class="note side-note">Connect a GitHub or GitLab remote in Settings.</p>
		{:else if pipelines.host.repo.kind === 'bitbucket'}
			<p class="note side-note">Bitbucket · soon</p>
		{/if}
	</aside>

	<section class="pane run" aria-label="Pipeline run">
		{#if !run}
			<div class="empty">
				<p>{pipelines.reading ? 'Reading runs…' : pipelines.branch ? `No runs on ${pipelines.branch} yet.` : 'Pick a branch.'}</p>
				<span class="note">{updated}</span>
			</div>
		{:else}
			<header class="run-head">
				<strong class="run-name">{run.name} <span class="number">#{run.id}</span></strong>
				<span class="state" data-tone={runTone(run)}>
					<span aria-hidden="true">{mark(run.state === 'completed' ? run.result : run.state)}</span>
					{word(run.state, run.result)}{elapsed(run) != null ? ` · ${clock(elapsed(run))}` : ''}
				</span>
				<span class="mono note">{run.branch} · {run.commit.slice(0, 7)} · {run.event}</span>
				<span class="grow"></span>
				{#if strip.length > 1}
					<div class="strip" aria-label="Last twelve runs">
						{#each strip as r (r.id)}
							<button
								class:selected={run.id === r.id}
								data-tone={runTone(r)}
								style:height={`${8 + (20 * (r.duration ?? longest)) / longest}px`}
								title={`#${r.id} · ${word(r.state, r.result)} · ${clock(r.duration)}`}
								aria-label={`Open run ${r.id}`}
								onclick={() => pipelines.openRun(r.id)}
							></button>
						{/each}
					</div>
				{/if}
				<Btn danger disabled={run.state === 'completed' || !pipelines.canWrite} busy={pipelines.busy} onclick={() => pipelines.send('cancel')}>Cancel</Btn>
				<Btn disabled={!pipelines.canWrite || run.state !== 'completed' || !failed(run)} busy={pipelines.busy} onclick={() => pipelines.send('rerunFailed')}>Re-run failed</Btn>
				<a class="external" href={run.url} target="_blank" rel="noreferrer">Open on {hostName} ↗</a>
			</header>
			{#if pipelines.graphWarning}<p class="hint" role="status">{pipelines.graphWarning}</p>{/if}

			<div class="graph-area">
				{#if jobs.length}
					<PipelineGraph {jobs} {stages} mode={gitLab ? 'stages' : 'needs'} selected={String(liveJob?.id ?? '')} onselect={(id) => pipelines.selectLiveJob(Number(id))} />
				{:else}
					<p class="empty note">{pipelines.reading ? 'Reading jobs…' : 'This run has no jobs yet.'}</p>
				{/if}
				{#if pipelines.moreJobs}<button class="more" onclick={() => pipelines.readJobs(true)}>More jobs</button>{/if}
			</div>

			{#if liveJob}
				<div class="log-head">
					<span class="dot" data-tone={tone(liveJob.state === 'completed' ? liveJob.result : liveJob.state)} aria-hidden="true"></span>
					<strong>{liveJob.name}</strong>
					<span class="note">
						{#if stepAt}step {stepAt.number} of {stepAt.of} · {stepAt.name}{:else}{liveJob.runner || 'runner not named'}{liveJob.duration != null ? ` · ${clock(liveJob.duration)}` : ''}{/if}
					</span>
					<span class="grow"></span>
					<span class="note">{gitLab && liveJob.state !== 'completed' ? 'partial trace · polled' : liveJob.state === 'completed' ? 'completed log' : 'steps only until it ends'}</span>
				</div>
				{#if liveJob.steps.length}
					<div class="steps">
						{#each liveJob.steps as step (step.number)}
							<span class="step" data-tone={tone(step.result || step.state)} title={step.result || step.state}><span aria-hidden="true">{mark(step.result || step.state)}</span> {step.name}</span>
						{/each}
					</div>
				{/if}
				{#if !gitLab && liveJob.state !== 'completed'}
					<p class="note log-info">GitHub gives the log once the job ends; the steps above refresh while it runs.</p>
				{/if}
				<div class="logs" aria-label="Job log">
					<VirtualRows items={logLines} key={(line) => String(line.index)} estimate={() => 22} label="Log lines">
						{#snippet row(line)}<pre>{line.text || ' '}</pre>{/snippet}
					</VirtualRows>
				</div>
			{/if}
			<div class="updated note" role="status">{updated}{pipelines.reading ? ' · reading' : ''}</div>
		{/if}
	</section>
</div>

<style>
	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 10px;
		padding: 0 12px 12px;
	}

	.pane {
		min-height: 0;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		overflow: hidden;
	}

	.k {
		padding: 10px 10px 4px;
		font-size: var(--fs-mono);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--muted);
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.grow {
		flex: 1;
	}

	.dot {
		flex: none;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--muted);
	}

	[data-tone='ok'].dot { background: var(--ok); }
	[data-tone='waits'].dot { background: var(--warn); }
	[data-tone='running'].dot { background: var(--lane-5); }
	[data-tone='failed'].dot { background: var(--danger); }

	/* Watched */
	.side {
		flex: 0 0 260px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 6px 8px 12px;
		overflow: auto;
	}

	.watch {
		position: relative;
		display: flex;
		align-items: center;
		border-radius: var(--r-button);
	}

	.watch:hover {
		background: var(--hover);
	}

	.watch.on {
		background: color-mix(in srgb, var(--accent) 12%, var(--surface-2));
	}

	.watch-main {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		text-align: left;
	}

	.watch-text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.watch-name,
	.watch-run {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.age {
		flex: none;
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.unwatch {
		position: absolute;
		top: 4px;
		right: 4px;
		width: 20px;
		height: 20px;
		border-radius: var(--r-pill);
		color: var(--muted);
		opacity: 0;
	}

	.watch:hover .unwatch,
	.unwatch:focus-visible {
		opacity: 1;
	}

	.unwatch:hover {
		color: var(--ink);
		background: var(--soft);
	}

	.side-note {
		margin: 4px 10px;
	}

	/* Run */
	.run {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 6px;
		margin: 0;
		padding: 24px;
		text-align: center;
	}

	.empty p {
		margin: 0;
	}

	.run-head {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
		padding: 12px 16px;
		border-bottom: 1px solid var(--soft);
	}

	.run-name {
		font-size: var(--fs-title);
		font-weight: 500;
	}

	.number {
		color: var(--muted);
	}

	.state {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		border-radius: var(--r-pill);
		border: 1px solid var(--soft);
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	.state[data-tone='ok'] { color: var(--ok); border-color: color-mix(in srgb, var(--ok) 45%, transparent); }
	.state[data-tone='running'] { color: var(--lane-5); border-color: color-mix(in srgb, var(--lane-5) 45%, transparent); }
	.state[data-tone='failed'] { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 45%, transparent); }
	.state[data-tone='waits'] { color: var(--warn); border-color: color-mix(in srgb, var(--warn) 45%, transparent); }

	.strip {
		display: flex;
		align-items: flex-end;
		gap: 3px;
		height: 28px;
	}

	.strip button {
		width: 7px;
		border-radius: 2px;
		background: var(--muted);
	}

	.strip button[data-tone='ok'] { background: var(--ok); }
	.strip button[data-tone='failed'] { background: var(--danger); }
	.strip button[data-tone='running'] { background: var(--lane-5); }
	.strip button[data-tone='waits'] { background: var(--warn); }

	.strip button.selected {
		outline: 1px solid var(--ink);
		outline-offset: 1px;
	}

	.external {
		display: inline-flex;
		align-items: center;
		height: 30px;
		padding: 0 12px;
		border-radius: var(--r-button);
		border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
		background-color: var(--surface-veil);
		box-shadow: var(--shadow-1);
		color: var(--ink);
		text-decoration: none;
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	.hint {
		flex: none;
		margin: 0;
		padding: 8px 16px;
		border-bottom: 1px solid var(--soft);
		background: color-mix(in srgb, var(--warn) 7%, transparent);
		font-size: var(--fs-secondary);
	}

	.graph-area {
		flex: 1 1 45%;
		min-height: 160px;
		display: flex;
		flex-direction: column;
	}

	.more {
		align-self: flex-start;
		margin: 0 16px 8px;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.log-head {
		flex: none;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 16px 6px;
		border-top: 1px solid var(--soft);
	}

	.steps {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		gap: 4px 14px;
		padding: 0 16px 8px;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.step[data-tone='ok'] span { color: var(--ok); }
	.step[data-tone='running'] { color: var(--ink); }
	.step[data-tone='running'] span { color: var(--lane-5); }
	.step[data-tone='failed'] span { color: var(--danger); }

	.log-info {
		margin: 0 16px 8px;
	}

	.logs {
		flex: 1 1 40%;
		min-height: 140px;
		margin: 0 16px 10px;
		padding: 8px 0;
		border-radius: var(--r-panel);
		background: var(--sunken);
		overflow: hidden;
	}

	.logs pre {
		margin: 0;
		padding: 1px 14px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		color: var(--ink);
	}

	.updated {
		flex: none;
		padding: 6px 16px;
		border-top: 1px solid var(--soft);
	}

	@media (max-width: 1000px) {
		.side {
			flex-basis: 200px;
		}
	}

	@media (max-width: 760px) {
		.body {
			flex-wrap: wrap;
			overflow: auto;
		}

		.side {
			flex: 1 1 100%;
			max-height: 180px;
		}

		.run {
			flex: 1 1 100%;
			min-height: 520px;
		}
	}
</style>
