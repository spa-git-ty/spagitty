<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import LiveView from './LiveView.svelte';
	import PreviewView from './PreviewView.svelte';
	import { pipelines } from './store.svelte';
	import type { Hosted } from './types';

	let watching = $state(false), watched = $state('');
	const workflow = $derived(pipelines.workflow);
	const errors = $derived(Object.values(pipelines.errors).filter(Boolean));
	const sourceLines = $derived(pipelines.source?.text.split(/\r?\n/) ?? []);
	let sourceRows = $state<{ scrollToIndex: (index: number, align: 'center') => void } | null>(null);
	$effect(() => {
		const line = pipelines.source?.line, rows = sourceRows;
		if (line && rows) untrack(() => rows.scrollToIndex(line - 1, 'center'));
	});

	const hostLabel = (h: Hosted) => `${h.repo.kind === 'gitHub' ? 'GitHub' : h.repo.kind === 'gitLab' ? 'GitLab' : 'Bitbucket'} · ${h.repo.owner}/${h.repo.name}`;

	function show(view: 'preview' | 'live') {
		void pipelines.setView(view);
		void goto(`/pipelines/${view}`);
	}

	async function watch() {
		if (!watched.trim()) return;
		await pipelines.watch(watched);
		if (!pipelines.errors.watches) { watching = false; watched = ''; }
	}
</script>

<div class="screen">
	<header class="head">
		<span class="title">Pipelines</span>
		<nav class="seg" aria-label="Pipelines views">
			<a href="/pipelines/preview" class:on={pipelines.view === 'preview'} aria-current={pipelines.view === 'preview' ? 'page' : undefined} onclick={(e) => { e.preventDefault(); show('preview'); }}>Preview</a>
			<a href="/pipelines/live" class:on={pipelines.view === 'live'} aria-current={pipelines.view === 'live' ? 'page' : undefined} onclick={(e) => { e.preventDefault(); show('live'); }}>Live</a>
		</nav>
		{#if pipelines.view === 'preview' && workflow}
			<span class="rule" aria-hidden="true"></span>
			<span class="file mono" title={workflow.file}>{workflow.file}</span>
			<span class="chip">working copy</span>
		{/if}
		<span class="grow"></span>
		{#if pipelines.hosts.length > 1}
			<select class="host" aria-label="Pipeline host" value={pipelines.remote} onchange={(e) => pipelines.choose(e.currentTarget.value)}>
				{#each pipelines.hosts as h (h.remote)}<option value={h.remote}>{hostLabel(h)}</option>{/each}
			</select>
		{:else if pipelines.host && pipelines.view === 'live'}
			<span class="chip">{hostLabel(pipelines.host)}</span>
		{/if}
		{#if pipelines.view === 'preview'}
			<Btn disabled={!workflow} onclick={() => workflow && pipelines.openSource(workflow.file)}>Open source</Btn>
			<Btn busy={pipelines.previewing} onclick={() => pipelines.preview()}>Refresh</Btn>
		{:else}
			<Btn disabled={!pipelines.host} onclick={() => { watching = !watching; }}>Watch a branch…</Btn>
			<Btn busy={pipelines.reading} onclick={() => pipelines.refresh()}>Refresh</Btn>
		{/if}
	</header>

	{#if watching}
		<form class="watch" onsubmit={(e) => { e.preventDefault(); void watch(); }}>
			<input aria-label="Branch to watch" bind:value={watched} placeholder="release/3.1" />
			<Btn primary quiet onclick={() => watch()}>Watch</Btn>
			<Btn onclick={() => { watching = false; }}>Cancel</Btn>
		</form>
	{/if}
	{#each errors as error, index (index)}
		<p role="alert" class="error">{error}</p>
	{/each}

	{#if pipelines.view === 'preview'}
		<PreviewView />
	{:else}
		<LiveView />
	{/if}

	{#if pipelines.source}
		<section class="source" aria-label="Workflow source">
			<header>
				<strong class="mono">{pipelines.source.file} · working copy</strong>
				<span class="note">line {pipelines.source.line}</span>
				<span class="grow"></span>
				<Btn onclick={() => pipelines.closeSource()}>Close source</Btn>
			</header>
			<div class="source-rows">
				<VirtualRows bind:this={sourceRows} items={sourceLines.map((text, index) => ({ text, line: index + 1 }))} key={(line) => String(line.line)} estimate={() => 22} label="Source lines">
					{#snippet row(line)}<pre class:highlighted={line.line === pipelines.source?.line}><span class="n">{line.line}</span>{line.text}</pre>{/snippet}
				</VirtualRows>
			</div>
		</section>
	{/if}
</div>

<style>
	.screen {
		position: relative;
		flex: 1;
		min-width: 0;
		height: 100%;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		flex: none;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
		padding: 12px 16px 10px;
	}

	.title {
		margin-right: 4px;
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	.seg {
		display: inline-flex;
		gap: 2px;
		padding: 3px;
		border-radius: var(--r-pill);
		background: var(--sunken);
		border: 1px solid var(--pane-edge);
	}

	.seg a {
		padding: 4px 14px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		color: var(--muted);
		text-decoration: none;
	}

	.seg a.on {
		background: var(--surface-2);
		color: var(--ink);
		font-weight: 600;
		box-shadow: var(--shadow-1);
	}

	.rule {
		flex: none;
		width: 1px;
		height: 22px;
		background: var(--soft);
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.file {
		min-width: 0;
		max-width: 40%;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--fs-secondary);
	}

	.chip {
		display: inline-flex;
		align-items: center;
		padding: 3px 10px;
		border-radius: var(--r-pill);
		border: 1px solid var(--soft);
		background-color: var(--surface-veil);
		font-size: var(--fs-mono);
		color: var(--muted);
		white-space: nowrap;
	}

	.grow {
		flex: 1;
	}

	.host {
		height: 30px;
		max-width: 320px;
		font-size: var(--fs-secondary);
	}

	.watch {
		flex: none;
		display: flex;
		gap: 8px;
		padding: 0 16px 10px;
	}

	.watch input {
		width: 240px;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.error {
		flex: none;
		margin: 0 12px 8px;
		padding: 8px 12px;
		border-radius: var(--r-field);
		background: color-mix(in srgb, var(--danger) 9%, transparent);
		border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent);
		color: var(--danger);
		font-size: var(--fs-secondary);
		overflow-wrap: anywhere;
	}

	.source {
		position: absolute;
		inset: 56px 12px 12px;
		z-index: 2;
		display: flex;
		flex-direction: column;
		padding: 14px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
		box-shadow: var(--shadow-2);
	}

	.source header {
		display: flex;
		gap: 10px;
		align-items: center;
	}

	.source-rows {
		flex: 1;
		min-height: 0;
		margin-top: 12px;
	}

	.source-rows pre {
		margin: 0;
		padding: 0 8px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.source-rows pre.highlighted {
		background: color-mix(in srgb, var(--warn) 16%, transparent);
	}

	.n {
		display: inline-block;
		min-width: 4ch;
		margin-right: 12px;
		text-align: right;
		color: var(--muted);
	}
</style>
