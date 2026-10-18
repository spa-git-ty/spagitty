<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import PipelineGraph from './PipelineGraph.svelte';
	import { mark, tone } from './states';
	import { pipelines } from './store.svelte';

	const workflow = $derived(pipelines.workflow);
	const picked = $derived(pipelines.job);
	const workflows = $derived(pipelines.local?.workflows ?? []);
	const groups = $derived(
		[
			{ kind: 'gitHub', label: 'GitHub Actions' },
			{ kind: 'gitLab', label: 'GitLab CI' }
		].map((group) => ({ ...group, files: workflows.filter((w) => w.kind === group.kind) }))
	);

	// GitLab draws by stage, GitHub by `needs`; the person can switch either way.
	let chosenMode = $state<'needs' | 'stages' | null>(null);
	const mode = $derived(chosenMode ?? (workflow?.stages.length ? 'stages' : 'needs'));

	let reference = $state(''), base = $state(''), start = $state('HEAD~1'), end = $state('HEAD');
	$effect(() => {
		reference = pipelines.simulation.reference;
		base = pipelines.simulation.base;
		start = pipelines.from;
		end = pipelines.to;
	});

	const jobs = $derived(workflow?.jobs ?? []);
	const counts = $derived({
		runs: jobs.filter((j) => j.state === 'runs').length,
		approval: jobs.filter((j) => j.state === 'approval').length,
		skipped: jobs.filter((j) => j.state === 'skipped').length,
		host: jobs.filter((j) => j.state === 'host').length
	});
	/** Every job is out because the workflow itself does not start: say that once. */
	const notStarted = $derived(jobs.length > 0 && jobs.every((j) => j.state !== 'runs' && j.state !== 'approval') && jobs.some((j) => j.reason.startsWith('The workflow')) ? jobs.find((j) => j.reason.startsWith('The workflow'))?.reason : null);

	let allProblems = $state(false);
	const problems = $derived(workflow?.problems ?? []);
	const shown = $derived(allProblems ? problems : problems.slice(0, 1));

	const eventWords: Record<string, string> = { push: 'push', pull_request: 'pull request', tag: 'tag push', schedule: 'schedule', manual: 'manual run' };
	const headline = (state: string) =>
		state === 'runs'
			? `Runs on this ${eventWords[pipelines.simulation.event] ?? 'event'}`
			: state === 'approval'
				? 'Waits for approval'
				: state === 'skipped'
					? 'Skipped'
					: 'Decided on the host';
	const fileName = (file: string) => (file.startsWith('.github/workflows/') ? file.slice('.github/workflows/'.length) : file);
</script>

