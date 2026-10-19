<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import AddAgentSheet from '../AddAgentSheet.svelte';
	import CrewTiles from '../CrewTiles.svelte';
	import RulesEditor from '../RulesEditor.svelte';
	import { FARM_COPY as C } from '../../describe';
	import * as api from '../../api';
	import type { FarmSettings } from '../../types';

	/**
	 * Step 3 (FEAT-141): the crew and the rules, as last time unless changed.
	 * The rules are the same editor the Rules sheet uses, so they read the
	 * same wherever they are changed.
	 */
	interface Props {
		rules: FarmSettings;
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		/** Offer *A slice per task* (FEAT-143). */
		workspaces: boolean;
	}

	let { rules = $bindable(), busy, act, workspaces }: Props = $props();
	let adding = $state(false);
</script>

<div class="part">
	<div class="part-head">
		<span class="small">Crew</span>
		<div class="tools">
			<Btn disabled={busy} onclick={() => act('Could not look for agents', api.detectAgents)}>
				<Icon name="refresh" />
				{C.lookAgain}
			</Btn>
			<Btn onclick={() => (adding = true)}>{C.addCli}</Btn>
			<Btn title="Every agent on this machine, and what each may do" onclick={() => goto('/settings#agents')}>Settings › Agents</Btn>
		</div>
	</div>
	<CrewTiles {busy} {act} />
</div>

<RulesEditor bind:settings={rules} {busy} {act} {workspaces} />

{#if adding}
	<AddAgentSheet {busy} {act} onclose={() => (adding = false)} />
{/if}

<style>
	.part {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.part-head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.tools {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.small {
		color: var(--muted);
		font-size: var(--fs-secondary);
	}
</style>
