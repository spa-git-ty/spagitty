<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { profiles } from '$lib/profiles/store.svelte';
	import { settings } from '$lib/settings/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import type { IdentityProfile } from '$lib/types';

	let isAdding = $state(false);
	let profileName = $state('');
	let authorName = $state('');
	let authorEmail = $state('');
	let signingKey = $state('');
	let isSaving = $state(false);
	let error = $state<string | null>(null);

	const list = $derived(profiles.list);
	const hasRepo = $derived(repo.info !== null);

	/** A label as a stable id: lower case, one dash per run, none at the ends. */
	function slug(label: string): string {
		return label
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '');
	}

	async function saveProfile(): Promise<void> {
		if (!profileName.trim() || !authorName.trim() || !authorEmail.trim()) {
			error = 'Name and email are required';
			return;
		}

		error = null;
		isSaving = true;

		const profile: IdentityProfile = {
			// From the trimmed label, not the raw one. Deriving it from what was
			// typed gave `  Work Laptop ` the id `--work-laptop-`: leading and
			// trailing dashes from the spaces, and a run of them wherever the
			// label had a space beside punctuation. The id is the key the
			// profile is stored under, so that is a key nobody could guess and
			// two labels differing only in spacing would collide on.
			id: slug(profileName),
			name: profileName.trim(),
			authorName: authorName.trim(),
			authorEmail: authorEmail.trim(),
			signingKey: signingKey.trim() || null
		};

		try {
			await profiles.save(profile);
			notice.ok('Profile saved', profile.name);
			isAdding = false;
			profileName = '';
			authorName = '';
			authorEmail = '';
			signingKey = '';
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			isSaving = false;
		}
	}

	async function applyProfile(profile: IdentityProfile, global = false): Promise<void> {
		try {
			await profiles.apply(profile, global);
			await settings.load();
			notice.ok(
				global ? 'Applied profile globally' : 'Applied profile to repository',
				profile.name
			);
		} catch (err) {
			notice.failed('Could not apply profile', err);
		}
	}

	async function deleteProfile(id: string): Promise<void> {
		try {
			await profiles.delete(id);
			notice.ok('Profile deleted');
		} catch (err) {
			notice.failed('Could not delete profile', err);
		}
	}

	onMount(() => {
		void profiles.fetch();
	});
</script>

<section class="section" aria-label="Identity profiles">
	<div class="row">
		<h2 class="title">Identity Profiles</h2>
		<Btn disabled={isAdding} onclick={() => (isAdding = true)}>+ Add Profile</Btn>
	</div>

	<p class="desc">
		Save author identity profiles (e.g. Work, Personal) to switch names, emails, and signing keys across repositories with one click.
	</p>

	{#if isAdding}
		<div class="form-card">
			<h3 class="form-title">New Identity Profile</h3>
			<div class="form-grid">
				<label class="field-col">
					<span class="field-label">Profile Label</span>
					<input
						type="text"
						class="field-input"
						placeholder="Work / Personal"
						bind:value={profileName}
					/>
				</label>
				<label class="field-col">
					<span class="field-label">Author Name</span>
					<input
						type="text"
						class="field-input"
						placeholder="Ada Lovelace"
						bind:value={authorName}
					/>
				</label>
				<label class="field-col">
					<span class="field-label">Author Email</span>
					<input
						type="email"
						class="field-input"
						placeholder="ada@example.com"
						bind:value={authorEmail}
					/>
				</label>
				<label class="field-col">
					<span class="field-label">GPG / SSH Signing Key (optional)</span>
					<input
						type="text"
						class="field-input mono"
						placeholder="3AA5C34371567BD2"
						bind:value={signingKey}
					/>
				</label>
			</div>

			{#if error}
				<div class="error" role="alert">{error}</div>
			{/if}

			<div class="form-actions">
				<Btn disabled={isSaving} onclick={() => (isAdding = false)}>Cancel</Btn>
				<Btn primary disabled={isSaving} onclick={saveProfile}>
					{isSaving ? 'Saving…' : 'Save Profile'}
				</Btn>
			</div>
		</div>
	{/if}

	<div class="profiles-list">
		{#if list.length === 0 && !isAdding}
			<div class="empty-note">No saved profiles. Click "+ Add Profile" to create one.</div>
		{:else}
			{#each list as p (p.id)}
				<div class="profile-card">
					<div class="card-info">
						<div class="name-row">
							<span class="profile-name">{p.name}</span>
							{#if p.signingKey}
								<span class="key-pill mono" title="Signing key configured">🔑 {p.signingKey}</span>
							{/if}
						</div>
						<div class="detail-row mono">
							<span class="author">{p.authorName}</span>
							<span class="email">&lt;{p.authorEmail}&gt;</span>
						</div>
					</div>

					<div class="card-actions">
						{#if hasRepo}
							<Btn onclick={() => void applyProfile(p, false)}>Apply to Repo</Btn>
						{/if}
						<Btn onclick={() => void applyProfile(p, true)}>Apply Globally</Btn>
						<button
							type="button"
							class="delete-btn"
							title="Delete profile"
							onclick={() => void deleteProfile(p.id)}
						>
							🗑️
						</button>
					</div>
				</div>
			{/each}
		{/if}
	</div>
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 14px;
		max-width: 680px;
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
		color: var(--ink);
	}

	.desc {
		margin: 0;
		font-size: var(--fs-secondary);
		color: var(--muted);
		line-height: 1.4;
	}

	.form-card {
		background: var(--panel);
		border: 1px solid var(--line);
		border-radius: var(--r-field, 6px);
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.form-title {
		margin: 0;
		font-size: var(--fs-secondary);
		font-weight: 600;
		color: var(--ink);
	}

	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}

	.field-col {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.field-label {
		font-size: var(--fs-mono);
		color: var(--muted);
		font-weight: 500;
	}

	.field-input {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 4px;
		padding: 6px 8px;
		font-size: var(--fs-secondary);
		color: var(--ink);
		outline: none;
	}

	.field-input:focus {
		border-color: var(--accent);
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 4px;
	}

	.profiles-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.profile-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 14px;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 6px;
		gap: 12px;
	}

	.card-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
		overflow: hidden;
		flex: 1;
	}

	.name-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.profile-name {
		font-weight: 600;
		font-size: var(--fs-secondary);
		color: var(--ink);
	}

	.key-pill {
		font-size: var(--fs-mono);
		background: var(--soft);
		color: var(--muted);
		padding: 1px 6px;
		border-radius: 4px;
	}

	.detail-row {
		font-size: var(--fs-secondary);
		color: var(--muted);
		display: flex;
		gap: 6px;
	}

	.author {
		color: var(--ink);
	}

	.card-actions {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.delete-btn {
		background: transparent;
		border: 1px solid var(--line);
		border-radius: 4px;
		padding: 4px 6px;
		cursor: pointer;
		font-size: var(--fs-secondary);
	}

	.delete-btn:hover {
		border-color: var(--danger);
	}

	.empty-note {
		text-align: center;
		padding: 24px;
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.error {
		font-size: var(--fs-secondary);
		color: var(--danger);
		background: var(--danger-soft);
		padding: 6px 8px;
		border-radius: 4px;
	}

	.mono {
		font-family: var(--font-mono);
	}
</style>
