<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Btn from '$lib/ui/Btn.svelte';
	import { builder } from './builder.svelte';
	import { hooks, MANAGER_LABEL } from './store.svelte';
	import { steps } from './builder-types';

	/** Only the selected record and log are read. A manager's existing commit
	 * output remains visible while its own hooks retain control. */
	const run = $derived(builder.current);
	const legacy = $derived(hooks.run && (hooks.run.status === 'running' || !run) ? hooks.run : null);
	const rows = $derived(run ? steps(run) : []);
	const selected = $derived(rows.find((row) => row.id === builder.selectedStep));
	const restaged = $derived([...new Set(rows.flatMap((row) => row.restaged))]);
	let now = $state(Date.now());
	let output = $state<HTMLPreElement | null>(null);
	let following = $state(true);
	const duration = (ms: number) => `${Math.floor(ms / 60000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, '0')}`;
	const elapsed = (at: string) => Math.max(0, now - Date.parse(at));
	onMount(() => { const timer = setInterval(() => { now = Date.now(); }, 1000); return () => clearInterval(timer); });
	$effect(() => { void builder.log; void legacy?.lines.length; if (output && following) output.scrollTop = output.scrollHeight; });
	function scrolled() { if (output) following = output.scrollTop + output.clientHeight >= output.scrollHeight - 8; }
	function stateMark(state: string) { return state === 'passed' ? '✓' : state === 'running' ? '●' : state === 'failed' ? '×' : state === 'warning' ? '!' : state === 'skipped' ? '–' : '○'; }
</script>

