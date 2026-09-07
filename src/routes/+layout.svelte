<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { fly } from 'svelte/transition';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import '../app.css';

	import * as api from '$lib/api';
	import CloneModal from '$lib/clone/CloneModal.svelte';
	import { clone } from '$lib/clone/store.svelte';
	import WorktreesModal from '$lib/worktrees/WorktreesModal.svelte';
	import AddWorktreeModal from '$lib/worktrees/AddWorktreeModal.svelte';
	import SubmodulesModal from '$lib/submodules/SubmodulesModal.svelte';
	import { network } from '$lib/network/store.svelte';
	import { rebase } from '$lib/rebase/store.svelte';
	import CommandLog from '$lib/commandlog/CommandLog.svelte';
	import { commandLog } from '$lib/commandlog/store.svelte';
	import RewardOverlay from '$lib/delight/RewardOverlay.svelte';
	import { delight } from '$lib/delight/store.svelte';
	import NavRail from '$lib/chrome/NavRail.svelte';
	import ResizeEdges from '$lib/chrome/ResizeEdges.svelte';
	import { appWindow } from '$lib/chrome/window';
	import RepoTabs from '$lib/chrome/RepoTabs.svelte';
	import StatusStrip from '$lib/chrome/StatusStrip.svelte';
	import TitleBar from '$lib/chrome/TitleBar.svelte';
	import Toolbar from '$lib/chrome/Toolbar.svelte';
	import { avatars } from '$lib/graph/avatars.svelte';
	import { graph } from '$lib/graph/store.svelte';
	import { ROW_PITCH } from '$lib/metrics';
	import Palette from '$lib/palette/Palette.svelte';
	import { palette } from '$lib/palette/store.svelte';
	import { registerCommands } from '$lib/palette/commands';
	import { panels } from '$lib/panels.svelte';
	import { repo } from '$lib/repo.svelte';
	import { scale } from '$lib/scale.svelte';
	import { settings } from '$lib/settings/store.svelte';
	import DialogHost from '$lib/ui/DialogHost.svelte';
	import NoticeToast from '$lib/ui/NoticeToast.svelte';
	import { settings as settingsStore } from '$lib/settings/store.svelte';
	import Splitter from '$lib/ui/Splitter.svelte';
	import { theme } from '$lib/theme.svelte';
	import { resumeSession } from '$lib/session';
	import { workspace } from '$lib/workspace.svelte';
	import { REPO_CHANGED_EVENT, type RepoChangedEvent } from '$lib/types';

	let { children } = $props();

	/** Re-walk when a ref moves, but not on every keystroke into a rebase. */
	let refreshTimer: ReturnType<typeof setTimeout> | null = null;

	const cleanups: Array<() => void> = [];

	onMount(() => {
		theme.init();
		// Publishes the structural metrics as well as the type scale, at the
		// stored zoom — so there is no frame at 100% before the user's zoom
		// arrives.
		scale.init();
		// After the metrics, so stored panel widths win over the defaults.
		panels.init();
		// The tab strip, before anything can open a repository into it.
		workspace.init();
		registerCommands();
		// The window's own corner and shadow come off when it is maximized, and
		// CSS cannot ask Tauri whether it is (FEAT-037).
		appWindow.watchMaximized().then((off) => cleanups.push(off));

		if (!api.inTauri()) return;

		let cancelled = false;

		// Whether there is a newer Spagitty, asked once at startup and only if
		// the preference says so — which means reading the preference before
		// asking, because a setting that stops a request has to stop it before
		// it is made.
		//
		// Deliberately not awaited with the rest: it is the one thing here that
		// touches a network, and nothing on screen should wait on it. A failure
		// is left in the Settings screen rather than raised as a notice — a
		// toast on every launch behind a captive portal would be worse than the
		// feature is worth.
		(async () => {
			try {
				const stored = await api.settings();
				if (cancelled || !stored.checkForUpdates) return;
				await settingsStore.checkForUpdate();
			} catch {
				// Settings unreadable, or the check failed. Neither is a reason
				// to interrupt somebody opening a repository.
			}
		})();

		(async () => {
			// Listeners go up before anything can emit, so the first batch of a
			// walk is never missed.
			cleanups.push(await graph.attach());
			// A clone survives navigation, so its listener belongs to the shell
			// rather than to whichever screen started it.
			cleanups.push(await clone.attach());
			// A rebase survives navigation for the same reason: people leave the
			// screen for Conflicts while it is still running, and its progress
			// must not stop being heard when they do.
			cleanups.push(await rebase.attach());
			// A fetch survives navigation too: somebody who starts one from the
			// toolbar and walks to Branches should not stop hearing about it.
			cleanups.push(await network.attach());
			// Recording starts with the app, not with the panel: turning the
			// toggle on mid-session should show what has already run.
			cleanups.push(await commandLog.attach());

			// The toggles are read once, here, rather than by the Settings
			// screen alone. Everything that consults them — the confirmation
			// before a history rewrite, the command log — is reachable without
			// ever opening Settings, and until this read lands they answer from
			// the defaults instead of from what the user chose.
			settings.load();

			const off: UnlistenFn = await listen<RepoChangedEvent>(
				REPO_CHANGED_EVENT,
				(event) => {
					if (event.payload.refs) {
						// Refs moved: HEAD, the chips and the history itself are
						// all potentially stale, so re-read and re-walk.
						if (refreshTimer) clearTimeout(refreshTimer);
						refreshTimer = setTimeout(() => {
							repo.refresh();
							graph.reload();
						}, 100);
					} else if (event.payload.worktree) {
						repo.refresh();
					}
				}
			);
			cleanups.push(off);

			// Guard against the two definitions of the row pitch drifting apart.
			try {
				const rust = await api.metrics();
				if (rust.rowPitch !== ROW_PITCH) {
					console.error(
						`row pitch mismatch: Rust says ${rust.rowPitch}, frontend says ${ROW_PITCH}. ` +
							'Lanes and rows will not line up.'
					);
				}
			} catch {
				// Older backend without the command; not fatal.
			}

			/*
			 * What this launch opens: the path on the command line, or else the
			 * tab the last session was on, with its route and its selection
			 * (BUG-013).
			 *
			 * The order lives in `$lib/session` rather than here, because a
			 * missing call in it is exactly the bug that got through, and
			 * `src/routes/**` is outside the coverage scope — nothing written in
			 * this file can be asserted on (TASK-019).
			 */
			await resumeSession({
				launchPath: api.launchPath,
				open: (path) => repo.open(path),
				active: () => workspace.active,
				placeOf: (path) => workspace.placeOf(path),
				route: () => page.url.pathname,
				goto,
				want: (id) => graph.want(id),
				cancelled: () => cancelled
			});
		})();

		return () => {
			cancelled = true;
			if (refreshTimer) clearTimeout(refreshTimer);
			for (const cleanup of cleanups) cleanup();
		};
	});

	/**
	 * `Ctrl+F` — or the command key equivalent on macOS — reaches Log search from
	 * anywhere, with the first field focused. The focus is carried in the URL
	 * rather than through a store, so the same shortcut and a bookmark behave
	 * identically.
	 */
	function shortcut(event: KeyboardEvent) {
		if (!(event.metaKey || event.ctrlKey) || event.altKey) return;

		switch (event.key) {
			case 'f':
				event.preventDefault();
				goto('/search?focus=1');
				return;
			case 'p':
				event.preventDefault();
				palette.toggle();
				return;
			// `=` is the unshifted key most keyboards put `+` on, and browsers
			// report both. Accepting only one means the shortcut works on some
			// layouts and not others.
			case '+':
			case '=':
				event.preventDefault();
				scale.zoomIn();
				return;
			case '-':
				event.preventDefault();
				scale.zoomOut();
				return;
			case '0':
				event.preventDefault();
				scale.reset();
				return;
		}
	}

	// A newly opened repository means a fresh walk.
	let lastGeneration = 0;
	$effect(() => {
		if (repo.generation !== lastGeneration) {
			lastGeneration = repo.generation;
			graph.restart();
		}
	});

	/*
	 * The delight layer follows the open repository and the git identity
	 * (FEAT-072).
	 *
	 * Both are pushed to it rather than read by it: the badge record must not
	 * import the repository store, because almost everything that reports a git
	 * operation is reachable from `repo` and importing it back would close a
	 * cycle through half the frontend. The shell already knows both facts, so
	 * the shell is where they are handed over.
	 */
	$effect(() => {
		delight.bind(repo.info?.path ?? null);
	});

	$effect(() => {
		const identity = settings.identity;
		if (!identity) return;
		delight.identify(
			identity.name?.local ?? identity.name?.global ?? null,
			identity.email?.local ?? identity.email?.global ?? null
		);
	});

	/*
	 * Author pictures follow the preference (FEAT-079).
	 *
	 * Here rather than inside the avatar store, which has no component to own
	 * an effect, and here rather than on the Graph screen, which is not the
	 * only thing that draws a face and is not mounted when the preference is
	 * changed on Settings. The store starts disabled, so nothing leaves the
	 * machine in the window between the first paint and this read landing.
	 */
	$effect(() => {
		avatars.setEnabled(settings.settings.fetchAvatars);
	});
