<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Chip from '$lib/ui/Chip.svelte';
	import { play } from '$lib/delight/sound';
	import type { Personality, SoundLevel } from '$lib/types';
	import { settings } from './store.svelte';

	/**
	 * How much personality Spagitty is allowed to show (FEAT-072).
	 *
	 * This section exists because the delight layer would be a mistake without
	 * it. Some people want the reward moment and the jokes; some people are
	 * concentrating and want a tool that stays out of the way; both are using
	 * the same application on the same afternoon. The setting is *intensity*,
	 * never existence — badges are earned at all three levels and the Badges
	 * screen works at all three, because a record of what somebody did is not
	 * decoration and taking it away would be taking away the useful half.
	 *
	 * **Professional silences the sound as well.** Choosing it writes both keys
	 * in one go rather than leaving a sound level that the personality quietly
	 * overrules — a setting that shows `full` and plays nothing is a setting
	 * that looks broken.
	 */
	const LEVELS: { id: Personality; label: string; what: string }[] = [
		{
			id: 'professional',
			label: 'Professional',
			what: 'No reward moments, no jokes, no sound. Badges are still earned.'
		},
		{
			id: 'balanced',
			label: 'Balanced',
			what: 'A short card when something is earned. The default.'
		},
		{
			id: 'fullSpagitty',
			label: 'Full Spagitty',
			what: 'Reward moments, anti-badges, and the occasional joke.'
		}
	];

	const SOUNDS: { id: SoundLevel; label: string; what: string }[] = [
		{ id: 'off', label: 'Off', what: 'Spagitty makes no sound at all.' },
		{ id: 'subtle', label: 'Subtle', what: 'The same sounds, quietly. Audible in a quiet room, not in a meeting.' },
		{ id: 'full', label: 'Full', what: 'Commit, merge, rebase, conflict, and a sound per badge rarity.' }
	];

	const personality = $derived(settings.settings.personality);
	const sound = $derived(settings.settings.sound);

	async function choosePersonality(next: Personality): Promise<void> {
		if (next === 'professional') {
			// One write, both keys. See the note in the header.
			await settings.write({ ...settings.settings, personality: next, sound: 'off' });
			return;
		}
		await settings.choose('personality', next);
	}

	async function chooseSound(next: SoundLevel): Promise<void> {
		await settings.choose('sound', next);
		// Play the thing being chosen, at the level being chosen. Picking a
		// volume you cannot hear until the next commit is picking blind.
		if (next !== 'off') play('commit', next);
	}
</script>

<section class="section">
	<header>
		<h2 class="heading">Personality</h2>
		<span class="note">Badges are earned at every level.</span>
	</header>

	<div class="group" role="group" aria-label="Personality">
		{#each LEVELS as level (level.id)}
			<div class="row">
				<Chip
					active={personality === level.id}
					onclick={() => choosePersonality(level.id)}
					title={level.what}
				>
					{level.label}
				</Chip>
				<div class="text">
					<div class="note">{level.what}</div>
				</div>
			</div>
		{/each}
	</div>

	<header>
		<h2 class="heading">Sound</h2>
		<span class="note">Selecting a level plays it.</span>
	</header>

	<div class="group" role="group" aria-label="Sound">
		{#each SOUNDS as level (level.id)}
			<div class="row">
				<Chip
					active={sound === level.id}
					disabled={personality === 'professional' && level.id !== 'off'}
					onclick={() => chooseSound(level.id)}
					title={personality === 'professional'
						? 'Professional is silent. Choose Balanced or Full Spagitty to enable sound.'
						: level.what}
				>
					{level.label}
				</Chip>
				<div class="text">
					<div class="note">{level.what}</div>
				</div>
			</div>
		{/each}
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

	.group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.row {
		display: flex;
		align-items: flex-start;
		gap: 8px;
	}

	/* The chips are a radio group, so they need one column between them —
	   otherwise the descriptions start at three different places. */
	.row :global(.chip) {
		min-width: 108px;
		justify-content: center;
	}

	.text {
		min-width: 0;
	}
</style>
