<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { search } from '$lib/search/store.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import Icon from '$lib/ui/Icon.svelte';

	/**
	 * The query fields and the chips they add up to.
	 *
	 * The chips are derived from the fields rather than stored beside them, so a
	 * chip and its field cannot disagree about what is applied — which is the
	 * failure that makes a filter bar untrustworthy.
	 */
	interface Props {
		/** Focused on mount when the screen was reached by its shortcut. */
		autofocus?: boolean;
	}

	let { autofocus = false }: Props = $props();

	let first = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (autofocus) first?.focus();
	});

	function submit(event: SubmitEvent) {
		event.preventDefault();
		search.run();
	}
</script>

<form class="bar" onsubmit={submit}>
	<div class="fields">
		<label class="field">
			<span class="note">Author</span>
			<input bind:this={first} bind:value={search.author} placeholder="name or email" />
		</label>
		<label class="field">
			<span class="note">Message</span>
			<input bind:value={search.message} placeholder="text in the message" />
		</label>
		<label class="field">
			<span class="note">Path</span>
			<input class="mono" bind:value={search.path} placeholder="a file the commit changed" />
		</label>
		<label class="field">
			<span class="note">In the diff</span>
			<input class="mono" bind:value={search.diffContent} placeholder="added or removed text" />
		</label>
		<label class="field date">
			<span class="note">Since</span>
			<input class="mono" bind:value={search.since} placeholder="YYYY-MM-DD" />
		</label>
		<label class="field date">
			<span class="note">Until</span>
			<input class="mono" bind:value={search.until} placeholder="YYYY-MM-DD" />
		</label>
		<span class="go"><Btn primary disabled={search.empty}><Icon name="search" size="1em" />Search</Btn></span>
	</div>

	{#if search.chips.length > 0}
		<div class="chips">
			<span class="note small">Applied</span>
			{#each search.chips as chip (chip.key)}
				<Chip
					active
					title={`Remove ${chip.label}`}
					onclick={() => search.removeChip(chip.key)}
				>
					{chip.label} ×
				</Chip>
			{/each}
		</div>
	{/if}
</form>

<style>
	.bar {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.fields {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr)) repeat(2, minmax(110px, 0.6fr)) auto;
		align-items: end;
		gap: 10px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
	}

	.field .note {
		font-size: var(--fs-mono);
		padding-left: 2px;
	}

	.field input {
		width: 100%;
		min-width: 0;
		height: 32px;
		box-sizing: border-box;
		border: 1px solid var(--pane-edge);
		border-radius: 10px;
		padding: 0 10px;
		background: var(--sunken);
		color: var(--ink);
		font-size: var(--fs-secondary);
	}

	.field input:focus {
		outline: none;
		border-color: var(--accent);
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 18%, transparent);
	}

	.go :global(button) {
		height: 32px;
	}

	.chips {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
	}

	.small {
		font-size: var(--fs-mono);
	}

	@media (max-width: 1200px) {
		.fields {
			grid-template-columns: repeat(2, minmax(0, 1fr)) repeat(2, minmax(100px, 0.6fr)) auto;
		}
	}
</style>
