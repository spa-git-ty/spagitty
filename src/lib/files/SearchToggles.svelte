<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { SearchOptions } from '$lib/types';
	import { files } from './store.svelte';

	/**
	 * Regex, whole word and match case, for both of Files' searches
	 * (FEAT-151): one setting, shown beside each field. Alt with R, W or C
	 * flips one from the field, as editors do.
	 */
	const TOGGLES: { name: keyof SearchOptions; label: string; title: string }[] = [
		{ name: 'regex', label: '.*', title: 'Regular expression (Alt+R)' },
		{ name: 'word', label: 'W', title: 'Whole word (Alt+W)' },
		{ name: 'case', label: 'Aa', title: 'Match case (Alt+C). Off, case counts only when you type a capital' }
	];
</script>

<span class="toggles" role="group" aria-label="Search options">
	{#each TOGGLES as toggle (toggle.name)}
		<button
			type="button"
			class="toggle mono"
			class:on={files.searchOptions[toggle.name]}
			aria-pressed={files.searchOptions[toggle.name]}
			title={toggle.title}
			onmousedown={(event) => event.preventDefault()}
			onclick={() => files.toggleOption(toggle.name)}>{toggle.label}</button
		>
	{/each}
</span>

<style>
	.toggles {
		display: inline-flex;
		gap: 2px;
		flex: none;
	}

	.toggle {
		min-width: 22px;
		height: 20px;
		padding: 0 4px;
		border: 1px solid transparent;
		border-radius: 5px;
		background: transparent;
		color: var(--muted);
		font-size: var(--fs-mono);
		line-height: 1;
	}

	.toggle:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.toggle.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--accent);
	}
</style>