<div class="body">
	<aside class="pane files" aria-label="Workflow files">
		{#each groups as group (group.kind)}
			{#if group.files.length}
				<div class="group">
					<span class="k">{group.label}</span>
					{#each group.files as w (w.file)}
						<button class="file" class:on={workflow?.file === w.file} title={w.problems.length ? `${w.problems.length} problems` : w.file} onclick={() => pipelines.selectFile(w.file)}>
							<span class="dot" class:bad={w.problems.length > 0} aria-hidden="true"></span>
							<span class="file-name mono">{fileName(w.file)}</span>
							<span class="count">{w.jobs.length}</span>
						</button>
					{/each}
				</div>
			{/if}
		{/each}
		{#if !workflows.length}
			<p class="note">{pipelines.loading || pipelines.previewing ? 'Reading workflows…' : 'No workflow files in the working copy.'}</p>
		{/if}
	</aside>

	<section class="pane graph" aria-label="Preview">
		<form class="bar" onsubmit={(e) => { e.preventDefault(); void pipelines.preview({ reference, base }, pipelines.mode, start, end); }}>
			<span class="note">If</span>
			<select class="sel" aria-label="Simulation event" value={pipelines.simulation.event} onchange={(e) => pipelines.preview({ event: e.currentTarget.value })}>
				<option value="push">push</option>
				<option value="pull_request">pull request</option>
				<option value="tag">tag</option>
				<option value="schedule">schedule</option>
				<option value="manual">manual</option>
			</select>
			<span class="note">to</span>
			<input class="sel ref" aria-label="Simulation branch or tag" placeholder="branch or tag" bind:value={reference} onchange={() => pipelines.preview({ reference })} />
			{#if pipelines.simulation.event === 'pull_request'}
				<span class="note">from base</span>
				<input class="sel ref" aria-label="Pull request base" placeholder="main" bind:value={base} onchange={() => pipelines.preview({ base })} />
			{/if}
			<span class="note">with</span>
			<select class="sel" aria-label="Changed files" value={pipelines.mode} onchange={(e) => pipelines.preview({}, e.currentTarget.value, start, end)}>
				<option value="working">working copy · {pipelines.local?.changed.length ?? 0} files</option>
				<option value="range">commit range</option>
			</select>
			{#if pipelines.mode === 'range'}
				<input class="sel ref" aria-label="Range from" bind:value={start} />
				<span class="note">…</span>
				<input class="sel ref" aria-label="Range to" bind:value={end} />
				<Btn>Apply</Btn>
			{/if}
			<span class="grow"></span>
			<div class="seg" role="group" aria-label="Graph layout">
				<button type="button" class:on={mode === 'stages'} aria-pressed={mode === 'stages'} onclick={() => { chosenMode = 'stages'; }}>Stages</button>
				<button type="button" class:on={mode === 'needs'} aria-pressed={mode === 'needs'} onclick={() => { chosenMode = 'needs'; }}>Needs</button>
			</div>
		</form>
		{#if pipelines.local?.changesWarning}
			<p class="hint" role="status">{pipelines.local.changesWarning} Path conditions are decided on the host.</p>
		{/if}
		{#if notStarted}
			<p class="hint" role="status">No pipeline: {notStarted.charAt(0).toLowerCase()}{notStarted.slice(1)}</p>
		{/if}

		{#if !workflow}
			<p class="empty note">{pipelines.loading || pipelines.previewing ? 'Reading workflows…' : 'Pick a workflow file.'}</p>
		{:else if !jobs.length}
			<p class="empty note">{problems.length ? 'This file could not be drawn. Its problems are listed below.' : 'This file defines no jobs.'}</p>
		{:else}
			<PipelineGraph {jobs} stages={workflow.stages} {mode} selected={picked?.id} onselect={(id) => pipelines.selectJob(id)}>
				{#snippet footer()}
					<div class="legend note">
						<span><span class="ok">●</span> runs {counts.runs}</span>
						<span><span class="waits">‖</span> waits for approval {counts.approval}</span>
						<span><span>–</span> skipped by if / paths {counts.skipped}</span>
						<span><span>?</span> decided on the host {counts.host}</span>
						<span><span class="mono">×N</span> matrix</span>
					</div>
				{/snippet}
			</PipelineGraph>
		{/if}

		{#if problems.length}
			<div class="problems" aria-label="Workflow problems">
				{#each shown as problem, index (index)}
					<div class="problem">
						<span class="bang" aria-hidden="true">!</span>
						<span class="where mono">{fileName(problem.file)}:{problem.line}</span>
						<span class="what">{problem.message}</span>
						<Btn onclick={() => pipelines.openSource(problem.file, problem.line)}>Go to line</Btn>
					</div>
				{/each}
				{#if problems.length > 1}
					<button class="more" onclick={() => { allProblems = !allProblems; }}>{allProblems ? 'Show one' : `${problems.length - 1} more`}</button>
				{/if}
			</div>
		{/if}
	</section>

	<aside class="pane detail" aria-label="Selected job">
		{#if picked}
			<div class="title">
				<span class="job-name">{picked.name}</span>
				<button class="mono source-link" title="Open the line in the working copy" onclick={() => pipelines.openSource(picked.file, picked.line)}>{fileName(picked.file)}:{picked.line}{picked.needs.length && !picked.implicit ? ` · needs ${picked.needs.join(', ')}` : ''}</button>
			</div>
			<div class="reason" data-tone={tone(picked.state)}>
				<strong><span aria-hidden="true">{mark(picked.state)}</span> {headline(picked.state)}</strong>
				<span>{picked.reason}</span>
				{#if picked.condition}<span class="mono">{picked.condition}</span>{/if}
			</div>
			<dl>
				<dt>runner</dt><dd>{picked.runsOn.join(', ') || 'host default'}</dd>
				<dt>needs</dt><dd>{picked.needs.join(', ') || 'none'}</dd>
				<dt>matrix</dt><dd>{picked.matrix.length ? `${picked.matrix.length} combinations` : 'none'}</dd>
				<dt>services</dt><dd>{picked.services.join(', ') || 'none'}</dd>
				<dt>timeout</dt><dd>{picked.timeout || 'host default'}</dd>
				<dt>secrets</dt><dd>{picked.secrets.join(', ') || 'none named'}</dd>
				{#if picked.environment}<dt>environment</dt><dd>{picked.environment}</dd>{/if}
			</dl>
			{#if picked.matrix.length}
				<details>
					<summary class="note">Matrix values</summary>
					<pre>{JSON.stringify(picked.matrix, null, 2)}</pre>
				</details>
			{/if}
			{#if picked.steps.length}
				<span class="k">Steps</span>
				<ol class="steps">
					{#each picked.steps as step, index (index)}
						<li title={step.command}><span class="n">{index + 1}</span><span class="step-name">{step.name || step.command}</span></li>
					{/each}
				</ol>
			{/if}
		{:else}
			<p class="note">Select a job for its details.</p>
		{/if}
		{#if workflow?.kind === 'gitLab' && pipelines.host?.repo.kind === 'gitLab'}
			<div class="lint">
				<Btn busy={pipelines.busy} onclick={() => pipelines.send('lint')}>Check with GitLab lint</Btn>
				{#if pipelines.lint}<pre>{pipelines.lint}</pre>{/if}
			</div>
		{/if}
	</aside>
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
		padding: 0 10px 4px;
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

	/* Files */
	.files {
		flex: 0 0 210px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 14px 8px;
		overflow: auto;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.file {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 7px 10px;
		border-radius: var(--r-button);
		text-align: left;
	}

	.file:hover {
		background: var(--hover);
	}

	.file.on {
		background: color-mix(in srgb, var(--accent) 12%, var(--surface-2));
	}

	.dot {
		flex: none;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--ok);
	}

	.dot.bad {
		background: var(--warn);
	}

	.file-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.count {
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.files .note {
		padding: 0 10px;
	}

	/* Graph */
	.graph {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.bar {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		padding: 10px 14px;
		border-bottom: 1px solid var(--soft);
	}

	.sel {
		height: 30px;
		padding-left: 10px;
		border-radius: var(--r-field);
		background-color: var(--sunken);
		border: 1px solid var(--pane-edge);
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.ref {
		width: 130px;
		padding-right: 10px;
	}

	.seg {
		display: inline-flex;
		padding: 3px;
		border-radius: var(--r-pill);
		background: var(--sunken);
		border: 1px solid var(--pane-edge);
	}

	.seg button {
		padding: 3px 14px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.seg button.on {
		background: var(--surface-2);
		color: var(--ink);
		font-weight: 600;
		box-shadow: var(--shadow-1);
	}

	.hint {
		flex: none;
		margin: 0;
		padding: 8px 14px;
		border-bottom: 1px solid var(--soft);
		background: color-mix(in srgb, var(--warn) 7%, transparent);
		font-size: var(--fs-secondary);
	}

	.empty {
		flex: 1;
		display: grid;
		place-items: center;
		margin: 0;
		padding: 24px;
		text-align: center;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 16px;
		margin-top: 18px;
	}

	.ok {
		color: var(--ok);
	}

	.waits {
		color: var(--warn);
	}

	.problems {
		flex: none;
		max-height: 40%;
		overflow: auto;
		border-top: 1px solid var(--soft);
		background: color-mix(in srgb, var(--warn) 7%, transparent);
	}

	.problem {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 14px;
		font-size: var(--fs-secondary);
	}

	.bang {
		flex: none;
		color: var(--warn);
		font-weight: 700;
	}

	.where {
		flex: none;
		white-space: nowrap;
	}

	.what {
		flex: 1;
		min-width: 0;
	}

	.more {
		padding: 0 14px 8px 34px;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	/* Job */
	.detail {
		flex: 0 0 285px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 16px;
		overflow: auto;
	}

	.title {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.job-name {
		font-size: var(--fs-title);
		font-weight: 600;
		overflow-wrap: anywhere;
	}

	.source-link {
		color: var(--muted);
		text-align: left;
		overflow-wrap: anywhere;
	}

	.source-link:hover {
		color: var(--ink);
		text-decoration: underline;
	}

	.reason {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px 12px;
		border-radius: var(--r-panel);
		background: var(--sunken);
		border: 1px solid var(--pane-edge);
		font-size: var(--fs-secondary);
		overflow-wrap: anywhere;
	}

	.reason[data-tone='ok'] {
		background: color-mix(in srgb, var(--ok) 9%, var(--sunken));
		border-color: color-mix(in srgb, var(--ok) 35%, transparent);
	}

	.reason[data-tone='waits'] {
		background: color-mix(in srgb, var(--warn) 9%, var(--sunken));
		border-color: color-mix(in srgb, var(--warn) 35%, transparent);
	}

	.reason[data-tone='skipped'] {
		border-style: dashed;
	}

	dl {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 6px 14px;
		margin: 0;
		font-size: var(--fs-secondary);
	}

	dt {
		color: var(--muted);
	}

	dd {
		margin: 0;
		overflow-wrap: anywhere;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.steps {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin: -8px 0 0;
		padding: 0;
		list-style: none;
	}

	.steps li {
		display: flex;
		gap: 12px;
		padding: 8px 12px;
		border-radius: var(--r-field);
		background: var(--surface-2);
		border: 1px solid var(--pane-edge);
		font-size: var(--fs-secondary);
	}

	.n {
		flex: none;
		color: var(--muted);
	}

	.step-name {
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.lint {
		margin-top: auto;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	pre {
		max-width: 100%;
		margin: 0;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	@media (max-width: 1000px) {
		.files {
			flex-basis: 165px;
		}

		.detail {
			flex-basis: 230px;
			padding: 12px;
		}
	}

	@media (max-width: 760px) {
		.body {
			flex-wrap: wrap;
			overflow: auto;
		}

		.files {
			flex: 1 1 100%;
			max-height: 140px;
		}

		.graph {
			flex: 1 1 100%;
			min-height: 420px;
		}

		.detail {
			flex: 1 1 100%;
		}
	}
</style>
