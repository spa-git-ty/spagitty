<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import { envColour } from './model';
	import type { Collection, Environment, Shared, Variable } from './types';

	/**
	 * The variables a call can use, on the right of Calls (FEAT-160,
	 * proposal §5.1): the active environment's, then those every environment
	 * shares, then the open call's collection's own. Secrets are dots and
	 * never drawn in the clear; kept values are `--ok`.
	 */

	interface Props {
		environment: Environment | null;
		shared: Shared;
		collection: Collection | null;
	}

	let { environment, shared, collection }: Props = $props();

	const groups = $derived(
		[
			{ title: 'Every environment', vars: shared.vars },
			{ title: collection ? `${collection.name} only` : '', vars: collection?.vars ?? [] }
		].filter((group) => group.vars.length > 0)
	);
</script>

{#snippet list(vars: Variable[])}
	<dl>
		{#each vars as variable (variable.name)}
			<div class="var">
				<dt class="mono" class:kept={variable.kept}>
					{variable.name}
					{#if variable.secret}<Icon name="lock" size={12} title="Secret: kept in this computer’s keychain" />{/if}
				</dt>
				<dd class="mono">
					{#if variable.secret}
						<span class="dots" aria-label="hidden">••••••••••</span>
					{:else if variable.kept}
						<span class="from">{variable.frozen ? 'kept, frozen' : 'kept from a response'}</span>
					{:else}
						{variable.value ?? ''}
					{/if}
				</dd>
			</div>
		{/each}
	</dl>
{/snippet}

<aside class="vars" aria-label="Variables">
	<header>
		{#if environment}
			<span class="dot" style:background={envColour(environment.color)} aria-hidden="true"></span>
			<span class="name">{environment.name}</span>
			<span class="count">{environment.vars.length} {environment.vars.length === 1 ? 'variable' : 'variables'}</span>
		{:else}
			<span class="name muted">No environment</span>
		{/if}
	</header>

	<div class="scroll">
		{#if environment}
			{#if environment.vars.length}
				{@render list(environment.vars)}
			{:else}
				<p class="hint">This environment has no variables yet.</p>
			{/if}
		{/if}
		{#each groups as group (group.title)}
			<h3>{group.title}</h3>
			{@render list(group.vars)}
		{/each}
	</div>

	<footer>
		<p>Type <kbd class="mono">{'{{'}</kbd> anywhere to insert one.</p>
		<p>Secrets stay on this computer and never go into the repository.</p>
	</footer>
</aside>

<style>
	.vars {
		flex: none;
		width: 250px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	header {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.name {
		font-weight: 600;
	}

	.muted,
	.count,
	.hint,
	h3,
	footer {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.scroll {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}

	h3 {
		margin: 14px 0 6px;
		font-weight: normal;
	}

	dl {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	dt {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: var(--fs-mono);
		font-weight: 600;
	}

	dt.kept {
		color: var(--ok);
	}

	dd {
		margin: 2px 0 0;
		color: var(--muted);
		font-size: var(--fs-mono);
		overflow-wrap: anywhere;
	}

	.from {
		color: var(--ok);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
	}

	.hint {
		margin: 0;
	}

	footer {
		padding-top: 10px;
		border-top: 1px solid var(--line);
	}

	footer p {
		margin: 0 0 6px;
	}

	kbd {
		padding: 0 4px;
		border: 1px solid var(--soft);
		border-radius: 4px;
	}
</style>
