<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { watcher } from '$lib/notifications/watcher.svelte';
	import { center } from '$lib/notifications/center.svelte';
	import { settings, type BooleanSetting } from './store.svelte';

	/**
	 * Pull request notifications (FEAT-114): whether to watch, where to say it,
	 * which kinds, and how often to look. The sound is Spagitty's own
	 * `notification` cue at the Sound level chosen under Personality.
	 */
	const KINDS: { key: BooleanSetting; label: string }[] = [
		{ key: 'notifyMerged', label: 'Merged or closed' },
		{ key: 'notifyComments', label: 'Comments and reviews' },
		{ key: 'notifyReviewRequests', label: 'Review requests' },
		{ key: 'notifyChecks', label: 'Failing checks' }
	];

	const EVERY = [1, 2, 5, 15];

	const stored = $derived(settings.settings);
	const on = $derived(stored.notifyPullRequests);
	const connected = $derived(settings.accounts.length > 0);

	let clearing = $state(false);
	let cleared = $state<number | null>(null);

	async function clearOld() {
		clearing = true;
		try {
			cleared = await center.clear(Date.now() - 90 * 24 * 60 * 60 * 1000);
		} finally {
			clearing = false;
		}
	}

	$effect(() => {
		void center.load();
	});
</script>

<section class="section">
	<header>
		<h2 class="heading">Notifications</h2>
	</header>

	<div class="row">
		<Chip
			active={on}
			disabled={settings.busy}
			onclick={() => settings.toggle('notifyPullRequests')}
			title="Looks at the connected accounts' pull requests while Spagitty is open"
		>
			{on ? 'on' : 'off'}
		</Chip>
		<div class="text">Tell me when my pull requests change</div>
	</div>

	{#if !connected}
		<p class="note">Connect an account under Accounts first.</p>
	{/if}

	{#if on}
		<div class="row">
			<Chip
				active={stored.notifyDesktop}
				disabled={settings.busy}
				onclick={() => settings.toggle('notifyDesktop')}
			>
				{stored.notifyDesktop ? 'on' : 'off'}
			</Chip>
			<div class="text">System notifications too</div>
		</div>

		<div class="hr"></div>

		<div class="group" role="group" aria-label="What to tell">
			{#each KINDS as kind (kind.key)}
				<Chip
					active={stored[kind.key]}
					disabled={settings.busy}
					onclick={() => settings.toggle(kind.key)}
				>
					{kind.label}
				</Chip>
			{/each}
		</div>

		<div class="group" role="group" aria-label="How often to look">
			<span class="note">Every</span>
			{#each EVERY as minutes (minutes)}
				<Chip
					active={stored.notifyEveryMinutes === minutes}
					disabled={settings.busy}
					onclick={() => settings.choose('notifyEveryMinutes', minutes)}
				>
					{minutes} min
				</Chip>
			{/each}
		</div>

		<div class="row">
			<Btn onclick={() => watcher.check()}>Check now</Btn>
			<Btn onclick={() => watcher.test()}>Send a test</Btn>
		</div>

		{#if watcher.lastError}
			<p class="note error">{watcher.lastError}</p>
		{:else if watcher.lastChecked}
			<p class="note">
				Last looked at {new Date(watcher.lastChecked).toLocaleTimeString([], {
					hour: '2-digit',
					minute: '2-digit'
				})}
			</p>
		{/if}
	{/if}

	<!--
		The notification center keeps everything until the person clears it
		(FEAT-142). Not behind the watcher's switch: the farm, the checks and git
		write there whether pull requests are watched or not.
	-->
	<div class="hr"></div>
	<div class="row">
		<Btn disabled={clearing} onclick={clearOld}>Clear older than 90 days</Btn>
		<span class="note">
			{cleared === null
				? `The notification center keeps ${center.entries.length} ${center.entries.length === 1 ? 'notification' : 'notifications'}.`
				: `${cleared} cleared.`}
		</span>
	</div>
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 10px;
		max-width: 640px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.row,
	.group {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
	}

	.text {
		min-width: 0;
	}

	.note {
		margin: 0;
		color: var(--muted);
	}

	.error {
		color: var(--danger);
	}

	.hr {
		height: 1px;
		background: var(--soft);
	}
</style>
