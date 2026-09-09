<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import BadgeChip from '$lib/delight/BadgeChip.svelte';
	import { BADGES, CATEGORY_LABELS } from '$lib/delight/badges';
	import { DEMOS, expand } from '$lib/delight/demo';
	import { play, status, type Cue } from '$lib/delight/sound';
	import { delight } from '$lib/delight/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';
	import { settings } from './store.svelte';

	/**
	 * God mode — the delight layer, driven by hand (FEAT-072).
	 *
	 * Every other badge in Spagitty takes real work to see. That is the point of
	 * them and it is also the problem: nobody can check that Git Lord looks right
	 * without earning Git Lord, and nobody is going to resolve ten conflicts to
	 * find out whether a sound is too loud. This section is the answer.
	 *
	 * # It says what it is doing to the record
	 *
	 * Three of the four groups below **write to the badge record of the open
	 * repository**, and they say so rather than being quietly reversible. The
	 * fourth — the preview grid — writes nothing at all, which is why it is
	 * first: the common case is wanting to look at a card, not wanting to own
	 * one.
	 *
	 * # Why the events are a list of situations
	 *
	 * "Fire a conflict with 12 files" is a form with four fields; "a twelve-file
	 * conflict, resolved" is a sentence. The list is also a readable statement of
	 * what each rule is *for*, which is worth having next to the rules
	 * themselves.
	 */

	const bound = $derived(delight.repository !== null);
	const actor = $derived(delight.me);

	/** The badges, grouped the way the Badges screen groups them. */
	const groups = $derived(
		CATEGORY_LABELS.map((entry) => ({
			...entry,
			badges: BADGES.filter((found) => found.category === entry.id)
		}))
	);

	const held = $derived(new Set(actor.earned.map((entry) => entry.id)));

	const CUES: { id: Cue; label: string }[] = [
		{ id: 'commit', label: 'Commit' },
		{ id: 'merge', label: 'Merge' },
		{ id: 'rebase', label: 'Rebase' },
		{ id: 'conflict', label: 'Conflict' },
		{ id: 'recovery', label: 'Recovery' },
		{ id: 'common', label: 'Common' },
		{ id: 'uncommon', label: 'Uncommon' },
		{ id: 'rare', label: 'Rare' },
		{ id: 'epic', label: 'Epic' },
		{ id: 'legendary', label: 'Legendary' }
	];

	/**
	 * What the audio device says about itself, re-read on every click.
	 *
	 * Read lazily rather than held: the context is not built until something
	 * first asks for a sound, so reading it at mount would report "unavailable"
	 * on a machine where audio works perfectly.
	 */
	let audio = $state<{ supported: boolean; state: string; note: string } | null>(null);

	function sound(cue: Cue): void {
		play(cue, settings.settings.sound);
		audio = status();
	}

	function fire(id: string): void {
		const demo = DEMOS.find((entry) => entry.id === id);
		if (!demo) return;

		const events = expand(demo, Date.now());
		for (const event of events) delight.record(event);

		// The queue is what actually says something happened, and a demo that
		// earned nothing new should say that rather than looking broken.
		if (delight.waiting === 0 && delight.showing === null) {
			notice.ok(demo.label, 'Counted. Nothing new was earned by it.');
		}
	}

	function toggle(id: string): void {
		if (held.has(id)) delight.revoke(id);
		else delight.grant(id);
	}

	async function clear(): Promise<void> {
		const agreed = await dialog.confirm({
			title: 'Clear the record here',
			body: 'Every badge, title and count in this repository is deleted. Nothing in git is touched.',
			confirmLabel: 'Clear',
			danger: true
		});
		if (!agreed) return;
		delight.forget();
		notice.ok('The record here was cleared');
	}
</script>

