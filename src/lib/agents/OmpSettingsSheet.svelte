<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { untrack } from 'svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import FarmSheet from '$lib/farm/components/FarmSheet.svelte';
	import type { OmpOptions } from './types';

	interface Props {
		initial: OmpOptions;
		onsave: (options: OmpOptions) => Promise<boolean>;
		onclose: () => void;
	}

	let { initial, onsave, onclose }: Props = $props();
	const start = untrack(() => initial);
	let model = $state(start.model);
	let profile = $state(start.profile);
	let busy = $state(false);

	async function save() {
		if (busy) return;
		busy = true;
		try {
			if (await onsave({ model: model.trim(), profile: profile.trim() })) onclose();
		} finally {
			busy = false;
		}
	}
</script>

<FarmSheet title="Oh My Pi settings" {onclose}>
	<label for="omp-model">Model</label>
	<input id="omp-model" class="field mono" bind:value={model} disabled={busy} placeholder="provider/model" spellcheck="false" />
	<p class="note">The model you choose with OMP’s --model option. Leave empty to use OMP’s default.</p>
	<label for="omp-profile">Profile</label>
	<input id="omp-profile" class="field mono" bind:value={profile} disabled={busy} spellcheck="false" />
	<p class="note">Optional OMP profile for your login and configuration. These choices apply to Test, Review and Merger.</p>
	<div class="actions">
		<Btn disabled={busy} onclick={onclose}>Cancel</Btn>
		<Btn primary quiet {busy} onclick={save}>Save</Btn>
	</div>
</FarmSheet>

<style>
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
