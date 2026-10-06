<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import * as farmApi from '../api';
	import { farmStore } from '../store.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import TaskChip from './TaskChip.svelte';
	import ContributedActions from '$lib/extensions/ContributedActions.svelte';
	import ExtensionPanels from '$lib/extensions/ExtensionPanels.svelte';
	import { extensions } from '$lib/extensions/store.svelte';
	import { duration, originLine, TASK_KIND_LABELS, verificationLine } from '../describe';
	import type { AgentStatus, TaskDetail } from '../types';

	/**
	 * Everything known about one task (FEAT-073).
	 *
	 * The plan's §23, and the place where being a Git client pays: the branch,
	 * the worktree, the diff and the commits are all things Spagitty already
	 * knows how to show, so the panel links to them rather than reimplementing
	 * them.
	 */
	interface Props {
		detail: TaskDetail;
		agents: AgentStatus[];
		transcript: string[];
		busy: boolean;
		onrun: (agent: string | null) => void;
		onstop: () => void;
		onretry: () => void;
		onverify: () => void;
		onreview: () => void;
		onmerge: () => void;
		onready: () => void;
		/** Ask an agent to break this task into smaller ones (FEAT-076). */
		ondecompose: () => void;
		/** This task's own row in the outline, so a container can say so. */
		children?: { done: number; total: number } | null;
		onedit: () => void;
		ondelete: () => void;
		onopenDiff: (branch: string) => void;
	}

	let {
		detail,
		agents,
		transcript,
		busy,
		onrun,
		onstop,
		onretry,
		onverify,
		onreview,
		onmerge,
		onready,
		ondecompose,
		children = null,
		onedit,
		ondelete,
		onopenDiff
	}: Props = $props();

	let checkingExtra = $state(false);
    async function checkExtra() {
        checkingExtra = true;
        try {
            const result = await farmApi.reviewSupplemental(task.id);
            if (result.outcome === 'pass') notice.ok('Additional review completed', result.summary);
            else notice.failed('Additional review needs attention', result.summary);
            await farmStore.refresh();
        }
        catch (error) { notice.failed('Additional review failed', error); }
        finally { checkingExtra = false; }
    }
	const task = $derived(detail.task);
	/** A task something was cut out of: a heading, and never run itself. */
	const container = $derived((children?.total ?? 0) > 0);
	const running = $derived(task.status === 'running' || task.status === 'verification');
	/** A task nothing is going to move on its own. */
	const stalled = $derived(task.status === 'blocked' || task.status === 'failed');

	// The task an extension's farm-task actions and panels are about (FEAT-096):
	// its worktree is what they read, never the user's checkout.
	$effect(() => {
		extensions.setContext('farmTask', {
			taskId: task.id,
			taskHasCommit: Boolean(task.branch && task.worktree),
			workdir: task.worktree,
			base: task.mergeTarget ?? null
		});
		return () => extensions.setContext('farmTask', null);
	});
</script>

