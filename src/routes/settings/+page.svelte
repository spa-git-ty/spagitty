<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import AccountsSection from '$lib/settings/AccountsSection.svelte';
	import RemotesSection from '$lib/settings/RemotesSection.svelte';
	import LicenseSection from '$lib/settings/LicenseSection.svelte';
	import AppearanceSection from '$lib/settings/AppearanceSection.svelte';
	import BehaviourSection from '$lib/settings/BehaviourSection.svelte';
	import IdentitySection from '$lib/settings/IdentitySection.svelte';
	import GodModeSection from '$lib/settings/GodModeSection.svelte';
	import PersonalitySection from '$lib/settings/PersonalitySection.svelte';
	import SigningSection from '$lib/settings/SigningSection.svelte';
	import UpdateSection from '$lib/settings/UpdateSection.svelte';
	import ExternalToolsSection from '$lib/settings/ExternalToolsSection.svelte';
	import ProfilesSection from '$lib/settings/ProfilesSection.svelte';
	import ExtensionsSection from '$lib/extensions/ExtensionsSection.svelte';
	import { SECTIONS, settings } from '$lib/settings/store.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { delight } from '$lib/delight/store.svelte';

	/**
	 * Identity, accounts, behaviour, appearance, and what this build is made of.
	 *
	 * A chip index rather than five screens: these are read rarely and changed
	 * rarely, and one route that says which part of itself is showing is easier
	 * to link to than five that each need a rail entry.
	 *
	 * The section is in the URL fragment, so `/settings#accounts` — which the
	 * Pull requests screen links to — lands on the right one, and so does going
	 * back to it. `hashchange` is listened for as well as read on mount: a
	 * fragment-only navigation from another screen does not remount this one.
	 *
	 * **Every chip has a branch, and there is no catch-all.** `accounts` was a
	 * chip with no branch: it fell through to a closing `{:else}` that drew the
	 * License section, while the accounts themselves were rendered under You the
	 * whole time. Nothing failed and nothing warned, because a catch-all cannot
	 * tell a section it does not know from the section it was written for.
	 *
	 * So the chip is gone — connecting a host is part of who you are, beside the
	 * identity, the signing key and the profiles — and the `{:else}` is gone
	 * with it. `Section` is a closed union and `showFromHash` guards what comes
	 * in from a fragment, so every value reaching here has a branch; a future
	 * section added without one draws nothing, loudly, instead of drawing
	 * somebody else's screen. `sections.test.ts` reads this file and fails
	 * first.
	 *
	 * Nothing here needs an open repository.
	 */
	/**
	 * God mode drives the delight layer by hand, so it is only offered while the
	 * layer is on (TASK-045). `#godmode` still lands on it.
	 */
	const chips = $derived(SECTIONS.filter((section) => section.id !== 'godmode' || delight.on));

	function follow() {
		settings.showFromHash(location.hash);
	}

	function choose(id: string) {
		if (typeof history !== 'undefined') {
			history.replaceState(history.state, '', `#${id}`);
		}
		settings.showFromHash(`#${id}`);
	}

	onMount(() => {
		follow();
		settings.load();

		window.addEventListener('hashchange', follow);
		return () => window.removeEventListener('hashchange', follow);
	});
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Settings</span>
			{#each chips as section (section.id)}
				<Chip active={settings.section === section.id} onclick={() => choose(section.id)}>
					{section.label}
				</Chip>
			{/each}
		</div>
		{#if settings.busy}<span class="note">Working…</span>{/if}
	</header>

	<div class="body">
		{#if settings.section === 'you'}
			<IdentitySection />
			<SigningSection />
			<ProfilesSection />
			<AccountsSection />
		{:else if settings.section === 'remotes'}
			<RemotesSection />
		{:else if settings.section === 'tools'}
			<ExternalToolsSection />
		{:else if settings.section === 'extensions'}
			<ExtensionsSection />
		{:else if settings.section === 'behaviour'}
			<BehaviourSection />
			<UpdateSection />
		{:else if settings.section === 'personality'}
			<PersonalitySection />
		{:else if settings.section === 'godmode'}
			<GodModeSection />
		{:else if settings.section === 'appearance'}
			<AppearanceSection />
		{:else if settings.section === 'license'}
			<LicenseSection />
		{/if}
	</div>

	<!--
		Only a failure gets a footer here. Where the settings are stored is not
		something the person changing them needs told, and rendered
		unconditionally the strip would be empty on every ordinary visit.
	-->
	{#if settings.writeError || settings.error}
		<footer class="foot">
			{#if settings.writeError}
				<span class="note error">{settings.writeError}</span>
			{:else}
				<span class="note error">{settings.error}</span>
			{/if}
		</footer>
	{/if}
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.left {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		flex-wrap: wrap;
	}

	.title {
		font-size: var(--fs-title);
		white-space: nowrap;
	}

	/* A column since **You** grew a second section: two stacked sections with no
	   gap read as one section with a stray heading in the middle. */
	.body {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 12px;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.foot {
		flex: none;
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-top: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.error {
		color: var(--danger);
	}
</style>
