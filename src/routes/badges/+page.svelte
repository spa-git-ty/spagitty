<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import BadgeChip from '$lib/delight/BadgeChip.svelte';
	import { badge, CATEGORY_LABELS, titleable } from '$lib/delight/badges';
	import { card, markdown } from '$lib/delight/card';
	import { delight } from '$lib/delight/store.svelte';
	import { section, standings, summary, tally, unknown } from '$lib/delight/standings';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Chip from '$lib/ui/Chip.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * Badges (1P) — what has been earned here, by whom (FEAT-072).
	 *
	 * One screen rather than a panel on an existing one, because it answers a
	 * question nothing else on the rail answers: not "what is the state of this
	 * repository" but "what has been *done* in it, and who did it well". That is
	 * also why it survives having no repository open with a sentence rather than
	 * a blank — the answer "none of this has happened yet" is a real answer.
	 *
	 * The screen is per repository. A badge earned in one codebase says nothing
	 * about another, and an aggregate across all of them would flatter whoever
	 * has the most repositories.
	 */

	let selected = $state<string | null>(null);

	const actors = $derived(delight.list);
	/** The chosen actor, or the person at the keyboard, or the first with a record. */
	const record = $derived(
		(selected ? delight.get(selected) : null) ?? delight.me ?? actors[0] ?? null
	);
	const counts = $derived(record ? tally(record) : { earned: 0, known: 0 });
	const sections = $derived(
		record
			? CATEGORY_LABELS.filter(
					(entry) => entry.id !== 'chaos' || delight.showsShame
				).map((entry) => section(record, entry.id, entry.label))
			: []
	);
	const titles = $derived(record ? titleable(record.earned.map((entry) => entry.id)) : []);
	const board = $derived(standings(actors));
	const rows = $derived(record ? summary(record) : []);
	const strangers = $derived(record ? unknown(record) : []);
	/** The best non-shame badge, which is what the share card is cut from. */
	const showpiece = $derived(
		record
			? (badge(record.title ?? '') ??
					titleable(record.earned.map((entry) => entry.id)).at(-1) ??
					null)
			: null
	);

	async function copy(text: string, said: string): Promise<void> {
		try {
			await navigator.clipboard.writeText(text);
			notice.ok(said);
		} catch (error) {
			notice.failed('Could not copy that', error);
		}
	}

	async function forget(): Promise<void> {
		const agreed = await dialog.confirm({
			title: 'Forget this record',
			body:
				'Every badge, title and count earned in this repository is deleted. ' +
				'Nothing in git is touched — this is only the record of what was noticed.',
			confirmLabel: 'Forget',
			danger: true
		});
		if (!agreed) return;
		delight.forget();
		notice.ok('The record here was forgotten');
	}
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Badges</span>
			{#if record}
				<span class="note">{counts.earned} / {counts.known}<span class="more">+?</span></span>
			{/if}
		</div>
		<div class="right">
			{#if showpiece}
				<Btn onclick={() => copy(card(showpiece, record?.name ?? ''), 'Achievement card copied')}>
					Copy card
				</Btn>
			{/if}
			{#if record}
				<Btn onclick={() => copy(markdown(record), 'Markdown copied')}>Copy markdown</Btn>
			{/if}
		</div>
	</header>

	<div class="body">
		{#if repo.info === null}
			<div class="empty"><p class="note">No repository open.</p></div>
		{:else if actors.length === 0}
			<div class="empty">
				<p class="note">
					Nothing has been earned here yet. Badges arrive on their own, for work worth
					noticing — a conflict resolved, a rebase survived, a fix small enough to be
					worth pointing at.
				</p>
			</div>
		{:else if record}
			<!-- Who. Only drawn once there is more than one, which is the day an agent shows up. -->
			{#if actors.length > 1}
				<section class="block who">
					{#each actors as actor (actor.id)}
						<Chip
							active={actor.id === record.id}
							onclick={() => (selected = actor.id)}
							title="{actor.name} — {actor.kind}"
						>
							{actor.name}
						</Chip>
					{/each}
				</section>
			{/if}

			<!-- The title, which is the one thing on this screen somebody chooses. -->
			<section class="block">
				<h2 class="heading">Title</h2>
				<p class="note">
					One equipped badge, shown beside {record.name} wherever a name appears.
				</p>
				<div class="titles">
					<Chip active={record.title === null} onclick={() => delight.equip(record.id, null)}>
						none
					</Chip>
					{#each titles as found (found.id)}
						<Chip
							active={record.title === found.id}
							onclick={() => delight.equip(record.id, found.id)}
							title={found.line}
						>
							{found.emoji}
							{found.name}
						</Chip>
					{/each}
					{#if titles.length === 0}
						<span class="note">Nothing to equip yet.</span>
					{/if}
				</div>
			</section>

			{#each sections as group (group.category)}
				<section class="block">
					<h2 class="heading">
						{group.label}
						<span class="note">{group.earned} / {group.slots.length}</span>
					</h2>
					{#if group.category === 'chaos'}
						<p class="note">
							Not achievements. Things that happened, acknowledged. Nobody is being
							judged here.
						</p>
					{/if}
					<div class="grid">
						{#each group.slots as slot (slot.id)}
							<BadgeChip
								found={slot.found}
								locked={slot.locked}
								equipped={record.title === slot.id}
								onclick={slot.locked || !slot.found
									? undefined
									: () => delight.equip(record.id, slot.id)}
							/>
						{/each}
					</div>
				</section>
			{/each}

			{#if board.length > 0}
				<section class="block">
					<h2 class="heading">Agents, in this repository</h2>
					<p class="note">
						Measured here and nowhere else. This is not a claim about which model is
						better — it is what happened in this codebase.
					</p>
					<div class="table" role="table" aria-label="Agent standings">
						{#each board as row (row.id)}
							<div class="row" role="row">
								<span class="glyph" role="cell">{row.best?.emoji ?? '·'}</span>
								<span class="name" role="cell">{row.name}</span>
								<span class="note num" role="cell">{row.tasks} tasks</span>
								<span class="note num" role="cell">
									{row.approval === null ? '—' : `${row.approval}% first pass`}
								</span>
								<span class="note num" role="cell">
									{row.tests === null ? '—' : `${row.tests}% tests`}
								</span>
								<span class="note num" role="cell">{row.badges} badges</span>
							</div>
						{/each}
					</div>
				</section>
			{/if}

			<section class="block">
				<h2 class="heading">What has been counted</h2>
				<div class="stats">
					{#each rows as row (row.label)}
						<div class="stat">
							<span class="note">{row.label}</span>
							<span class="mono value">{row.value}</span>
						</div>
					{/each}
				</div>
				{#if strangers.length > 0}
					<p class="note">
						{strangers.length}
						{strangers.length === 1 ? 'badge was' : 'badges were'} earned by a newer
						Spagitty than this one. They are kept, not shown.
					</p>
				{/if}
			</section>

			<section class="block foot">
				<Btn onclick={forget}>Forget this record</Btn>
			</section>
		{/if}
	</div>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		position: relative;
		z-index: 1;
	}

	.left,
	.right {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}

	/* The `??` the design document asks for: a total that does not give the
	   secrets away by arithmetic. */
	.more {
		color: var(--muted);
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 12px;
		display: flex;
		flex-direction: column;
		gap: 18px;
	}

	.empty {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px 20px;
		text-align: center;
	}

	.empty .note {
		max-width: 52ch;
	}

	.block {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: 600;
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.who,
	.titles {
		flex-direction: row;
		flex-wrap: wrap;
		gap: 6px;
		display: flex;
		align-items: center;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
		gap: 8px;
	}

	.row {
		display: grid;
		grid-template-columns: 24px minmax(0, 1fr) repeat(4, minmax(0, 120px));
		align-items: center;
		gap: 10px;
		padding: 5px 4px;
		border-bottom: 1px solid var(--soft);
	}

	.num {
		text-align: right;
	}

	.stats {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 6px 16px;
	}

	.stat {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 10px;
		border-bottom: 1px solid var(--soft);
		padding: 3px 0;
	}

	.value {
		font-size: var(--fs-mono);
	}

	.foot {
		flex-direction: row;
		padding-top: 6px;
	}
</style>
