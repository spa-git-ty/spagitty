<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import type { IdentityKey } from '$lib/types';
	import { describeOrigin, describeOverride } from './describe';
	import { settings } from './store.svelte';

	/**
	 * Who commits: `user.name` and `user.email`, read from and written to git's
	 * own configuration.
	 *
	 * The scope is chosen, never inferred. Writing a repository-local identity
	 * into the global file is a mistake nobody notices until it is on somebody
	 * else's commits, so both scopes are shown, the fields say which one they
	 * are editing, and each value says which file it is coming from.
	 */
	const FIELDS: { key: IdentityKey; label: string; placeholder: string }[] = [
		{ key: 'name', label: 'Name', placeholder: 'Ada Lovelace' },
		{ key: 'email', label: 'Email', placeholder: 'ada@example.com' }
	];

	const identity = $derived(settings.identity);
</script>

<section class="section">
	<header class="row">
		<h2 class="heading">You</h2>
	</header>

	<div class="row">
		<span class="note">Editing</span>
		<Chip active={settings.scope === 'global'} onclick={() => settings.setScope('global')}>
			global
		</Chip>
		<Chip
			active={settings.scope === 'local'}
			onclick={() => settings.setScope('local')}
			title={settings.canEditLocally
				? 'This repository only'
				: 'No repository is open, so there is no repository configuration to edit'}
		>
			this repository
		</Chip>
		{#if !settings.canEditLocally}
			<!-- Why the chip beside this is disabled. The chip's own title says
			     the same thing; this says it without a hover, because a control
			     that cannot be pressed has to explain itself where it is. -->
			<span class="note">No repository is open.</span>
		{/if}
	</div>

	{#if identity === null}
		<p class="note">Reading the git configuration…</p>
	{:else}
		{#each FIELDS as field (field.key)}
			{@const value = identity[field.key]}
			<div class="field-row">
				<label class="label" for="identity-{field.key}">{field.label}</label>
				<input
					id="identity-{field.key}"
					class="field"
					type="text"
					placeholder={field.placeholder}
					value={settings.draft(field.key)}
					oninput={(event) => settings.setDraft(field.key, event.currentTarget.value)}
				/>
				<Btn
					primary
					disabled={settings.busy || !settings.isDirty(field.key)}
					onclick={() => settings.save(field.key)}
				>
					Save
				</Btn>
				<Btn
					disabled={settings.busy || settings.draft(field.key) === ''}
					title="Empty the field. Saving it empty unsets the key rather than storing a blank value."
					onclick={() => settings.clear(field.key)}
				>
					Clear
				</Btn>
			</div>
			<p class="note under">
				{#if value.effective === null}
					{describeOrigin(value.origin)}
				{:else}
					In effect: <span class="mono">{value.effective}</span> — {describeOrigin(value.origin)}
				{/if}
			</p>
			{#if describeOverride(value, settings.scope)}
				<p class="note under warn">{describeOverride(value, settings.scope)}</p>
			{/if}
		{/each}

		<!--
			The paragraph that used to close this section said two things: that
			the write goes through `git config`, and what Clear does. The first
			is visible in the "In effect" line above every field, which names
			the file the value came from. The second belongs on the button that
			does it, and is on it (TASK-038).
		-->
	{/if}
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 640px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.row,
	.field-row {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.label {
		width: 48px;
		flex: none;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.under {
		margin: -4px 0 4px 56px;
	}

	.warn {
		color: var(--ink);
	}

	.field {
		background: transparent;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		padding: 3px 6px;
		width: 260px;
	}

	.field:focus {
		outline: none;
		border-color: var(--accent);
	}

	.field::placeholder {
		color: var(--placeholder);
	}

	p {
		margin: 0;
	}
</style>
