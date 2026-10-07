<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import FarmSheet from './FarmSheet.svelte';
	import { FARM_COPY as C } from '../describe';
	import * as api from '../api';
	import type { AgentDefinition } from '../types';

	/**
	 * Adding a command-line agent the farm did not find on its own (FEAT-111).
	 *
	 * Shared by Setup and Crew. A custom agent is a coder until somebody says
	 * otherwise: it takes a prompt, runs headless, and exits.
	 */
	interface Props {
		busy: boolean;
		act: (message: string, run: () => Promise<unknown>) => Promise<boolean>;
		onclose: () => void;
	}

	let { busy, act, onclose }: Props = $props();

	let name = $state('');
	let executable = $state('');
	let input = $state<'cliPrompt' | 'stdin'>('cliPrompt');

	const ready = $derived(!!name.trim() && !!executable.trim());

	async function add() {
		if (!ready) return;
		const agent: AgentDefinition = {
			id: `custom-${crypto.randomUUID()}`,
			provider: 'custom',
			displayName: name.trim(),
			executable: executable.trim(),
			capabilities: ['coding'],
			inputMode: input,
			role: 'general',
			extraArgs: [],
			enabled: true,
			traits: {
				headless: true,
				streaming: true,
				resumableSessions: false,
				structuredOutput: false,
				toolUse: false
			}
		};
		if (await act('Could not add the agent', () => api.saveAgent(agent))) onclose();
	}
</script>

<FarmSheet title={C.addCli} {onclose}>
	<p class="muted">{C.cliDetail}</p>
	<label>{C.cliName}<input bind:value={name} /></label>
	<label>{C.cliExecutable}<input class="mono" bind:value={executable} /></label>
	<label>
		{C.input}
		<select bind:value={input}>
			<option value="cliPrompt">As an argument</option>
			<option value="stdin">On stdin</option>
		</select>
	</label>
	<div class="actions">
		<Btn onclick={onclose}>{C.close}</Btn>
		<Btn primary disabled={busy || !ready} onclick={add}>{C.save}</Btn>
	</div>
</FarmSheet>
