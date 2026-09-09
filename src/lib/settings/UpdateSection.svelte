<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { settings } from './store.svelte';

	/**
	 * Whether there is a newer Spagitty, and whether to look on startup.
	 *
	 * It sits under Behaviour's heading rather than beside the licence, because
	 * the interesting half is the toggle: it is the only preference in the
	 * application that causes a network request, and somebody deciding about it
	 * should be reading the sentence that says so.
	 *
	 * The link is shown rather than opened. Spagitty does not launch browsers —
	 * there is no opener in the build and adding one to save a copy-and-paste
	 * would be a dependency and a new way for a URL from a host to be acted on.
	 */

	const update = $derived(settings.update);
	const stored = $derived(settings.settings);

	let copied = $state(false);

	async function copy(url: string) {
		try {
			await navigator.clipboard.writeText(url);
			copied = true;
			setTimeout(() => (copied = false), 1200);
		} catch {
			// No clipboard, which is the case in a plain browser. The URL is on
			// screen either way, which is the part that matters.
		}
	}
</script>

<section class="section" id="updates">
	<header>
		<h2 class="heading">Updates</h2>
	</header>

	<div class="row">
		<Chip
			active={stored.checkForUpdates}
			disabled={settings.busy}
			onclick={() => settings.toggle('checkForUpdates')}
			title="Ask the project for the latest release when Spagitty starts"
		>
			{stored.checkForUpdates ? 'on' : 'off'}
		</Chip>
		<div class="text">
			<div>Check for updates when Spagitty starts</div>
			<!--
				Trimmed to the two claims, and only the two (TASK-038). What
				this line is for is answering "what does it send" and "does off
				mean off" — everything else it used to say was an elaboration
				of the first answer.
			-->
			<div class="note" title="One request to the project's releases page. Turning it off stops every request.">
				No account, no identifier.
			</div>
		</div>
	</div>

	<div class="hr"></div>

	<div class="row">
		<Btn disabled={settings.checking} onclick={() => settings.checkForUpdate()}>
			{settings.checking ? 'Checking…' : 'Check now'}
		</Btn>
		{#if update}
			<span class="note">
				This build:
				<span class="mono">{update.current ?? 'compiled here, with no release tag'}</span>
			</span>
		{/if}
	</div>

	{#if settings.updateError}
		<p class="note error">{settings.updateError}</p>
	{:else if update === null}
		<p class="note">Not checked yet.</p>
	{:else if update.channel === 'development'}
		<!--
			A build somebody compiled has no tag to be behind, and is usually
			ahead of every release. Telling them otherwise would be wrong.
		-->
		<p class="note">Development build. Latest release: <span class="mono">{update.latest}</span>.</p>
	{:else if update.newer}
		<p class="note">
			<span class="mono">{update.latest}</span> has been released.
		</p>
		<div class="row">
			<span class="mono url">{update.url}</span>
			<Btn onclick={() => copy(update.url)}>{copied ? 'Copied' : 'Copy link'}</Btn>
		</div>
	{:else}
		<p class="note">
			Up to date — <span class="mono">{update.latest}</span> is the latest release.
		</p>
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

	.row {
		display: flex;
		align-items: flex-start;
		gap: 8px;
		flex-wrap: wrap;
	}

	.text {
		min-width: 0;
	}

	.url {
		font-size: var(--fs-secondary);
		color: var(--muted);
		overflow-wrap: anywhere;
		min-width: 0;
	}

	.error {
		color: var(--danger);
	}

	p {
		margin: 0;
	}
</style>
