<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import Btn from '$lib/ui/Btn.svelte';
	import EmptyState from '$lib/ui/EmptyState.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { sortable, type SortState } from '$lib/ui/sortable';
	import BlockEditor from './BlockEditor.svelte';
	import HookRuns from './HookRuns.svelte';
	import { builder } from './builder.svelte';
	import { CATALOGUE, STAGES, type Stage } from './builder-types';
	import { MANAGER_LABEL } from './store.svelte';

	/** Hooks (1X): the two artboards share a store and the shell's own chrome. */
	let { mode }: { mode: 'runs' | 'edit' } = $props();
	const draft = $derived(builder.draft);
	const data = $derived(builder.data);
	let dryStage = $state<Stage>('pre-commit');
	let dragging = $state<string | null>(null);
	let over = $state<string | null>(null);

	/** By the grip, with the pointer: HTML drag and drop never reached the page on Windows (BUG-092). */
	function dropped(moving: string, target: string) {
		if (!draft) return;
		const siblings = draft.blocks.filter((block) => block.stage === draft.blocks.find((block) => block.id === moving)?.stage);
		const from = siblings.findIndex((block) => block.id === moving);
		const to = siblings.findIndex((block) => block.id === target);
		if (from >= 0 && to >= 0) for (let i = 0; i < Math.abs(to - from); i++) builder.move(moving, to > from ? 1 : -1);
	}
	const sorting = $derived({ disabled: builder.readonly, onmove: dropped, onstate: (state: SortState) => { dragging = state.dragging; over = state.over; } });
	async function dryRun() {
		await builder.dryRun(dryStage, () => { void goto('/hooks/runs'); });
	}
</script>

