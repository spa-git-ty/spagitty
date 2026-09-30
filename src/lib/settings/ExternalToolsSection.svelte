<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { notice } from '$lib/ui/notice.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import type { ExternalToolsConfig } from '$lib/types';

	let config = $state<ExternalToolsConfig | null>(null);
	let loading = $state(false);
	let isGlobal = $state(false);
	let isSaving = $state(false);

	async function loadConfig(): Promise<void> {
		loading = true;
		try {
			config = await api.externalToolsConfig();
		} catch (err) {
			notice.failed('Could not load external tools configuration', err);
		} finally {
			loading = false;
		}
	}

	async function updateDiffTool(toolId: string): Promise<void> {
		isSaving = true;
		try {
			await api.setExternalTool('diff', toolId || null, isGlobal);
			notice.ok('Diff tool updated', toolId || 'none');
			await loadConfig();
		} catch (err) {
			notice.failed('Could not update diff tool', err);
		} finally {
			isSaving = false;
		}
	}

	async function updateMergeTool(toolId: string): Promise<void> {
		isSaving = true;
		try {
			await api.setExternalTool('merge', toolId || null, isGlobal);
			notice.ok('Merge tool updated', toolId || 'none');
			await loadConfig();
		} catch (err) {
			notice.failed('Could not update merge tool', err);
		} finally {
			isSaving = false;
		}
	}

	onMount(() => {
		void loadConfig();
	});
</script>

<section class="section" aria-label="External diff and merge tools">
	<div class="row">
		<h2 class="heading">External Tools</h2>
		<label class="scope-toggle">
			<input type="checkbox" bind:checked={isGlobal} />
			<span>Save to <span class="mono">~/.gitconfig</span></span>
		</label>
	</div>

	{#if loading && !config}
		<div class="note">Reading…</div>
	{:else if config}
		<div class="tool-settings">
			<!-- Diff Tool -->
			<div class="field-card">
				<div class="card-head">
					<span class="label">Diff tool</span>
				</div>
				<div class="control-row">
					<select
						class="field-select"
						disabled={isSaving}
						value={config.diffTool ?? ''}
						onchange={(e) => void updateDiffTool(e.currentTarget.value)}
					>
						<option value="">Built-in</option>
						{#each config.availableDiffTools as tool}
							<option value={tool.id}>
								{tool.name}{tool.isInstalled ? '' : ' (not installed)'}
							</option>
						{/each}
					</select>
				</div>
			</div>

			<!-- Merge Tool -->
			<div class="field-card">
				<div class="card-head">
					<span class="label">Merge tool</span>
				</div>
				<div class="control-row">
					<select
						class="field-select"
						disabled={isSaving}
						value={config.mergeTool ?? ''}
						onchange={(e) => void updateMergeTool(e.currentTarget.value)}
					>
						<option value="">Built-in</option>
						{#each config.availableMergeTools as tool}
							<option value={tool.id}>
								{tool.name}{tool.isInstalled ? '' : ' (not installed)'}
							</option>
						{/each}
					</select>
				</div>
			</div>
		</div>
	{:else}
		<div class="field-card">
			<span class="note">Could not load external tools configuration.</span>
			<div class="control-row" style="margin-top: 8px;">
				<Btn disabled={loading} onclick={loadConfig}>Try again</Btn>
			</div>
		</div>
	{/if}
</section>

<style>
	/*
	 * Themed from the application's own tokens (FEAT-068, fixed under FEAT-072).
	 *
	 * This section was written against `--fg`, `--dim` and `--bg-2`, none of
	 * which exist in `src/app.css`. Every one of them therefore fell through to
	 * its hard-coded fallback — `#eee`, `#888`, `#141416` — which is a dark
	 * palette nailed into the markup. On any light theme the whole section was
	 * pale text on pale cards, and on a dark theme other than the default it was
	 * simply the wrong dark.
	 *
	 * The tokens that do exist are `--ink`, `--muted`, `--surface`, `--sunken`,
	 * `--line`, `--soft`, `--accent` and `--ok`, and none of them needs a
	 * fallback: a token that is always defined does not have a second value, and
	 * writing one is how the first set went unnoticed for a whole feature.
	 *
	 * The select is not styled here at all. `app.css` gives every input in the
	 * application the same sunken well and the same accent focus ring, and a
	 * screen that restates it is a screen that will drift from it.
	 */
	.section {
		display: flex;
		flex-direction: column;
		gap: 16px;
		max-width: 680px;
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}


	.scope-toggle {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: var(--fs-secondary);
		color: var(--muted);
		cursor: pointer;
	}

	.tool-settings {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	/* A raised surface with the panel's own hairline, like every other card in
	   the application — not a hand-picked near-black. */
	.field-card {
		background-color: var(--surface-veil);
		border: 1px solid var(--soft);
		border-radius: var(--r-panel);
		padding: 14px 16px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.card-head {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 10px;
	}

	.label {
		font-size: var(--fs-ui);
	}

	/* What is configured now. The accent, because it is the answer to the
	   question the row is asking. */

	.control-row {
		display: flex;
		gap: 8px;
	}

	.field-select {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-ui);
	}




	/* Present or absent, said with the palette's own green rather than a
	   hard-coded one — the dot is the whole content of the column. */



</style>