<section class="section">
	<header>
		<h2 class="heading">God mode</h2>
		<span class="note">The delight layer, driven by hand.</span>
	</header>

	{#if !bound}
		<p class="note warn">No repository open — writes have nowhere to go.</p>
	{:else}
		<p class="note">
			Writing to <span class="mono">{repo.info?.name}</span>, as
			<span class="mono">{actor.name}</span>.
			{held.size} of {BADGES.length} held.
		</p>
	{/if}

	<!-- 1. Look at a card. Writes nothing. -->
	<div class="group">
		<h3 class="sub">Preview a reward moment</h3>
		<p class="note">Shows the card and plays its sound. Earns nothing; ignores Personality.</p>
		{#each groups as group (group.id)}
			<div class="line">
				<span class="note label">{group.label}</span>
				<div class="grid">
					{#each group.badges as found (found.id)}
						<BadgeChip
							{found}
							size="small"
							onclick={() => {
								delight.preview(found.id);
								sound(found.rarity);
							}}
						/>
					{/each}
				</div>
			</div>
		{/each}
	</div>

	<!-- 2. Drive the engine with the events it really sees. -->
	<div class="group">
		<h3 class="sub">Fire an event</h3>
		<p class="note">Real shapes through the real rules. What is earned is what would be earned.</p>
		<div class="demos">
			{#each DEMOS as demo (demo.id)}
				<div class="demo">
					<Chip disabled={!bound} onclick={() => fire(demo.id)} title={demo.what}>
						{demo.label}{demo.times && demo.times > 1 ? ` ×${demo.times}` : ''}
					</Chip>
					<span class="note">{demo.what}</span>
				</div>
			{/each}
		</div>
	</div>

	<!-- 3. Award and take back, bypassing the engine entirely. -->
	<div class="group">
		<h3 class="sub">Grant or revoke</h3>
		<p class="note">Straight into the record, no rule consulted. Click a badge to toggle it.</p>
		<div class="grid">
			{#each BADGES as found (found.id)}
				<BadgeChip
					{found}
					size="small"
					locked={!held.has(found.id)}
					equipped={actor.title === found.id}
					onclick={() => toggle(found.id)}
				/>
			{/each}
		</div>
	</div>

	<!-- 4. Whole-record operations, and the sounds. -->
	<div class="group">
		<h3 class="sub">The record</h3>
		<div class="row">
			<Btn disabled={!bound} onclick={() => delight.grantEvery()}>Grant every badge</Btn>
			<Btn disabled={!bound} onclick={() => delight.seedAgents()}>Seed three agents</Btn>
			<Btn disabled={!bound} onclick={clear}>Clear the record</Btn>
		</div>
		<p class="note" title="Seeding runs real tasks for Claude, GPT and Codex through the real engine.">Real tasks, real engine.</p>
	</div>

	<div class="group">
		<h3 class="sub">Sounds</h3>
		<p class="note">
			At the level Personality is set to (<span class="mono">{settings.settings.sound}</span>).
		</p>
		<div class="row wrap">
			{#each CUES as cue (cue.id)}
				<Chip onclick={() => sound(cue.id)}>{cue.label}</Chip>
			{/each}
		</div>

		<!--
			What the device says about itself, after something has asked it for a
			sound. It is here because of the failure that is otherwise invisible:
			on Linux, WebKitGTK renders Web Audio through GStreamer, and without
			the sink from `gst-plugins-good` the context reports itself running,
			every oscillator starts and stops exactly as asked, and nothing at
			all reaches the speakers. From the page, silence and success look
			identical — so the page says what it can and names the thing worth
			checking.
		-->
		{#if settings.settings.sound === 'off'}
			<p class="note">Sound is Off. Nothing here will make a noise.</p>
		{:else if audio}
			<p class="note" class:warn={!audio.supported}>
				Audio device: <span class="mono">{audio.state}</span>. {audio.note}
			</p>
		{/if}
	</div>
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 16px;
		max-width: 760px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.sub {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: 600;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.line {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	.label {
		flex: none;
		width: 140px;
		padding-top: 4px;
	}

	.grid {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	.demos {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 6px 16px;
	}

	.demo {
		display: flex;
		flex-direction: column;
		gap: 2px;
		align-items: flex-start;
		padding: 3px 0;
		border-bottom: 1px solid var(--soft);
	}

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.wrap {
		flex-wrap: wrap;
	}

	.warn {
		color: var(--warn);
	}
</style>