<div class="screen">
	<header class="head">
		<span class="title">Hooks</span>
		<nav class="tabs" aria-label="Hooks views"><a href="/hooks/runs" class:on={mode === 'runs'} aria-current={mode === 'runs' ? 'page' : undefined}>Runs</a><a href="/hooks/edit" class:on={mode === 'edit'} aria-current={mode === 'edit' ? 'page' : undefined}>Hooks</a></nav>
		{#if builder.dirty}<span class="unsaved">Unsaved hook changes</span>{/if}
		<span class="grow"></span>
		{#if mode === 'edit' && draft}
			<span class="storage mono" title="Personal definitions are kept in application data.">{draft.shared ? '.spagitty/hooks.toml' : 'Personal hooks'}</span>
			<label class="share"><input type="checkbox" role="switch" checked={draft.shared} disabled={builder.readonly || builder.saving} onchange={(event) => builder.share(event.currentTarget.checked)} /> Share with repo</label>
			<select aria-label="Dry run stage" value={dryStage} onchange={(event) => { dryStage = event.currentTarget.value as Stage; }}><option value="pre-commit">pre-commit</option><option value="pre-push">pre-push</option></select>
			<Btn disabled={builder.readonly || draft.blocks.length === 0 || builder.saving} busy={builder.running} onclick={dryRun}>Dry run on staged</Btn>
			<Btn primary quiet busy={builder.saving} disabled={builder.readonly || builder.running} onclick={() => builder.save()}>Save & install</Btn>
		{:else if data}
			<Btn onclick={() => builder.refreshRuns(true)}>Refresh</Btn>
		{/if}
	</header>
	{#each Object.entries(builder.errors).filter(([name, value]) => name !== 'matches' && value) as [name, error] (name)}<p class="error" role="alert">{error}</p>{/each}
	{#if builder.loading}<Loader label="Reading hooks…" />
	{:else if !data || !draft}<EmptyState message={builder.errors.load || 'Open a repository to see and build its hooks.'} />
	{:else}
		{#if data.info.manager !== 'git' || builder.imported}
			<div class="manager"><span>{builder.imported ? 'Commands imported. The original manager still owns the installed hooks until Save & install.' : `${MANAGER_LABEL[data.info.manager]} manages this repository’s hooks. Runs are available; the builder is read-only.`}</span><Btn disabled={builder.imported} onclick={() => builder.importManager()}>Import</Btn></div>
		{:else if draft.blocks.length > 0 && !data.installed}
			<div class="manager"><span>This definition is saved but is not installed for this clone.</span><Btn disabled={builder.saving} onclick={() => builder.save()}>Install hooks</Btn></div>
		{/if}
		<div class="body">
			{#if mode === 'runs'}<HookRuns />
			{:else}
				<aside class="stack" aria-label="Hook stages">
					{#each STAGES as stage (stage)}
						<section class="stage" aria-label={stage} use:sortable={sorting}>
							<div class="stage-head"><strong class="mono">{stage}</strong><span class="note">{draft.blocks.filter((block) => block.stage === stage).length}</span><span class="grow"></span><input type="checkbox" role="switch" aria-label={`Enable ${stage}`} checked={draft.activeStages.includes(stage)} disabled={builder.readonly || builder.saving} onchange={() => builder.toggleStage(stage)} /></div>
							{#each draft.blocks.filter((block) => block.stage === stage) as block, index (block.id)}
								<div class="block" class:selected={builder.block?.id === block.id} class:off={!block.enabled || !draft.activeStages.includes(stage)} class:dragging={dragging === block.id} class:over={over === block.id} data-sort-key={block.id} role="group" aria-label={block.label}>
									<button class="block-pick" onclick={() => builder.select(block.id)}><span class="grip" data-sort-grip aria-hidden="true">⠿</span><span class="square" style:--tone={CATALOGUE.find((item) => item.kind === block.action.kind)?.tone}></span><span>{block.label}</span></button>
									<div class="order"><button aria-label={`Move ${block.label} up`} title="Move up" disabled={builder.readonly || index === 0} onclick={() => builder.move(block.id, -1)}>↑</button><button aria-label={`Move ${block.label} down`} title="Move down" disabled={builder.readonly || index === draft.blocks.filter((item) => item.stage === stage).length - 1} onclick={() => builder.move(block.id, 1)}>↓</button></div>
								</div>
							{:else}<p class="note no-blocks">No blocks at this stage.</p>{/each}
						</section>
					{/each}
					<span class="grow"></span><p class="note hint">Order runs top to bottom · drag to reorder</p>
				</aside>
				<BlockEditor />
				<aside class="catalogue" aria-label="Hook block catalogue"><span class="label">Add</span>{#each CATALOGUE as item (item.kind)}<button class="catalogue-block" disabled={builder.readonly || draft.blocks.length >= 100} title={item.detail} onclick={() => builder.add(item.kind, builder.block?.stage ?? 'pre-commit')}><span class="catalogue-title"><span class="square" style:--tone={item.tone}></span><strong>{item.label}</strong></span><span class="note">{item.detail}</span></button>{/each}<span class="grow"></span>{#if data.info.config}<Btn onclick={() => builder.importManager()}>Import {data.info.config.path}</Btn>{/if}</aside>
			{/if}
		</div>
	{/if}
</div>

<style>
	.screen { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
	.head { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; padding: 12px 16px; flex: none; }
	.title { font-size: var(--fs-title); margin-right: 5px; }
	.tabs { display: flex; padding: 3px; border: 1px solid var(--line); border-radius: var(--r-pill); background: var(--sunken); }
	.tabs a { padding: 4px 14px; color: var(--muted); border-radius: var(--r-pill); text-decoration: none; }
	.tabs a.on { background: var(--hover); color: var(--ink); }
	.grow { flex: 1; }
	.unsaved { color: var(--warn); font-size: var(--fs-secondary); }
	.storage { border: 1px solid var(--line); border-radius: var(--r-pill); padding: 3px 10px; font-size: var(--fs-mono); color: var(--muted); }
	.share { display: flex; align-items: center; gap: 7px; white-space: nowrap; font-size: var(--fs-secondary); }
	input[type='checkbox'] { accent-color: var(--ok); }
	.head select { max-width: 120px; font-size: var(--fs-secondary); }
	.manager { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin: 0 12px 10px; padding: 10px 14px; border: 1px solid var(--line); border-radius: var(--r-button); font-size: var(--fs-secondary); color: var(--muted); }
	.body { display: flex; flex: 1; min-height: 0; gap: 10px; padding: 0 12px 12px; }
	.stack, .catalogue { width: 270px; flex: none; min-height: 0; overflow: auto; padding: 14px; display: flex; flex-direction: column; gap: 10px; border: 1px solid var(--pane-edge); border-radius: var(--r-panel); background: var(--surface); }
	.stage { display: flex; flex-direction: column; gap: 7px; }
	.stage + .stage { margin-top: 10px; }
	.stage-head { display: flex; align-items: center; gap: 8px; margin: 0 1px 3px; }
	.block { display: flex; align-items: center; border: 1px solid var(--line); border-radius: var(--r-button); background: var(--surface-2); }
	.block.selected { outline: 2px solid var(--accent); outline-offset: 2px; }
	.block.off { opacity: 0.5; }
	.block.dragging { opacity: 0.4; }
	.block.over { box-shadow: inset 0 2px 0 var(--accent); }
	.block-pick { display: flex; align-items: center; flex: 1; min-width: 0; gap: 8px; text-align: left; padding: 10px; font-size: var(--fs-secondary); }
	.block-pick > span:last-child { overflow-wrap: anywhere; }
	.square { width: 9px; height: 9px; border-radius: 3px; background: var(--tone); flex: none; }
	.grip { color: var(--muted); cursor: grab; touch-action: none; }
	.order { display: flex; flex-direction: column; padding-right: 5px; }
	.order button { color: var(--muted); font-size: var(--fs-secondary); padding: 0 3px; }
	.no-blocks { margin: 2px 0 8px; }
	.note { color: var(--muted); font-size: var(--fs-secondary); line-height: 1.45; }
	.hint { margin: 5px 0 0; }
	.label { color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; font-size: var(--fs-secondary); }
	.catalogue-block { display: flex; flex-direction: column; gap: 6px; padding: 12px; text-align: left; border: 1px solid var(--line); border-radius: var(--r-button); background: var(--surface-2); }
	.catalogue-block:hover { background: var(--hover); }
	.catalogue-title { display: flex; align-items: center; gap: 8px; font-size: var(--fs-secondary); }
	.error { color: var(--danger); margin: 0; padding: 0 16px 8px; }
	@media (max-width: 1100px) { .catalogue { width: 216px; padding: 10px; } .stack { width: 230px; padding: 10px; } .storage { display: none; } }
</style>
