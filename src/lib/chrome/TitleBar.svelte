<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">

	import { appWindow } from '$lib/chrome/window';
	import { isMac } from '$lib/platform';
	import Icon from '$lib/ui/Icon.svelte';
	import type { IconName } from '$lib/ui/icons';

	/**
	 * The title bar is the workspace bar: what this program is, the way back to
	 * every repository, and the ones open right now.
	 *
	 * The branch used to be here as a chip. It is on the toolbar's branch picker
	 * one row below and on the active tab, and three copies of one fact is two
	 * too many — so the bar says the name of the program and gets out of the way.
	 */

	/**
	 * On Linux and Windows the window has no platform decorations, so these are
	 * the only close, minimize and maximize controls there are.
	 *
	 * Deliberately neither macOS traffic lights nor Windows' full-height filled
	 * blocks: small, evenly weighted glyph buttons that read as Spagitty's own,
	 * and entirely colourless — they use the theme's neutral tokens and nothing
	 * else, including the close button.
	 *
	 * **Not on macOS** (TASK-042). Three neutral glyphs on the right is not what
	 * a Mac window looks like, and on that platform the difference between "an
	 * application" and "a web page inside a custom frame" is mostly this one
	 * detail. `src-tauri/tauri.macos.conf.json` restores real decorations with
	 * an overlay title bar, so the system draws its own traffic lights at the
	 * top left, over this bar — and drawing a second set on the right would be
	 * two answers to one question.
	 */
	const mac = isMac();
	const CONTROLS: { kind: string; icon: IconName; label: string; run: () => void }[] = [
		{ kind: 'minimize', icon: 'minimize', label: 'Minimize', run: () => appWindow.minimize() },
		{ kind: 'maximize', icon: 'maximize', label: 'Maximize', run: () => appWindow.toggleMaximize() },
		{ kind: 'close', icon: 'close', label: 'Close', run: () => appWindow.close() }
	];
</script>

<!-- Dragging the bar moves the window; double-clicking it maximizes, as a
     title bar is expected to. Controls stop the event so they don't drag. -->
<div
	class="titlebar"
	data-tauri-drag-region
	ondblclick={() => appWindow.toggleMaximize()}
	role="toolbar"
	tabindex="-1"
	aria-label="Window"
>
	<!--
		The empty side that makes the middle the middle (TASK-021). The bar is a
		three-column grid whose outer columns are equal, so the name sits in the
		centre of the *window* rather than in the centre of whatever the window
		controls left over. Without this the name would be centred in a space
		that is short by the width of three buttons, and land visibly left.
	-->
	<span class="side" class:traffic={mac} aria-hidden="true"></span>

	<span class="name">Spagitty</span>

	<!--
		The tabs and the way back to every repository were both here. The tabs
		have a row of their own now (FEAT-044) — they are a workspace control,
		and this row is window controls — and `All repositories` went with them
		rather than staying as a button that read like a tab which is always
		open. It is screen 1J on the rail, which is where the way back belongs.
	-->

	<!--
		What the title bar says is what it knows: which repository, and the ones
		open right now. The theme belongs to Settings → Appearance, which is the
		one place it is set; a second control here would be a second thing to keep
		in step. There was also a `⌘K` chip that opened Log search — the shortcut
		is `⌘F`, and writing a macOS key name on every platform for a combination
		that does nothing is worse than no hint at all.

		The build identity — licence and version — used to sit here too. It is the
		least changing fact in the application and it was in the most contested
		row, which also has to give way to tabs as repositories are opened; it is
		on the status strip along the bottom now (FEAT-043).
	-->

	<div class="controls">
		{#each mac ? [] : CONTROLS as control (control.kind)}
			<button
				class="control {control.kind}"
				title={control.label}
				aria-label={control.label}
				onclick={(event) => {
					event.stopPropagation();
					control.run();
				}}
			>
				<Icon name={control.icon} size="0.95em" weight={1.9} />
			</button>
		{/each}
	</div>
</div>

<style>
	.titlebar {
		height: var(--titlebar-h);
		flex: none;
		/*
		 * Three columns, outer two equal: empty, name, controls (TASK-021).
		 *
		 * `minmax(0, 1fr)` rather than `1fr` so the outer columns may shrink
		 * below their content on a narrow window — with a bare `1fr` the
		 * controls set a floor for both sides and the name is pushed off centre
		 * exactly when there is least room to lose.
		 *
		 * A grid rather than absolute positioning: an absolutely placed name
		 * would sit over the drag region and have to opt out of the pointer to
		 * let the window be dragged by its middle, and would need a stacking
		 * index to stay under the controls. Neither is needed if the layout
		 * simply says where the middle is.
		 */
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
		align-items: center;
		gap: 8px;
		padding: 0 10px;
		/*
		 * Glass. The bar takes its colour from the ambient light behind the
		 * window rather than being painted a shade of the panel, which is what
		 * makes it look like a pane laid over the application instead of a strip
		 * cut out of it.
		 */
		background-color: var(--chrome-veil);
		border-bottom: 1px solid color-mix(in srgb, var(--line) 60%, transparent);
		box-shadow: var(--glass-rim);
		font-size: var(--fs-secondary);
	}

	/* Hard against the right edge, whatever its column has been given. */
	.controls {
		justify-self: end;
		display: flex;
		align-items: center;
		gap: 2px;
		margin-left: 4px;
		margin-right: -6px;
	}

	.control {
		width: 24px;
		height: 22px;
		border-radius: var(--r-field);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-mono);
		line-height: 1;
		color: var(--muted);
		transition:
			background 0.1s ease,
			color 0.1s ease;
	}

	/* Colourless by design: no platform's palette, no red close button. The
	   affordance is a neutral tint from the theme's own tokens. */
	.control:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.control:active {
		background: var(--press);
		transform: scale(0.94);
	}

	/*
	 * The close button is the exception to the colourless rule above, and only
	 * on hover: every desktop in the world turns it red under the pointer, and
	 * a window whose close button looks exactly like its minimize button is the
	 * one place being unlike the platform costs somebody real work.
	 */
	.control.close:hover {
		background: var(--danger);
		color: var(--on-accent);
	}

	/* Bold, because it is the one thing on this bar that is not a control: it
	   says which program you are looking at, and everything else says state. */
	.name {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-weight: 700;
		letter-spacing: 0.01em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.side {
		min-width: 0;
	}

	/*
	 * Room for the traffic lights macOS draws over this bar (TASK-042).
	 *
	 * With `titleBarStyle: "Overlay"` the system's three buttons are painted at
	 * the top left *on top of* the webview, so anything Spagitty puts there is
	 * underneath them. Seventy-eight pixels is the width they occupy at the
	 * standard spacing; the outer columns of this grid are equal, so reserving
	 * it on the left keeps the name centred in the window rather than centred
	 * in what is left.
	 *
	 * Not a `--titlebar-*` metric, because it is not Spagitty's number — it is
	 * Apple's, it does not scale with the interface zoom, and publishing it as a
	 * token would invite something else to lay itself out against it.
	 */
	.side.traffic {
		min-width: 78px;
	}

	.muted {
		color: var(--muted);
	}

</style>
