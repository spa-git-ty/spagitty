<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { actionsFor, unavailable } from './contributions';
	import { extensions } from './store.svelte';
	import type { ContextKind } from './types';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * The actions extensions contribute to a screen: one button each, greyed
	 * with its reason when it cannot run. Nothing is drawn when there are none.
	 */
	interface Props {
		context: ContextKind;
	}

	let { context }: Props = $props();
	const actions = $derived(actionsFor(extensions.all, context));
</script>

{#if actions.length}
	<div class="actions" role="group" aria-label="Extension actions">
		{#each actions as action (action.key)}
			{@const reason = unavailable(action.extension, action.command, extensions.facts)}
			<Btn
				disabled={reason !== null}
				title={reason ?? `${action.extension.name}`}
				onclick={() => extensions.run(action.extension.id, action.command)}
			>
				{action.command.title}
			</Btn>
		{/each}
	</div>
{/if}

<style>
	.actions {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}
</style>
