<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { relativeTime } from '$lib/format';
	import type { MergerSide } from '$lib/types';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import BranchPicker from './BranchPicker.svelte';
	import { behindLine, cardRole, pullRefusal, sinceSplit, type Drift, type Roles, type SideKey } from './plan';
	import SideBadge from './SideBadge.svelte';
	import type { Pickable } from './store.svelte';

	/**
	 * One branch on Merger's stage (FEAT-100): its picker, how many commits it
	 * has since the two split, the newest three, and what it is in this merge —
	 * the one the result lands on is outlined in its colour.
	 *
	 * When the branch it tracks has commits it lacks, the card says so and
	 * offers *Pull now* (FEAT-126), whichever branch is checked out: a merge
	 * planned against a branch that is behind is planned against the wrong
	 * thing.
	 */
	interface Props {
		key: SideKey;
		name: string | null;
		side: MergerSide | null;
		baseShort: string;
		who: Roles | null;
		options: Pickable[];
		other: string | null;
		current: string | null;
		onpick: (name: string) => void;
		/** How far behind its upstream it is, when it is. */
		drift?: Drift | null;
		/** *Pull now* is running for this branch. */
		pulling?: boolean;
		/** Something else is running that a pull would collide with. */
		waiting?: string | null;
		onpull?: () => void;
	}

	let {
		key,
		name,
		side,
		baseShort,
		who,
		options,
		other,
		current,
		onpick,
		drift = null,
		pulling = false,
		waiting = null,
		onpull
	}: Props = $props();

	const role = $derived(who ? cardRole(key, who) : null);
	const more = $derived(side ? side.ahead - side.newest.length : 0);
	const refusal = $derived(name && drift ? pullRefusal(name, drift) : null);
</script>

<section
	class="card branch {key}"
	class:lands={role?.role === 'lands'}
	aria-label="Branch {key.toUpperCase()}"
>
	<div class="top">
		<SideBadge side={key} />
		<BranchPicker
			value={name}
			which={key.toUpperCase()}
			{options}
			{other}
			{current}
			{onpick}
		/>
		{#if role}
			<span class="role" class:strong={role.role === 'lands'}>{role.label}</span>
		{/if}
	</div>
	{#if name && drift}
		<div class="behind" role="status">
			<span class="behind-line" title="As of the last fetch">
				<Icon name="fetch" size="1em" weight={2} />{behindLine(drift)}
			</span>
			<Btn
				primary
				quiet
				busy={pulling}
				disabled={!onpull || refusal !== null || (waiting !== null && !pulling)}
				title={refusal ??
					waiting ??
					`Move ${name} up to ${drift.upstream}. Only forward: nothing is merged or rewritten.`}
				onclick={() => onpull?.()}
			>
				{pulling ? 'Pulling…' : 'Pull now'}
			</Btn>
		</div>
	{/if}
	{#if side}
		<span class="note">{sinceSplit(side, baseShort)}</span>
		<ul class="commits">
			{#each side.newest as commit (commit.id)}
				<li>
					<span class="dot" aria-hidden="true"></span>
					<span class="mono muted id">{commit.short}</span>
					<span class="summary" title={commit.summary}>{commit.summary}</span>
				</li>
			{/each}
		</ul>
		{#if more > 0}<span class="note small">+ {more} more</span>{/if}
		<span class="grow"></span>
		<span class="note small">last commit {relativeTime(side.time)}</span>
	{/if}
</section>

<style>
	.branch {
		flex: 0 1 286px;
		min-width: 220px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px 16px;
		border-radius: 18px;
	}

	.lands.a {
		border: 1.5px solid var(--side-a);
		background: color-mix(in srgb, var(--side-a) 7%, var(--surface));
	}

	.lands.b {
		border: 1.5px solid var(--side-b);
		background: color-mix(in srgb, var(--side-b) 7%, var(--surface));
	}

	/* The role wraps under the name rather than squeezing it. */
	.top {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.grow {
		flex: 1;
	}

	/* Its upstream has moved: said in the warning colour, with the way to
	   catch up beside it. Wraps under the line in a narrow card. */
	.behind {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 8px 10px;
		border-radius: 12px;
		background: color-mix(in srgb, var(--warn) 12%, transparent);
		border: 1px solid color-mix(in srgb, var(--warn) 35%, transparent);
	}

	.behind-line {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		font-size: var(--fs-secondary);
		color: var(--warn);
		overflow-wrap: anywhere;
	}

	.role {
		margin-left: auto;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 3px 10px;
		font-size: var(--fs-mono);
		white-space: nowrap;
		color: var(--muted);
		background-color: var(--surface-veil);
	}

	.a .role.strong {
		color: var(--side-a);
		border-color: var(--side-a);
		font-weight: 600;
	}

	.b .role.strong {
		color: var(--side-b);
		border-color: var(--side-b);
		font-weight: 600;
	}

	.commits {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 7px;
	}

	.commits li {
		display: flex;
		gap: 9px;
		align-items: baseline;
		min-width: 0;
		font-size: var(--fs-secondary);
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex: none;
		transform: translateY(-1px);
	}

	.a .dot {
		background: var(--side-a);
	}

	.b .dot {
		background: var(--side-b);
	}

	.id {
		flex: none;
		font-size: var(--fs-mono);
	}

	.summary {
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.small {
		font-size: var(--fs-mono);
	}
</style>
