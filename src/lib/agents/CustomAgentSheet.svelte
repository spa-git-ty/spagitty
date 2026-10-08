<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import FarmSheet from '$lib/farm/components/FarmSheet.svelte';
	import type { AgentDefinition } from '$lib/farm/types';

	/**
	 * Any command-line agent, added by hand (2.0): a name, the command and its
	 * arguments, as the farm's Custom adapter takes them. `{prompt}` in the
	 * arguments is where the prompt goes; without it the prompt goes on
	 * standard input.
	 */
	interface Props {
		onsave: (definition: AgentDefinition) => Promise<boolean>;
		onclose: () => void;
	}

	let { onsave, onclose }: Props = $props();

	let name = $state('');
	let command = $state('');
	let args = $state('{prompt}');
	let stdin = $state(false);
	let busy = $state(false);

	const ready = $derived(!!name.trim() && !!command.trim());

	/** Arguments split as a shell would on spaces, `{prompt}` added or taken
	 * out to match where the prompt goes. */
	function argumentsOf(): string[] {
		const split = args.trim().split(/\s+/).filter(Boolean);
		const kept = split.filter((arg) => !stdin || !arg.includes('{prompt}'));
		if (!stdin && !kept.some((arg) => arg.includes('{prompt}'))) kept.push('{prompt}');
		return kept;
	}

	async function save() {
		if (!ready || busy) return;
		busy = true;
		const slug = name.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'agent';
		const definition: AgentDefinition = {
			id: `custom-${slug}`,
			provider: 'custom',
			displayName: name.trim(),
			executable: command.trim(),
			capabilities: ['review', 'coding'],
			inputMode: stdin ? 'stdin' : 'cliPrompt',
			role: 'general',
			extraArgs: argumentsOf(),
			enabled: true,
			traits: {
				headless: true,
				streaming: true,
				resumableSessions: false,
				structuredOutput: false,
				toolUse: false
			}
		};
		const saved = await onsave(definition);
		busy = false;
		if (saved) onclose();
	}
</script>

<FarmSheet title="A command-line agent" {onclose}>
	<div class="row">
		<label class="label" for="custom-name">Name</label>
		<input id="custom-name" class="field" bind:value={name} placeholder="review-bot" spellcheck="false" />
	</div>
	<div class="row">
		<label class="label" for="custom-command">Command</label>
		<input
			id="custom-command"
			class="field mono"
			bind:value={command}
			placeholder="~/tools/review-bot"
			spellcheck="false"
		/>
	</div>
	<div class="row">
		<label class="label" for="custom-args">Arguments</label>
		<input id="custom-args" class="field mono" bind:value={args} spellcheck="false" />
	</div>
	<div class="row">
		<span class="label">Prompt</span>
		<div class="group" role="group" aria-label="Where the prompt goes">
			<Chip active={!stdin} onclick={() => (stdin = false)}>as an argument</Chip>
			<Chip active={stdin} onclick={() => (stdin = true)}>on standard input</Chip>
		</div>
	</div>
	<div class="actions">
		<Btn onclick={onclose}>Cancel</Btn>
		<Btn primary quiet disabled={!ready} {busy} onclick={save}>Add</Btn>
	</div>
</FarmSheet>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.label {
		width: 96px;
		flex: none;
		color: var(--muted);
	}

	.field {
		flex: 1;
		min-width: 0;
		font-size: var(--fs-ui);
	}

	.group {
		display: flex;
		gap: 6px;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
