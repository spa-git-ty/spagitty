<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { mod } from '$lib/platform';
	import Chip from '$lib/ui/Chip.svelte';
	import { theme } from '$lib/theme.svelte';
	import { FAMILIES, paletteOf, type Mode } from '$lib/themes';
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
	 * The palette: which family, and light or dark within it.
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
	 * Each family is shown in the mode that is on, so the swatches are the
	 * colours that would actually appear rather than a light preview of a theme
	 * about to be used in the dark.
	 */
	const swatches = $derived(
		FAMILIES.map((family) => {
			const palette = paletteOf(family.id, theme.mode);
			return {
				id: family.id,
				name: family.name,
				variant: family[theme.mode].name,
				colours: [palette.bg, palette.panel, palette.accent, palette.ink]
			};
		})
	);
</script>

<section class="section">
	<header>
		<h2 class="heading">Appearance</h2>
	</header>

	<div class="row">
		<span class="note label">Mode</span>
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

	<div class="row">
		<span class="note label">Theme</span>
		<span class="note">{theme.variant.name}</span>
	</div>

	<div class="families">
		{#each swatches as family (family.id)}
			<button
				class="family"
				class:active={theme.family === family.id}
				aria-pressed={theme.family === family.id}
				onclick={() => theme.setFamily(family.id)}
			>
				<span class="swatch" aria-hidden="true">
					{#each family.colours as colour, index (index)}
						<span class="chip-colour" style="background: {colour}"></span>
					{/each}
				</span>
				<span class="names">
					<span class="family-name">{family.name}</span>
					<span class="note">{family.variant}</span>
				</span>
			</button>
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
		<span class="note">Type and row height.</span>
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
			oninput={(event) => scale.setZoom(Number(event.currentTarget.value))}
		/>
		<span class="mono muted reading">{Math.round(scale.zoom * 100)}%</span>
		<Chip onclick={() => scale.setZoom(1)}>Reset</Chip>
		<!--
			The one thing here a person cannot work out by dragging the slider:
			that the same control has a keyboard shortcut, from anywhere.
		-->
		<span class="note">
			Everything. <span class="mono">{mod()}</span> with <span class="mono">+</span>,
			<span class="mono">−</span> or <span class="mono">0</span>.
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

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.label {
		width: 48px;
		flex: none;
	}

	.families {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 8px;
	}

	.family {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border: 1px solid var(--soft);
		border-radius: var(--r-field);
		text-align: left;
		min-width: 0;
	}

	.family:hover {
		border-color: var(--accent);
	}

	.family.active {
		border-color: var(--accent);
		background: var(--selection);
	}

	.swatch {
		display: flex;
		flex: none;
		border: 1px solid var(--soft);
		border-radius: var(--r-field);
		overflow: hidden;
	}

	.chip-colour {
		width: 12px;
		height: 24px;
	}

	.names {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.family-name {
		font-size: var(--fs-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
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
