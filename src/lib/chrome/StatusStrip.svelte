<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { version } from '$lib/version';
	import { relativeTime } from '$lib/format';
	import { graph } from '$lib/graph/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import { settings } from '$lib/settings/store.svelte';
	import { profiles } from '$lib/profiles/store.svelte';
	import Menu from '$lib/ui/Menu.svelte';
	import type { MenuItem } from '$lib/ui/menu';

	/**
	 * The state of the open repository lives here, not in the rail (FEAT-040
	 * moved it into the rail's foot; this moves it out again).
	 *
	 * It was four lines stacked under the rail's screens — how much is changed,
	 * how fresh the walk and the remote are, how many tags and submodules — and
	 * two of the four were second copies of counts the rail's own rows already
	 * carry as badges. Stacked there they read as a fifth navigation block, they
	 * pushed the screens up, and they took the width of the rail to say what
	 * fits on one line of a strip that spans the window.
	 *
	 * A status strip is where an application says what is true of the thing it
	 * has open. That is exactly what these are, so they are one row here,
	 * between the identity and the licence, and the rail is navigation again.
	 */
	const counts = $derived(repo.counts);

	const identity = $derived(settings.identity);
	const name = $derived(identity?.name.effective ?? null);
	const email = $derived(identity?.email.effective ?? null);

	let menu = $state<{ x: number; y: number; anchor: HTMLElement } | null>(null);

	const activeProfile = $derived(
		profiles.list.find((p) => p.authorName === name && p.authorEmail === email)
	);

	function openProfileMenu(event: MouseEvent) {
		if (menu) {
			menu = null;
			return;
		}
		const btn = event.currentTarget as HTMLElement;
		const box = btn.getBoundingClientRect();
		menu = { x: box.left, y: box.top - 4, anchor: btn };
	}

	const menuItems = $derived.by((): MenuItem[] => {
		const items: MenuItem[] = profiles.list.map((p) => ({
			id: p.id,
			label: p.name,
			note: `${p.authorName} <${p.authorEmail}>`,
			run: () => void profiles.apply(p, false)
		}));
		items.push({
			id: 'manage-profiles',
			label: 'Manage Profiles…',
			run: () => {
				window.location.href = '/settings#you';
			}
		});
		return items;
	});

	/**
	 * A count of `null` means "not computed yet" and reads as a dot. Inventing
	 * the rest would make the strip lie about how much work is waiting.
	 */
	const workingLabel = $derived(
		counts.working === null
			? 'working copy not read yet'
			: counts.working === 0
				? 'working copy clean'
				: `${counts.working} changed ${counts.working === 1 ? 'file' : 'files'}`
	);

	const tagsLabel = $derived(counts.tags === null ? '·' : String(counts.tags));
	const submodulesLabel = $derived(counts.submodules === null ? '·' : String(counts.submodules));

	/**
	 * `now` is a signal so the ages re-read when anything else changes; nothing
	 * here polls. A strip that ticked would draw the eye to its quietest row.
	 */
	let now = $state(Date.now());

	const refreshed = $derived(
		graph.refreshedAt === null
			? 'not refreshed yet'
			: `refreshed ${relativeTime(graph.refreshedAt, now)}`
	);

	const fetched = $derived.by(() => {
		if (!repo.info) return null;
		const at = repo.info.lastFetched;
		// An empty time, or a time invented for a fetch that never happened, is
		// the thing this must not do.
		return at === null ? 'never fetched' : `fetched ${relativeTime(at, now)}`;
	});

	// Re-read whenever the walk finishes or the counts move, which is every
	// moment the numbers behind these could have changed.
	$effect(() => {
		void graph.refreshedAt;
		void repo.counts.working;
		void repo.info?.lastFetched;
		now = Date.now();
	});

	onMount(() => {
		void profiles.fetch();
	});
</script>