</script>

<svelte:window onkeydown={shortcut} />

<div class="app">
	<!--
		The column the shell is laid out in.

		It was the element a pane of glass bent (FEAT-057): `.app` carries the
		window's outline and cast shadow, both drawn outside its border box, and
		a filter clips to that box — so the filter went on an inner element that
		casts nothing. The lens was retired in TASK-022 when it was measured at
		180ms of every frame, and nothing filters anything now.

		The element stays because the layout does: it holds the chrome column and
		is the positioning context `.ground` is placed against. Folding it back
		into `.app` is a tidy-up worth doing on its own, not as a passenger to a
		performance fix.
	-->
	<div class="lens">
		<!--
			The ground the window paints on: flat `--bg`, `aria-hidden` and
			pointer-transparent, because it is a material property of the window
			rather than content. It carried the ambient washes until those were
			taken out, and it stays because FEAT-055 needs it — with the DMABuf
			renderer off, WebKitGTK keeps painting reliably only on a layer it
			was given at startup, and without this one the content area stops
			painting and the desktop shows through the window.
		-->
		<div class="ground" aria-hidden="true"></div>

		<TitleBar />
		<!-- Its own row, and absent when nothing is open (FEAT-044). -->
		<RepoTabs />
		<Toolbar />
		<div class="main">
			<NavRail />
			<Splitter panel="rail" label="Resize the nav rail" />
			<!--
				Screens arrive rather than appear (FEAT-053).

				Keyed on the path, so every navigation remounts the screen inside
				a short upward slide — which is what a rail click already does
				invisibly. The motion is 140ms and 6px: enough to say "this is a
				different screen", not enough to wait for. `prefers-reduced-motion`
				turns it off in `app.css`, along with everything else that moves.
			-->
			{#key page.url.pathname}
				<div class="screen-slot" in:fly={{ y: 6, duration: 140 }}>
					{@render children()}
				</div>
			{/key}
		</div>
		<!--
			The window's own bottom edge (FEAT-043). Outside `.main`, so it spans
			the rail as well as the screen, and after it, so nothing scrolls over
			it.
		-->
		<StatusStrip />
	</div>
</div>

<!--
	Mounted here rather than by a screen: a clone keeps running while the user
	navigates, and a modal owned by a screen would go with it.
-->
<CloneModal />
<WorktreesModal />
<AddWorktreeModal />
<SubmodulesModal />

<!-- Reaches every command from every screen, so it belongs to the shell. -->
<Palette />

<!--
	Every confirmation and every result. Both are mounted once, here, because an
	action started on the graph can finish after the user has navigated away —
	a dialog owned by a screen would take the question with it.
-->
<DialogHost />
<NoticeToast />

<!--
	The reward moment (FEAT-072). Mounted by the shell for the same reason as
	the dialog and the notice: a rebase started on the Graph screen can finish
	after the user has walked to Conflicts, and the badge it earns has to arrive
	wherever they are.
-->
<RewardOverlay />

<!--
	The record of what Spagitty ran. Mounted by the shell for the same reason as
	the dialog: a command started on one screen finishes wherever the user is.
-->
<CommandLog />

<!-- The window is undecorated, so it provides its own resize edges. -->
<ResizeEdges />

<style>
	/*
	 * The card the whole application sits on (FEAT-037).
	 *
	 * The window is transparent and undecorated, so the corner, the edge and the
	 * shadow are all drawn here. Three things together make it read as a
	 * physical surface rather than a flat rectangle:
	 *
	 * - a **hairline outline** at sub-pixel width, which on a HiDPI display
	 *   lands as a real edge and on a 1x display as a faint one. Heavier and it
	 *   reads as a border, which is a different thing — a border belongs to a
	 *   component, an edge belongs to a window;
	 * - an **inset highlight** along the top, where light would catch a raised
	 *   surface. It is what stops the outline reading as a drawn line;
	 * - a **two-part shadow** — a tight, dark contact shadow holding the card
	 *   down, and a wide, soft one giving it height. One shadow can do either
	 *   but not both, and a single mid-sized blur is what makes a page look
	 *   like it has a sticker on it.
	 */
	.app {
		flex: 1;
		min-height: 0;
		height: 100%;
		width: 100%;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg);
		/* The lens is positioned against this. */
		position: relative;
		border-radius: var(--r-window);
		outline: 0.2px solid var(--window-edge);
		outline-offset: -0.2px;
		box-shadow:
			0 1px 2px var(--window-contact),
			0 12px 30px var(--window-cast),
			inset 0 1px 0 var(--window-sheen);
	}

	/*
	 * The shell's column (was the filtered element, FEAT-057, until TASK-022).
	 *
	 * It fills `.app` exactly and draws nothing of its own — no outline, no
	 * shadow, no background to double the one underneath. That was what let a
	 * filter sit on it without clipping anything visible; it is now simply why
	 * the element is invisible.
	 *
	 * It inherits the window's radius and carries the column layout, both of
	 * which are still load-bearing.
	 */
	.lens {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		position: relative;
		border-radius: inherit;
	}

	/*
	 * Square against the screen edge, and nothing to cast a shadow onto.
	 *
	 * `flush` is the same picture arrived at from the other direction: the
	 * compositor drew the corner and the shadow already, so a second set inside
	 * them is a card sitting in a window rather than a window (BUG-029). The
	 * outline goes too — it was the inner card's edge, and against the
	 * compositor's own border it reads as a double rule.
	 */
	:global(:root[data-window='maximized']) .app,
	:global(:root[data-window='flush']) .app {
		border-radius: 0;
		box-shadow: none;
	}

	:global(:root[data-window='flush']) .app {
		outline: none;
	}

	.main {
		flex: 1;
		min-height: 0;
		display: flex;
		overflow: hidden;
		/* Above the ground layer, below the chrome. */
		position: relative;
		z-index: 1;
	}

	/*
	 * The ground (see the note in the markup). One flat promoted layer that
	 * never animates and never repaints for content — content paints above it,
	 * and it exists so the window always has a surface that does paint.
	 *
	 * `z-index: -1` rather than `0`: it has to sit under the chrome bars, none
	 * of which carry a stacking index of their own, while still painting over
	 * `.app`'s own background.
	 */
	.ground {
		position: absolute;
		inset: 0;
		z-index: -1;
		pointer-events: none;
		background: var(--bg);
		transform: translateZ(0);
	}

	/*
	 * The box a screen is transitioned inside. It has to be a flex container of
	 * its own: every screen is `flex: 1` against `.main`, and wrapping one in a
	 * plain div would leave it sized by its content instead of by the window.
	 */
	.screen-slot {
		flex: 1;
		min-width: 0;
		min-height: 0;
		height: 100%;
		width: 100%;
		display: flex;
		overflow: hidden;
	}

</style>