<div class="detail">
	<header class="head">
		<span class="id mono">{task.id}</span>
		<TaskChip status={task.status} />
	</header>

	<h2 class="title">{task.title}</h2>
	{#if task.description}
		<p class="note body">{task.description}</p>
	{/if}

	{#if task.note}
		<p class="reason" class:bad={stalled}>{task.note}</p>
	{/if}

	{#if container}
		<p class="note">
			This was broken down. It finishes when the {children?.total} tasks under it do —
			{children?.done} so far — and nothing runs against it directly.
		</p>
	{/if}

	<div class="actions">
		{#if task.status === 'draft'}
			<Btn primary disabled={busy} onclick={onready}>Add to the plan</Btn>
		{/if}
		{#if running}
			<Btn danger disabled={busy} onclick={onstop}>Stop</Btn>
		{:else if stalled}
			<Btn primary disabled={busy} onclick={onretry}>Try again</Btn>
		{:else if !container && task.status !== 'done' && task.status !== 'cancelled'}
			<Btn primary disabled={busy} onclick={() => onrun(null)}>Run</Btn>
		{/if}
		{#if !container && !running && task.status !== 'done' && task.status !== 'cancelled'}
			<!--
				Offered on any task that has not been broken down, because
				"this is bigger than I thought" is something you find out at
				any point, including after a failed attempt.
			-->
			<Btn disabled={busy} onclick={ondecompose}>Break it down</Btn>
		{/if}
		{#if task.status === 'review'}
			<Btn disabled={busy} onclick={onreview}>Send for review</Btn>
			<Btn primary quiet disabled={busy} onclick={onmerge}>Approve and merge</Btn>
		{/if}
		{#if task.worktree && !running}
			<Btn disabled={busy} onclick={onverify}>Verify</Btn>
		{/if}
		<Btn disabled={busy} onclick={onedit}>Edit</Btn>
		<Btn danger disabled={busy || running} onclick={ondelete}>Delete</Btn>
        {#if task.status === 'review'}
            <Btn disabled={busy || checkingExtra} onclick={checkExtra}>{checkingExtra ? 'Reviewing…' : 'Review for the additional gate'}</Btn>
        {/if}
		<ContributedActions context="farmTask" />
	</div>

	{#if !running && task.status !== 'done' && agents.length > 1}
		<div class="line">
			<span class="note label">Run with</span>
			<div class="chips">
				{#each agents as agent (agent.definition.id)}
					<Chip disabled={busy} onclick={() => onrun(agent.definition.id)}>
						{agent.definition.displayName}
					</Chip>
				{/each}
			</div>
		</div>
	{/if}

    {#if farmStore.farm?.supplementalEvidence?.[task.id]}
        <p class="note">Additional review: {farmStore.farm.supplementalEvidence[task.id].summary}</p>
    {/if}
	<dl class="facts">
		<dt>Asked for by</dt>
		<dd>{originLine(task.origin)}</dd>

		<dt>Kind</dt>
		<dd>{TASK_KIND_LABELS[task.kind]}</dd>

		<dt>Agent</dt>
		<dd>{task.assignedAgent ?? 'Not assigned'}</dd>

		{#if task.dependsOn.length > 0}
			<dt>Depends on</dt>
			<dd class="mono">{task.dependsOn.join(', ')}</dd>
		{/if}

		<dt>Branch</dt>
		<dd class="mono">
			{#if task.branch}
				<button class="link" onclick={() => onopenDiff(task.branch ?? '')}>{task.branch}</button>
			{:else}
				Not created yet
			{/if}
		</dd>

		{#if task.worktree}
			<dt>Worktree</dt>
			<dd class="mono wrap">{task.worktree}</dd>
		{/if}

		<dt>Attempts</dt>
		<dd>{task.attempts}</dd>

		<dt>Verification</dt>
		<dd class:bad={detail.verification !== null && !detail.verification.passed}>
			{verificationLine(detail.verification)}
		</dd>
	</dl>

	{#if task.allowedPaths.length > 0}
		<section class="block">
			<h3 class="heading">Allowed paths</h3>
			<ul class="list mono">
				{#each task.allowedPaths as path (path)}
					<li>{path}</li>
				{/each}
			</ul>
		</section>
	{/if}

	{#if task.acceptanceCriteria.length > 0}
		<section class="block">
			<h3 class="heading">Acceptance criteria</h3>
			<ul class="list">
				{#each task.acceptanceCriteria as criterion (criterion)}
					<li>{criterion}</li>
				{/each}
			</ul>
		</section>
	{/if}

	{#if detail.verification && detail.verification.results.length > 0}
		<section class="block">
			<h3 class="heading">Checks</h3>
			<ul class="checks">
				{#each detail.verification.results as result (result.command)}
					<li class:bad={!result.passed}>
						<span class="mark">{result.passed ? '✓' : '✗'}</span>
						<span class="mono">{result.command}</span>
						<span class="note">{duration(result.durationMs)}</span>
						{#if !result.passed && result.output}
							<pre class="output">{result.output}</pre>
						{/if}
					</li>
				{/each}
			</ul>
		</section>
	{/if}

	{#if detail.review}
		<section class="block">
			<h3 class="heading">
				Review
				<span class="note">{detail.review.decision === 'approve' ? 'Approved' : 'Changes asked for'}</span>
			</h3>
			{#if detail.review.summary}
				<p class="note body">{detail.review.summary}</p>
			{/if}
			{#each detail.review.issues as issue, index (index)}
				<p class="issue" class:bad={issue.severity === 'high'}>
					<span class="mono">{issue.file}</span>
					{issue.message}
				</p>
			{/each}
		</section>
	{/if}

	<ExtensionPanels location="farmTask" revision={task.id} onsend={async (extension, record, findings) => { await extensions.send(extension, record, findings); await farmStore.refresh(); }} />

	{#if detail.handoff && detail.handoff.summary}
		<section class="block">
			<h3 class="heading">What the agent said</h3>
			<p class="note body">{detail.handoff.summary}</p>
			{#if detail.handoff.filesChanged.length > 0}
				<ul class="list mono">
					{#each detail.handoff.filesChanged as file (file)}
						<li>{file}</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}

	{#if transcript.length > 0}
		<section class="block">
			<h3 class="heading">Transcript</h3>
			<pre class="transcript">{transcript.join('\n')}</pre>
		</section>
	{/if}

	{#if detail.runs.length > 0}
		<section class="block">
			<h3 class="heading">Runs</h3>
			<ul class="runs">
				{#each detail.runs as run (run.id)}
					<li>
						<span class="note">{run.phase}</span>
						<span>{run.agent}</span>
						<span class="note">{run.outcome.state}</span>
						<span class="note">
							{duration(run.endedMs === null ? null : run.endedMs - run.startedMs)}
						</span>
					</li>
				{/each}
			</ul>
		</section>
	{/if}
</div>

<style>
	.detail {
		flex: 1;
		width: 100%;
		height: 100%;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 16px 24px 24px 16px;
		overflow-y: auto;
		scrollbar-gutter: stable;
	}

	.head {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.id {
		color: var(--muted);
		font-size: var(--fs-mono);
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.body {
		margin: 0;
		white-space: pre-wrap;
	}

	.reason {
		margin: 0;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.bad {
		color: var(--danger);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.line {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.label {
		flex: none;
		font-size: var(--fs-mono);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
	}

	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 3px 12px;
		margin: 0;
		font-size: var(--fs-secondary);
	}

	.facts dt {
		color: var(--muted);
	}

	.facts dd {
		margin: 0;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.wrap {
		overflow-wrap: anywhere;
		white-space: normal;
	}

	.link {
		background: none;
		border: none;
		padding: 0;
		color: var(--accent);
		text-decoration: underline;
		font: inherit;
		cursor: pointer;
	}

	.block {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-secondary);
		font-weight: 600;
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.list,
	.checks,
	.runs {
		margin: 0;
		padding-left: 16px;
		font-size: var(--fs-secondary);
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.checks,
	.runs {
		list-style: none;
		padding-left: 0;
	}

	.checks li,
	.runs li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 6px;
	}

	.mark {
		color: var(--ok);
	}

	.checks li.bad .mark {
		color: var(--danger);
	}

	.output,
	.transcript {
		margin: 4px 0 0;
		padding: 6px 8px;
		width: 100%;
		max-height: 220px;
		overflow: auto;
		border-radius: var(--r-field);
		background-color: var(--stripe);
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.issue {
		margin: 0;
		font-size: var(--fs-secondary);
		display: flex;
		gap: 6px;
	}
</style>
