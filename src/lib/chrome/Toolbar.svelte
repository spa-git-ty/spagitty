<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { activity, type ActivityKind, type Lane } from '$lib/activity/store.svelte';
	import BranchMenu from '$lib/branches/BranchMenu.svelte';
	import type { BranchOption } from '$lib/branches/pick';
	import { branches } from '$lib/branches/store.svelte';
	import { page } from '$app/state';
	import { graph } from '$lib/graph/store.svelte';
	import { commandLog } from '$lib/commandlog/store.svelte';
	import { fetchAll, pull, pushCurrent, quickStash } from '$lib/graph/actions';
	import { branching } from '$lib/graph/branching.svelte';
	import { network } from '$lib/network/store.svelte';
	import { remotes } from '$lib/remotes/store.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import type { IconName } from '$lib/ui/icons';
	import Loader from '$lib/ui/Loader.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';
	import { repo } from '$lib/repo.svelte';
	import { settings } from '$lib/settings/store.svelte';

	const head = $derived(repo.info?.head ?? null);

	/**
	 * The location line, and the dropdown that is actually a dropdown (FEAT-045).
	 *
	 * The repository's name is text: a name, not a control. What used to be a
	 * button here navigated to All repositories, which the rail and the tabs row
	 * both already reach, so the third route was a control that looked like a
	 * list and replaced the screen instead.
	 */
	const branchLabel = $derived(head?.branch ?? head?.short ?? '—');

	let branchMenu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	/**
	 * Opened under the control rather than at the pointer, so a keyboard
	 * activation puts the list in the same place a click does.
	 *
	 * The list is read on opening and not before: the toolbar is drawn for
	 * every screen, and reading every branch on start-up to fill a menu nobody
	 * opened is work done for nothing. `branches.load` is the same call the
	 * Branches screen makes and is guarded by the store's own sequence counter,
	 * so the two cannot race into a stale list.
	 */
	function openBranchMenu(event: MouseEvent) {
		// A second click on the control closes it. The menu leaves mousedowns on
		// its anchor alone precisely so that this decision is made here, once,
		// rather than by a mousedown that closes and a click that reopens
		// (BUG-018).
		if (branchMenu) {
			branchMenu = null;
			return;
		}
		// A switch would write the working copy under whatever is writing it
		// now (FEAT-121). The control says why, and stays where it is.
		if (switchWaits) return;
		const button = event.currentTarget as HTMLElement;
		const box = button.getBoundingClientRect();
		branchMenu = { x: box.left, y: box.bottom + 4, anchor: button };
		// Read again on every opening (FEAT-122): a branch made on the graph or
		// in Merger since the last one is otherwise missing from the list that
		// is meant to switch to it. What was read before is shown meanwhile.
		if (!branches.loading) void branches.load();
	}

	/**
	 * Local branches only.
	 *
	 * A remote-tracking ref is not a thing to check out — doing so detaches HEAD
	 * — so offering one in a switcher is how an accidental detached HEAD
	 * happens. The Branches screen still shows every ref there is.
	 *
	 * The branch already checked out is listed first, ticked and marked
	 * `current`, at full strength (FEAT-122). It used to be the one disabled
	 * row, which made where you are the faintest thing in the list.
	 */
	const BRANCH_OPTIONS: BranchOption[] = $derived(
		branches.rows
			.filter((row) => row.kind === 'branch')
			.sort((x, y) => Number(y.name === head?.branch) - Number(x.name === head?.branch))
			.map((row) => ({ name: row.name, group: 'local' as const }))
	);

	/** Why the branch cannot be switched just now, or null (FEAT-121). */
	const switchWaits = $derived(activity.blocked('repository'));

	/**
	 * There is no `PENDING` any more, and that is the change (BUG-030).
	 *
	 * Undo and Redo sat in the first group with `title: 'Not built yet'` and
	 * nothing else — no handler, no `disabled`, no `aria-disabled`. They
	 * rendered as ordinary toolbar buttons in the most prominent row in the
	 * application, they took the pointer, they took focus, they announced
	 * themselves to a screen reader as buttons, and clicking either did
	 * nothing at all. A tooltip does not repair that: a tooltip is read after
	 * the click, by a pointer user, on hover, if they wait.
	 *
	 * They are gone rather than disabled. A permanently dead control is still
	 * a claim that the feature is nearly here, and Spagitty's actual recovery
	 * story is not an undo stack — it is the Reflog screen, which is
	 * operation-specific, already built, and reachable from the rail and the
	 * palette. A general Undo would need a model of what each git operation
	 * reverses, and inventing one to fill a gap in a toolbar is the wrong
	 * reason to design it.
	 */

	interface ToolItem {
		icon: IconName;
		label: string;
		/** Where it goes, for the actions that are a screen. */
		href?: string;
		/** What it does, for the actions that are not. */
		act?: () => void;
		title?: string;
		/**
		 * What kind of work it starts and which lanes that work writes to
		 * (FEAT-121). While one of its kind runs, the button shows the strands;
		 * while a lane it needs is taken, it waits, dimmed, and says for what.
		 * Absent for an action that starts nothing by itself.
		 */
		kind?: ActivityKind;
		lanes?: Lane[];
		/**
		 * The alternatives this action offers, and the label its caret
		 * announces. Present means the button is a split button: the main half
		 * does the safe default, the caret opens the choices.
		 */
		alternatives?: { label: string; open: (anchor: HTMLElement) => void };
	}

	/**
	 * Two groups, divided: what talks to a remote, and what moves work about.
	 * Grouping is how a row of glyphs becomes something you can aim at without
	 * reading every label. It was three; the first held only Undo and Redo.
	 */
	type Anchored = { x: number; y: number; anchor: HTMLElement };

	let pullMenu = $state<Anchored | null>(null);
	let fetchMenu = $state<Anchored | null>(null);

	/**
	 * Where a menu opens: under the control, aligned to its left edge.
	 *
	 * Both of these used to open at the pointer, because both were reachable
	 * only by right-clicking. A menu opened at the pointer cannot be opened
	 * from the keyboard at all — there is no pointer — and the two would have
	 * appeared in different places depending on which half of the button was
	 * hit. Anchoring is the same arrangement the branch switcher already uses,
	 * and for the same reason (FEAT-045).
	 */
	function under(anchor: HTMLElement): Anchored {
		const box = anchor.getBoundingClientRect();
		return { x: box.left, y: box.bottom + 4, anchor };
	}

	/**
	 * Fetch offers one remote at a time (FEAT-018).
	 *
	 * Every layer has taken a remote since the plumbing was built; the button
	 * always sent the empty string, so "fetch one remote" existed everywhere
	 * except where somebody could ask for it — and then, once it existed, it
	 * was on a right-click handler, which is to say it existed for people who
	 * already knew it was there (BUG-030). The caret is what says so.
	 *
	 * The list is read on opening the menu rather than kept live: remotes change
	 * about once a year, and a store loaded on every repository change to fill a
	 * menu nobody opened would be work done for nothing.
	 */
	async function openFetchMenu(anchor: HTMLElement) {
		if (fetchMenu) {
			fetchMenu = null;
			return;
		}
		// Anchored before the await, so the menu lands under the control the
		// user pressed rather than wherever it has scrolled to by the time the
		// remotes have been read.
		const at = under(anchor);
		await remotes.load();
		fetchMenu = at;
	}

	const FETCH_ITEMS = $derived<MenuItem[]>([
		{ heading: 'Fetch' },
		{
			id: 'all',
			label: 'Every remote',
			note: settings.settings.pruneOnFetch ? 'pruning' : undefined,
			run: () => void fetchAll()
		},
		...(remotes.list.length > 0 ? [{ separator: true as const }] : []),
		...remotes.list.map((remote) => ({
			id: remote.name,
			label: remote.name,
			note: remote.url,
			run: () => void network.fetch(remote.name)
		}))
	]);

	/**
	 * A name field on the graph, in HEAD's row (FEAT-104) — not the Branches
	 * screen, and not a dialog. The graph is opened if another screen was.
	 */
	function branchHere() {
		// The commit selected on the graph, when the graph is what is showing
		// and a commit row is selected (BUG-055); HEAD otherwise.
		const selected = page.url.pathname === '/' ? graph.selected : null;
		const id = selected?.id ?? head?.id;
		if (!id) return;
		branching.start(id);
		if (page.url.pathname !== '/') void goto('/');
	}

	function openPullMenu(anchor: HTMLElement) {
		// A second press closes it, the convention `Menu` is built around
		// (BUG-018).
		pullMenu = pullMenu ? null : under(anchor);
	}

	/**
	 * The three ways to pull, offered rather than assumed.
	 *
	 * Clicking Pull takes the fast-forward-only path, because it is the one that
	 * cannot go wrong: it either moves the branch forward or refuses and says so.
	 * Merging and rebasing both write history and both can stop in a conflict, so
	 * they are a deliberate choice rather than what a single click does.
	 */
	const PULL_ITEMS: MenuItem[] = [
		{ heading: 'Pull' },
		{
			id: 'ff',
			label: 'Fast-forward only',
			note: 'never writes a commit',
			run: () => pull('fastForwardOnly')
		},
		{ id: 'merge', label: 'Merge if it cannot fast-forward', run: () => pull('merge') },
		{
			id: 'rebase',
			label: 'Rebase my commits on top',
			note: 'rewrites them',
			danger: true,
			run: () => pull('rebase')
		}
	];

	const GROUPS: ToolItem[][] = $derived([
		[
			{
				icon: 'pull',
				label: 'Pull',
				title: 'Fetch and fast-forward the current branch',
				kind: 'pull',
				lanes: ['repository', 'network'],
				act: () => pull(),
				alternatives: { label: 'How to pull', open: openPullMenu }
			},
			{
				icon: 'fetch',
				label: 'Fetch',
				title: settings.settings.pruneOnFetch
					? 'Fetch every remote, pruning'
					: 'Fetch every remote',
				kind: 'fetch',
				lanes: ['network'],
				act: () => fetchAll(),
				alternatives: { label: 'What to fetch', open: (anchor) => void openFetchMenu(anchor) }
			},
			{
				icon: 'push',
				label: 'Push',
				title: 'Push the current branch',
				kind: 'push',
				lanes: ['network'],
				act: () => pushCurrent()
			}
		],
		[
			{
				icon: 'branch',
				label: 'Branch',
				title: head?.id
					? 'Name a branch at the commit selected in the graph, or at HEAD'
					: 'Nothing to branch from yet',
				kind: 'branch',
				lanes: ['repository'],
				act: () => branchHere()
			},
			{
				icon: 'stash',
				label: 'Stash',
				title: 'Stash the work in progress',
				kind: 'stash',
				lanes: ['repository'],
				act: () => void quickStash()
			}
		]
	]);