<div class="strip" role="contentinfo" aria-label="Application status">
	<span class="left">
		{#if name || email}
			<button
				type="button"
				class="profile-btn"
				title={email ? `${name ?? ''} <${email}>` : (name ?? '')}
				onclick={openProfileMenu}
			>
				<span class="avatar-dot">👤</span>
				<span class="profile-text">
					{#if activeProfile}
						<b>{activeProfile.name}</b> ({name})
					{:else}
						{name ?? email}
					{/if}
				</span>
			</button>
		{/if}
	</span>

	{#if repo.info}
		<!--
			Two groups, not one run of dots.

			They answer different questions. **State** is what is happening and
			how current it is — the walk, the working copy, the last fetch — and
			every part of it can change while you look at it. **Counts** is
			inventory: how many commits, tags and submodules this repository has.
			Strung together with the same `·` between all of them, the eye had no
			way to tell "1 changed file" (something to do) from "Tags 42"
			(something that is merely true), and the freshness pair got read as
			part of the tally.

			So: the state group, a rule, the counts group. The rule is a real
			divider rather than a wider gap, because a gap in a row of dot-
			separated text reads as a typo.
		-->
		<span class="repo note">
			<span class="group state">
				<span class="walk" class:running={!graph.complete}>
					<span class="pulse" aria-hidden="true"></span>
					<span>{graph.complete ? 'Repository ready' : 'Loading history…'}</span>
				</span>
				<span class="sep" aria-hidden="true">·</span>
				<span class="fact">{workingLabel}</span>
				<span class="sep" aria-hidden="true">·</span>
				<span class="fact">{refreshed}{fetched ? ` · ${fetched}` : ''}</span>
			</span>

			<span class="divider tail" aria-hidden="true"></span>

			<span class="group counts tail">
				<span class="fact">{graph.count} commits</span>
				<span class="sep" aria-hidden="true">·</span>
				<span class="fact">Tags {tagsLabel}</span>
				<span class="sep" aria-hidden="true">·</span>
				<span class="fact">Submodules {submodulesLabel}</span>
			</span>
		</span>
	{/if}

	<span class="note mono license" title={version.license}>
		{version.licenseShort} · v{version.number}
	</span>
</div>

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		anchor={menu.anchor}
		items={menuItems}
		label="Identity profiles"
		onclose={() => (menu = null)}
	/>
{/if}

<style>
	.strip {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 0 10px;
		height: var(--strip-h);
		border-top: 1px solid color-mix(in srgb, var(--line) 60%, transparent);
		background-color: var(--chrome-veil);
	}

	.left {
		min-width: 0;
	}

	.note {
		font-size: var(--fs-secondary);
		color: var(--muted);
		white-space: nowrap;
	}

	/*
	 * The repository's own facts, centred between the identity and the licence.
	 *
	 * `min-width: 0` and the overflow rules matter more than they look: this is
	 * the one part of the strip that can be longer than the window, and it must
	 * give way rather than push the licence off the end — the GPL notice is the
	 * one thing on this strip that is not optional (FEAT-043).
	 */
	.repo {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Inside a group the parts are one sentence, so they keep the tight
	   dot-separated spacing the groups themselves do not. */
	.group {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}

	/* The inventory does not move while you read it, so it is the quieter of
	   the two and gives way first. */
	.counts {
		flex: none;
		opacity: 0.85;
		font-variant-numeric: tabular-nums;
	}

	/* The rule between the groups. Short, and the same ink as the strip's own
	   border, so it separates without becoming a third thing to read. */
	.divider {
		flex: none;
		width: 1px;
		height: 12px;
		background: color-mix(in srgb, var(--line) 70%, transparent);
	}

	.fact {
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.sep {
		opacity: 0.55;
	}

	.license {
		flex: none;
	}

	/* How far the walk has got. */
	.walk {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		flex: none;
		font-variant-numeric: tabular-nums;
	}

	/*
	 * The dot. Still while the walk is finished, breathing while it runs — the
	 * one piece of motion in the strip, and the only thing on it that says work
	 * is happening.
	 */
	.pulse {
		width: 6px;
		height: 6px;
		flex: none;
		border-radius: var(--r-pill);
		background: var(--ok);
	}

	.walk.running .pulse {
		background: var(--accent);
		animation: pulse-breathe 1.6s ease-in-out infinite;
	}

	@keyframes pulse-breathe {
		0%,
		100% {
			opacity: 0.35;
			transform: scale(0.82);
		}
		50% {
			opacity: 1;
			transform: scale(1.1);
		}
	}

	/*
	 * The counts group is what the rail's own rows already carry as badges, so
	 * it — and the rule in front of it — is the first thing to go when the strip
	 * runs out of room. What is left is the half that changes.
	 */
	@media (max-width: 1100px) {
		.tail {
			display: none;
		}
	}

	.profile-btn {
		background: transparent;
		border: none;
		color: var(--muted);
		font: inherit;
		font-size: var(--fs-secondary);
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 2px 4px;
		border-radius: 4px;
		cursor: pointer;
	}

	.profile-btn:hover {
		background: var(--soft);
		color: var(--ink);
	}

	.avatar-dot {
		font-size: var(--fs-mono);
	}

	.profile-text {
		max-width: 320px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