<div class="runs">
	<aside class="left" aria-label="Hook run steps">
		{#if legacy}
			{#each legacy.names as name}<div class="step legacy"><span class="mark running">{legacy.status === 'running' ? '●' : legacy.status === 'passed' ? '✓' : '×'}</span><span>{name}</span></div>{/each}
		{:else if run}
			<div class="run-label"><span class="mono">{run.stage}</span><span class:failed={run.exit !== null && run.exit !== 0}>{run.exit === null ? 'Running' : run.exit === 0 ? 'Passed' : 'Blocked'}</span></div>
			{#each rows as row (row.id)}
				<button class="step" class:chosen={row.id === builder.selectedStep} class:skipped={row.state === 'skipped'} onclick={() => builder.openStep(row.id)}>
					<span class="mark {row.state}">{stateMark(row.state)}</span>
					<span class="step-copy"><strong>{row.label}</strong>{#if row.why}<span class="note">{row.why === 'no-match' ? 'no change matched' : row.why === 'only-markdown' ? 'only Markdown changed' : row.why}</span>{:else if row.matched}<span class="note">{row.matched} staged files match</span>{/if}</span>
					<span class="time mono">{row.state === 'running' ? duration(elapsed(row.startedAt)) : row.state === 'queued' || row.state === 'skipped' ? '' : duration(row.ms)}</span>
				</button>
			{/each}
		{:else}
			<p class="note">No hook runs yet. Make a commit in Spagitty or a terminal, or use Dry run on staged.</p>
		{/if}
		<div class="earlier"><span class="label">Earlier</span>{#each builder.data?.runs ?? [] as earlier (earlier.id)}<button class:active={run?.id === earlier.id} onclick={() => { following = true; void builder.openRun(earlier.id); }}><span class="mark" class:passed={earlier.exit === 0} class:failed={earlier.exit !== null && earlier.exit !== 0}>{earlier.exit === null ? '●' : earlier.exit === 0 ? '✓' : '×'}</span><span><span class="mono date">{earlier.at.replace('T', ' ').replace('Z', '')}</span><span class="earlier-subject">{earlier.subject || earlier.stage}</span></span></button>{:else}<p class="note">The newest 50 runs appear here.</p>{/each}</div>
	</aside>
	<section class="detail" aria-label="Hook output">
		<header><span class="block-title">{legacy ? `${MANAGER_LABEL[builder.data?.info.manager ?? 'git']} · ${legacy.names.join(', ')}` : selected?.label ?? 'Hook output'}</span>{#if selected?.matched}<span class="badge">{selected.matched} staged files</span>{/if}<span class="grow"></span>{#if run && run.exit === null && !legacy}<Btn onclick={() => builder.stop()}>Stop</Btn>{/if}</header>
		{#if run?.subject && !legacy}<p class="subject">“{run.subject}”</p>{/if}
		<pre class="log mono" bind:this={output} onscroll={scrolled} aria-label="Selected hook log">{legacy ? legacy.lines.join('\n') || 'Waiting for output…' : builder.log || (selected?.state === 'skipped' ? 'No command ran for this block.' : selected?.state === 'queued' ? 'Waiting for the preceding blocks.' : 'Select a step to read its log.')}</pre>
		{#if legacy?.error}<p class="error" role="alert">{legacy.error}</p>{/if}
		<div class="summaries">
			{#if restaged.length}<div class="restaged"><strong>Format rewrote {restaged.length} files and restaged them</strong><div class="paths mono">{restaged.join(' · ')}</div><div class="actions"><Btn onclick={() => goto('/changes')}>See the change</Btn><Btn disabled={run?.exit === null} onclick={() => builder.undoRestage()}>Undo restage</Btn></div></div>{/if}
			<div class="staged"><strong>Staged · {builder.data?.staged.length ?? 0} files</strong><div class="paths mono">{builder.data?.staged.join('\n') || 'Nothing staged.'}</div>{#if selected?.state === 'skipped'}<p class="note">{selected.label} skipped — {selected.why === 'no-match' ? 'no change matched' : selected.why}.</p>{/if}</div>
		</div>
	</section>
</div>

<style>
	.runs { display: flex; flex: 1; min-height: 0; gap: 10px; }
	.left, .detail { background: var(--surface); border: 1px solid var(--pane-edge); border-radius: var(--r-panel); }
	.left { width: 260px; min-width: 200px; flex: none; padding: 14px; display: flex; flex-direction: column; gap: 7px; overflow: auto; }
	.run-label { display: flex; justify-content: space-between; gap: 10px; font-size: var(--fs-secondary); padding: 3px 2px 10px; }
	.step { display: flex; align-items: center; gap: 10px; padding: 10px; min-width: 0; width: 100%; text-align: left; border: 1px solid var(--line); border-radius: var(--r-button); background: var(--surface-2); }
	/* A manager's hooks run as one: there is no step to pick, so they are not drawn as cards (BUG-099). */
	.step.legacy { border-color: transparent; background: none; padding: 6px 10px; }
	.step.chosen { outline: 2px solid var(--lane-5); outline-offset: 2px; }
	.step.skipped { opacity: 0.55; border-style: dashed; }
	.step-copy { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 5px; overflow-wrap: anywhere; }
	.mark { color: var(--muted); flex: none; }
	.mark.passed { color: var(--ok); }
	.mark.running { color: var(--lane-5); }
	.failed { color: var(--danger); }
	.mark.warning { color: var(--warn); }
	.time { font-size: var(--fs-mono); color: var(--muted); }
	.earlier { margin-top: auto; padding: 12px; border-radius: var(--r-button); background: var(--sunken); }
	.label { color: var(--muted); font-size: var(--fs-secondary); text-transform: uppercase; letter-spacing: 0.06em; }
	.earlier button { display: flex; align-items: center; gap: 8px; width: 100%; padding: 8px 0; text-align: left; font-size: var(--fs-secondary); }
	.earlier button.active { color: var(--accent); }
	.earlier-subject { display: block; overflow-wrap: anywhere; }
	.date { display: block; color: var(--muted); font-size: var(--fs-mono); }
	.detail { flex: 1; min-width: 0; min-height: 0; padding: 14px; display: flex; flex-direction: column; gap: 12px; }
	header { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; }
	.block-title { font-size: var(--fs-title); font-weight: 600; }
	.badge { padding: 3px 10px; border: 1px solid var(--line); border-radius: var(--r-pill); color: var(--muted); font-size: var(--fs-secondary); }
	.grow { flex: 1; }
	.subject { color: var(--muted); margin: 0; font-size: var(--fs-secondary); }
	.log { flex: 1; min-height: 120px; overflow: auto; margin: 0; padding: 16px; border-radius: var(--r-button); background: var(--sunken); font-size: var(--fs-code); line-height: 1.55; white-space: pre-wrap; overflow-wrap: anywhere; }
	.summaries { display: flex; flex-wrap: wrap; gap: 10px; }
	.staged, .restaged { flex: 1; min-width: 200px; padding: 14px; background: var(--sunken); border-radius: var(--r-button); }
	.restaged { background: color-mix(in srgb, var(--warn) 8%, var(--surface)); border: 1px solid color-mix(in srgb, var(--warn) 40%, var(--line)); }
	.paths { color: var(--muted); white-space: pre-wrap; overflow-wrap: anywhere; max-height: 90px; overflow: auto; font-size: var(--fs-mono); margin-top: 10px; }
	.actions { display: flex; gap: 8px; margin-top: 10px; }
	.error { color: var(--danger); }
	.note { font-size: var(--fs-secondary); line-height: 1.5; }
	@media (max-width: 850px) { .left { width: 210px; min-width: 0; padding: 10px; } .time { display: none; } }
</style>