</script>

{#if branchMenu}
	<BranchMenu
		x={branchMenu.x}
		y={branchMenu.y}
		anchor={branchMenu.anchor}
		label="Switch branch"
		options={BRANCH_OPTIONS}
		value={head?.branch ?? null}
		current={head?.branch ?? null}
		loading={!branches.loaded}
		onpick={(name) => void branches.checkout(name)}
		onclose={() => (branchMenu = null)}
	/>
{/if}

{#if fetchMenu}
	<Menu
		x={fetchMenu.x}
		y={fetchMenu.y}
		anchor={fetchMenu.anchor}
		label="What to fetch"
		items={FETCH_ITEMS}
		onclose={() => (fetchMenu = null)}
	/>
{/if}

{#if pullMenu}
	<Menu
		x={pullMenu.x}
		y={pullMenu.y}
		anchor={pullMenu.anchor}
		label="How to pull"
		items={PULL_ITEMS}
		onclose={() => (pullMenu = null)}
	/>
{/if}

<!--
	An ornament: a floating pill under the pane, centred in the status row
	(FEAT-082). It was a full-width bar across the top of the window.
-->
<div class="toolbar ornament" role="toolbar" aria-label="Repository actions">
	<!--
		What is running, in git's own words where it has given any (FEAT-121):
		a pull, a stash, a checkout and the rest, as well as the fetch and the
		push that used to say so in the pill.

		Over the pill rather than in it. A line inside widens the pill, the
		pill is centred, and so every button slid sideways from under the
		pointer that had just pressed one. Here nothing moves, and it takes no
		clicks from what is under it.
	-->
	{#if activity.running}
		<span class="working" role="status">
			<Loader size="inline" label="Working" />
			<span class="doing">{activity.label}</span>
		</span>
	{/if}
	<!--
		Where you are: the branch, as a real list (FEAT-045). The repository's
		name left with the bar — the active tab says it, one row up (FEAT-082).
	-->
	<div class="location">
		{#if repo.info}
			<button
				class="field"
				aria-haspopup="listbox"
				aria-expanded={branchMenu !== null}
				aria-disabled={switchWaits ? 'true' : undefined}
				title={switchWaits ?? 'Switch branch'}
				onclick={openBranchMenu}
			>
				<span class="value">{branchLabel}</span>
				<span class="mono muted" aria-hidden="true">▾</span>
			</button>
		{:else}
			<span class="none">no repository</span>
		{/if}

		<!--
			A checkout git refused. It is said here, where the action was taken,
			rather than left for the Branches screen to show — that screen may
			never be opened, and a switch that silently did not happen is the
			worst outcome this control has.
		-->
		{#if branches.writeError}
			<span class="note error" role="alert" title={branches.writeError}>{branches.writeError}</span>
		{/if}

		<!--
			A fetch or push that failed, in git's own words (FEAT-018), beside
			the location: it is about the repository. One that worked is said
			in a notice and then gone (BUG-091); its line used to stay here,
			hashes and all, until the next one replaced it.
		-->
		{#if activity.running}
			<!-- Said over the pill while it runs: see `.working`. -->
		{:else if network.error}
			<span class="note error" role="alert" title={network.error}>{network.error}</span>
		{/if}
	</div>

	<span class="vr" aria-hidden="true"></span>

	<div class="actions">
		{#each GROUPS as group, index (index)}
			{#if index > 0}
				<span class="vr"></span>
			{/if}
			{#each group as action (action.label)}
				<!--
					A split button where there are alternatives (BUG-030).

					The two halves are wrapped rather than made one control,
					because they do different things and both have to be
					reachable: the main half runs the safe default, the caret
					opens the choices. Both are `<button>`s, so both are in the
					tab order and both answer Enter and Space — which is the
					whole defect being fixed, since a `contextmenu` handler is
					unreachable without a pointer and undiscoverable with one.
				-->
				{@const waits = action.lanes ? activity.blocked(...action.lanes) : null}
				{@const mine = action.kind !== undefined && activity.is(action.kind)}
				<!--
					Dimmed, not removed, and not `disabled` (FEAT-121): a button
					that turned `disabled` under the keyboard would drop the
					focus, and one that cannot be hovered cannot say what it is
					waiting for.
				-->
				<div
					class="tool-group"
					class:split={action.alternatives !== undefined}
					class:waits={waits !== null}
				>
					<button
						class="tool"
						title={waits ?? action.title}
						aria-disabled={waits ? 'true' : undefined}
						aria-busy={mine || undefined}
						oncontextmenu={(event) => {
							if (!action.alternatives) return;
							if (waits) {
								event.preventDefault();
								return;
							}
							// Kept: it was the only way in, and taking it away
							// would break the habit of everybody who found it.
							event.preventDefault();
							action.alternatives.open(event.currentTarget as HTMLElement);
						}}
						onkeydown={(event) => {
							// The convention for a menu button, so the caret
							// does not have to be tabbed to separately.
							if (!action.alternatives || event.key !== 'ArrowDown') return;
							event.preventDefault();
							if (waits) return;
							action.alternatives.open(event.currentTarget as HTMLElement);
						}}
						onclick={() => {
							if (waits) return;
							if (action.act) action.act();
							else if (action.href) void goto(action.href);
						}}
					>
						<!--
							The strands take the icon's own box while this is the
							thing running, so the label does not move.
						-->
						<span class="glyph">
							{#if mine}<Loader size="inline" label="Working" />{:else}<Icon
									name={action.icon}
									size="1.25em"
								/>{/if}
						</span>
						<span class="label">{action.label}</span>
					</button>
					{#if action.alternatives}
						{@const open = action.alternatives.open}
						<button
							class="caret"
							aria-haspopup="menu"
							aria-expanded={(action.label === 'Pull' ? pullMenu : fetchMenu) !== null}
							aria-label={action.alternatives.label}
							aria-disabled={waits ? 'true' : undefined}
							title={waits ?? action.alternatives.label}
							onclick={(event) => {
								if (!waits) open(event.currentTarget as HTMLElement);
							}}
						>
							<span aria-hidden="true">▾</span>
						</button>
					{/if}
				</div>
			{/each}
		{/each}
	</div>

	<!--
		Only when the toggle is on. The feature is opt-in, and a button for it
		sitting in the chrome of every session would be a second, quieter answer
		to a question Settings already asks. The Settings gear that sat beside it
		is gone: Settings is on the rail (FEAT-082).
	-->
	{#if settings.settings.showGitCommands}
		<div class="trailing">
			<span class="vr" aria-hidden="true"></span>
			<button
				class="tool"
				title="What Spagitty has run"
				aria-pressed={commandLog.open}
				onclick={() => commandLog.toggle()}
			>
				<span aria-hidden="true">≡</span><span class="label">Commands</span>
			</button>
		</div>
	{/if}

	<!--
		No Commit button. Committing is the Working copy screen's job — it has the
		message box, the staged list and its own Commit — and a second one here
		was a button that could not do what it said: it navigated. The staged
		count went with it, to the screen that can act on it.
	-->
</div>

<style>
	/*
	 * The pill (FEAT-082). `.ornament` in `app.css` is the material; this is
	 * the shape. It sizes to what it holds and is centred by the status row, so
	 * nothing here needs to know how wide the window is.
	 */
	.toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 46px;
		padding: 0 7px;
		border-radius: var(--r-ornament);
		min-width: 0;
		max-width: 100%;
		position: relative;
		z-index: 2;
	}

	.vr {
		height: 22px;
	}

	.trailing {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.location {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		overflow: hidden;
	}

	.none {
		padding: 0 8px;
		color: var(--muted);
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	/* git's own progress line, which can be long. It gets a bounded share of
	   the pill rather than pushing the actions out of it. */
	.location .note {
		max-width: 220px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/*
	 * What is running (FEAT-121): a small capsule over the middle of the pill.
	 * It waits a moment before it shows: most checkouts and stashes are over
	 * inside that moment, and a line that flashed for each of them would be
	 * noise about nothing.
	 */
	.working {
		position: absolute;
		left: 50%;
		bottom: calc(100% + 8px);
		transform: translateX(-50%);
		display: inline-flex;
		align-items: center;
		gap: 7px;
		max-width: min(440px, calc(100vw - 32px));
		padding: 5px 13px;
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-pill);
		background: var(--surface-2);
		box-shadow: var(--shadow-2);
		font-size: var(--fs-secondary);
		color: var(--ink);
		white-space: nowrap;
		pointer-events: none;
		animation: settle 0.2s var(--ease) 0.18s both;
	}

	.doing {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	@keyframes settle {
		from {
			opacity: 0;
		}
	}

	/*
	 * Waiting for what is running. Dimmed after the same moment, for the
	 * same reason, and nothing else about it changes: it keeps its place,
	 * its focus and its title, which now says what it is waiting for.
	 */
	.field[aria-disabled='true'],
	.waits .tool,
	.waits .caret {
		opacity: 0.42;
		cursor: default;
		transition: opacity 0.2s var(--ease) 0.18s;
	}

	.field[aria-disabled='true']:hover {
		border-color: var(--pane-edge);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
	}

	.waits .tool:hover,
	.waits .caret:hover,
	.waits .tool:active,
	.waits .caret:active {
		color: var(--ink);
		background: transparent;
	}

	.waits .tool:hover :global(svg) {
		color: var(--muted);
	}

	/* The one that is running keeps its strength: it is the answer to "did
	   that do anything", not something in the way. */
	.waits .tool[aria-busy='true'] {
		opacity: 1;
	}

	.glyph {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.25em;
		height: 1.25em;
		flex: none;
	}

	/* An error is the palette's red, which is what the rest of the application
	   uses to mean "this did not work". */
	.error {
		color: var(--danger);
	}

	/*
	 * The branch picker: a capsule inside the capsule, filled a shade deeper
	 * than the pill so it reads as the one thing here that holds a value.
	 */
	.field {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		height: 32px;
		padding: 0 12px;
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-pill);
		background: color-mix(in srgb, var(--sunken) 70%, transparent);
		min-width: 0;
		max-width: 170px;
		transition:
			border-color var(--t-fast) var(--ease),
			background var(--t-fast) var(--ease);
	}

	.field:hover {
		border-color: color-mix(in srgb, var(--accent) 55%, var(--line));
		background: color-mix(in srgb, var(--sunken) 70%, var(--accent) 8%);
	}

	.value {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	/*
	 * A button and, where there are alternatives, its caret.
	 *
	 * The two are one visual object: separate pills would read as two actions,
	 * and one control would leave the choices reachable only by guessing which
	 * half to press. The group carries the capsule so the halves meet flush.
	 */
	.tool-group {
		display: flex;
		align-items: stretch;
		height: 32px;
		border-radius: var(--r-pill);
	}

	.tool {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 32px;
		padding: 0 11px;
		border-radius: var(--r-pill);
		font-size: var(--fs-secondary);
		color: var(--ink);
		user-select: none;
		white-space: nowrap;
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	.tool-group .tool {
		border-radius: inherit;
	}

	.tool :global(svg) {
		color: var(--muted);
		transition: color var(--t-fast) var(--ease);
	}

	.split .tool {
		border-start-end-radius: 0;
		border-end-end-radius: 0;
		padding-inline-end: 6px;
	}

	.tool:hover {
		color: var(--accent);
		background: var(--accent-soft);
	}

	.tool:hover :global(svg) {
		color: var(--accent);
	}

	.tool:active {
		background: var(--press);
	}

	/*
	 * The caret. Its own button rather than a glyph inside the first one: it is
	 * what makes the alternatives visible, and a `<button>` is what makes them
	 * reachable from the keyboard (BUG-030).
	 */
	.caret {
		display: flex;
		align-items: center;
		padding: 0 10px 0 5px;
		border-radius: inherit;
		border-start-start-radius: 0;
		border-end-start-radius: 0;
		font-size: var(--fs-mono);
		color: var(--muted);
		user-select: none;
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	.caret:hover,
	.caret[aria-expanded='true'] {
		color: var(--accent);
		background: var(--accent-soft);
	}

	.caret:active {
		background: var(--press);
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 2px;
		flex: none;
	}

	.actions .vr {
		margin: 0 6px;
	}

	/*
	 * Narrower than the pill's natural width, the labels go and the icons stay.
	 * The caret's glyph is not a label and must not go with them.
	 */
	@media (max-width: 1180px) {
		.tool > .label {
			display: none;
		}

		.tool {
			padding: 0 9px;
		}

		.split .tool {
			padding-inline-end: 4px;
		}
	}

	@media (max-width: 760px) {
		.trailing,
		.location .note {
			display: none;
		}
	}
</style>
