<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { density, DENSITIES } from '$lib/graph/density.svelte';
	import { mod } from '$lib/platform';
	import Chip from '$lib/ui/Chip.svelte';
	import { theme } from '$lib/theme.svelte';
	import type { Mode } from '$lib/themes';
	import {
		scale,
		TEXT_MAX,
		TEXT_MIN,
		TEXT_STEP,
		ZOOM_MAX,
		ZOOM_MIN,
		ZOOM_STEP
	} from '$lib/scale.svelte';

	/**
	 * The palette: light or dark, or the desktop's own (TASK-051: one family).
	 *
	 * This is the one place either is set — the title bar used to carry a
	 * toggle as well, which meant two controls for one preference.
	 *
	 * The theme is not one of the behaviour toggles and is not stored with
	 * them: it has to be applied before anything has been read from disk, so it
	 * lives in `localStorage` where the boot path can reach it. See
	 * `src/lib/theme.svelte.ts`.
	 */
	const MODES: { id: Mode; label: string }[] = [
		{ id: 'light', label: 'Light' },
		{ id: 'dark', label: 'Dark' }
	];

	/**
	 * Following the desktop is a third choice beside Light and Dark, not a
	 * checkbox above them (BUG-031).
	 *
	 * The three are one decision — where light-or-dark comes from — and a
	 * switch that greyed the other two out would be two controls for it. The
	 * chip that is active is the answer; pressing Light or Dark is the act of
	 * taking over, which is why `theme.setMode` sets the source itself.
	 */
	const following = $derived(theme.source === 'system');

	/**
	 * Following the desktop's whole palette (FEAT-080).
	 *
	 * Offered only when one has actually been read. The alternative — always
	 * showing it and explaining on click why it cannot be used — puts a control
	 * that does nothing on the screen of every user who is not running Omarchy,
	 * which is the defect BUG-030 was about.
	 */
	const desktop = $derived(theme.source === 'omarchy');
</script>

<section class="section">
	<header>
		<h2 class="heading">Appearance</h2>
	</header>

	<div class="row">
		{#each MODES as option (option.id)}
			<Chip
				active={!following && theme.mode === option.id}
				onclick={() => theme.setMode(option.id)}
			>
				{option.label}
			</Chip>
		{/each}
		<Chip
			active={following}
			title="Follow the desktop's light and dark setting, and keep following it"
			onclick={() => theme.followSystem()}
		>
			Follow system
		</Chip>
		<!--
			What "follow system" is currently resolving to. Without it the
			control says what it will do and never what it did, which on a
			desktop that has just changed is the one thing worth showing.
		-->
		{#if following}
			<span class="note">now {theme.mode}</span>
		{/if}
	</div>

	{#if theme.desktopAvailable}
		<div class="row">
			<Chip
				active={desktop}
				title="Take the colours from the desktop's own theme, and follow it as it changes"
				onclick={() => theme.followDesktop()}
			>
				Follow Omarchy
			</Chip>
			{#if desktop}
				<!--
					The desktop's own name for what is on. Worth showing because
					a followed palette has no family and no variant, so the row
					above it says nothing about which colours these are.
				-->
				<span class="note">{theme.desktopName ?? 'the desktop palette'}</span>
			{/if}
		</div>
	{/if}

	<div class="hr"></div>

	<!--
		How much room the graph column asks for (TASK-041).

		Here rather than on the Graph screen's own header: it is an appearance
		preference, it persists, and it belongs beside the other two dials that
		decide how much of the window the work gets.
	-->
	<div class="row">
		<span class="note label">Graph</span>
		{#each DENSITIES as option (option.id)}
			<Chip
				active={density.id === option.id}
				title={option.note}
				onclick={() => density.set(option.id)}
			>
				{option.label}
			</Chip>
		{/each}
	</div>

	<div class="hr"></div>

	<div class="row">
		<span class="note label">Text</span>
		<input
			class="slider"
			type="range"
			min={TEXT_MIN}
			max={TEXT_MAX}
			step={TEXT_STEP}
			value={scale.text}
			aria-label="Text size"
			oninput={(event) => scale.setText(Number(event.currentTarget.value))}
		/>
		<span class="mono muted reading">{Math.round(scale.text * 100)}%</span>
		<Chip onclick={() => scale.setText(1)}>Reset</Chip>
	</div>

	<div class="row">
		<span class="note label">Zoom</span>
		<input
			class="slider"
			type="range"
			min={ZOOM_MIN}
			max={ZOOM_MAX}
			step={ZOOM_STEP}
			value={scale.zoom}
			aria-label="Interface zoom"
			title="Also {mod()} with +, − or 0, from anywhere"
			oninput={(event) => scale.setZoom(Number(event.currentTarget.value))}
		/>
		<span class="mono muted reading">{Math.round(scale.zoom * 100)}%</span>
		<Chip onclick={() => scale.setZoom(1)}>Reset</Chip>
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

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.label {
		width: 48px;
		flex: none;
	}









	.slider {
		flex: 1;
		min-width: 0;
		max-width: 260px;
		accent-color: var(--accent);
	}

	/* Fixed width so the number does not shift the Reset chip as it changes. */
	.reading {
		width: 44px;
		flex: none;
		text-align: right;
	}

</style>
