<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Host, RefChip } from '$lib/types';

	interface Props {
		chip: RefChip;
	}

	let { chip }: Props = $props();

	/**
	 * Where the branch lives, said in glyphs (FEAT-036).
	 *
	 * `main` and `origin/main` are one branch in two places, so they are one
	 * chip carrying two marks rather than two chips repeating the name. The word
	 * `origin` never reaches the screen: it is longer than the glyph and answers
	 * a question nobody asked. What people want to know is whether the branch is
	 * only on their machine, and a mark answers that at a glance.
	 *
	 * The marks are hidden from assistive technology and the `title` carries the
	 * whole sentence instead, so nothing here is icon-only to anything that
	 * cannot see icons.
	 */
	const HOST_LABEL: Record<Host, string> = {
		gitHub: 'GitHub',
		gitLab: 'GitLab',
		bitbucket: 'Bitbucket',
		azureDevOps: 'Azure DevOps',
		generic: 'a remote'
	};

	const where = $derived.by(() => {
		if (chip.kind === 'tag') return chip.name;

		const parts: string[] = [];
		if (chip.local) parts.push('on this machine');
		for (const remote of chip.remotes) {
			const host = remote.host === 'generic' ? '' : ` (${HOST_LABEL[remote.host]})`;
			parts.push(`on ${remote.name}${host}`);
		}

		const said = parts.length === 0 ? chip.name : `${chip.name} — ${parts.join(', ')}`;
		// On `chip.divergence`, not on `drift`: a level branch draws nothing and
		// still says "level" when asked. Silence on the chip is about crowding;
		// the tooltip has room and the answer is worth having.
		return chip.divergence === null ? said : `${said}. ${drifted}`;
	});

	/**
	 * How far this branch has drifted from its upstream (FEAT-033).
	 *
	 * A level branch is `null` here rather than a pair of zeroes: the chip must
	 * say nothing, because `0/0` on every row is noise on every row, and the
	 * graph gutter is the most crowded place in the application.
	 */
	const drift = $derived.by(() => {
		const found = chip.divergence;
		if (!found || (found.ahead === 0 && found.behind === 0)) return null;
		return found;
	});

	/** The sentence the title carries, so the arrows are never the only telling. */
	const drifted = $derived.by(() => {
		const found = chip.divergence;
		if (!found) return '';
		if (found.ahead === 0 && found.behind === 0) return `Level with ${found.upstream}`;
		return `${found.ahead} ahead of and ${found.behind} behind ${found.upstream}, as of the last fetch`;
	});
</script>

<!--
	Tags take a right-notched radius and a dashed border so a tag is tellable
	from a branch at a glance, without a label or an icon. The current branch is
	the only chip that gets accent color and a check.
-->
<span
	class="ref"
	class:current={chip.current}
	class:tag={chip.kind === 'tag'}
	class:remote={chip.kind === 'remote'}
	title={where}
>
	{#if chip.current}<span aria-hidden="true">✔</span>{/if}<span class="name">{chip.name}</span>

	<!--
		The drift, when there is any (FEAT-033). Behind then ahead, the same
		order and the same two colours the Branches screen's bar uses, so the
		two places that show a divergence read the same way round.
	-->
	{#if drift}
		<span class="drift mono" aria-hidden="true">
			{#if drift.behind > 0}<span class="behind">↓{drift.behind}</span>{/if}
			{#if drift.ahead > 0}<span class="ahead">↑{drift.ahead}</span>{/if}
		</span>
	{/if}

	{#if chip.kind !== 'tag' && (chip.local || chip.remotes.length > 0)}
		<span class="marks" aria-hidden="true">
			{#if chip.local}
				<!-- A monitor: this branch exists on the machine in front of you. -->
				<svg class="mark" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
					<rect x="1.7" y="2.7" width="12.6" height="8.2" rx="1.2" />
					<path d="M5.7 13.3h4.6" stroke-linecap="round" />
				</svg>
			{/if}
			{#each chip.remotes as remote (remote.name)}
				{#if remote.host === 'gitHub'}
					<svg class="mark" viewBox="0 0 16 16" fill="currentColor">
						<path
							d="M8 .5a7.5 7.5 0 0 0-2.37 14.62c.37.07.5-.16.5-.36 0-.18-.006-.64-.01-1.26-2.09.45-2.53-1.01-2.53-1.01-.34-.87-.83-1.1-.83-1.1-.68-.47.05-.46.05-.46.75.05 1.15.77 1.15.77.67 1.15 1.76.82 2.19.63.07-.49.26-.82.48-1.01-1.67-.19-3.42-.83-3.42-3.71 0-.82.29-1.49.77-2.02-.08-.19-.34-.95.07-1.99 0 0 .63-.2 2.06.77a7.1 7.1 0 0 1 3.75 0c1.43-.97 2.06-.77 2.06-.77.41 1.04.15 1.8.07 1.99.48.53.77 1.2.77 2.02 0 2.89-1.76 3.52-3.43 3.7.27.23.51.69.51 1.39 0 1-.01 1.81-.01 2.06 0 .2.13.44.51.36A7.5 7.5 0 0 0 8 .5Z"
						/>
					</svg>
				{:else if remote.host === 'gitLab'}
					<svg class="mark" viewBox="0 0 16 16" fill="currentColor">
						<path
							d="m8 14.9-2.6-8h5.2l-2.6 8Zm-6.9-8h3.2l2.6 8-5.8-8Zm0 0L.3 9.4a.6.6 0 0 0 .22.67L8 14.9 1.1 6.9Zm0 0 1.1-3.4a.28.28 0 0 1 .53 0L4.3 6.9H1.1Zm13.8 0h-3.2l-2.6 8 5.8-8Zm0 0 .8 2.5a.6.6 0 0 1-.22.67L8 14.9l6.9-8Zm0 0-1.1-3.4a.28.28 0 0 0-.53 0L11.7 6.9h3.2Z"
						/>
					</svg>
				{:else if remote.host === 'bitbucket'}
					<svg class="mark" viewBox="0 0 16 16" fill="currentColor">
						<path
							d="M1.4 2a.62.62 0 0 0-.62.72l1.98 11.1a.62.62 0 0 0 .61.52h8.32a.5.5 0 0 0 .5-.42l1.98-11.2a.62.62 0 0 0-.62-.72H1.4Zm8.16 8.1H6.5L5.7 5.9h4.66l-.8 4.2Z"
						/>
					</svg>
				{:else if remote.host === 'azureDevOps'}
					<svg class="mark" viewBox="0 0 16 16" fill="currentColor">
						<path d="M15 3.7v8.5l-3.5 2.8-5.4-2v2L3 11.3l8.4.65V4.05L15 3.7Zm-3.6.4L4.7 1v2.6L1.4 4.6 1 11l2.9-.5V5.6l7.5-1.5Z" />
					</svg>
				{:else}
					<!-- A cloud: a remote whose host Spagitty does not recognise. -->
					<svg class="mark" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
						<path
							d="M4.5 12.1h6.8a2.85 2.85 0 0 0 .3-5.66A3.95 3.95 0 0 0 4.1 5.85 2.85 2.85 0 0 0 4.5 12.1Z"
							stroke-linejoin="round"
						/>
					</svg>
				{/if}
			{/each}
		</span>
	{/if}
</span>

<style>
	.ref {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		border: 1px solid var(--soft);
		border-radius: var(--r-field);
		padding: 0 6px;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		/* A label is a small raised object sitting on the row, so it gets the
		   raised surface and the light along its top edge. */
		background: var(--surface);
		box-shadow: var(--sheen);
		white-space: nowrap;
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		/*
		 * BUG-006. The three rules above promise a chip that ellipsises rather
		 * than pushing its neighbours out of the box, and without this one they
		 * do not keep that promise anywhere the chip is a flex item.
		 *
		 * A flex item's `min-width` defaults to `auto`, whose used value is the
		 * content's own width — so the chip refuses to shrink below a long
		 * branch name, `max-width: 100%` never gets to apply, and the overflow
		 * lands on top of whatever sits beside it. On the All repositories card
		 * that is the "N branches" count, which is what the overlap was.
		 */
		min-width: 0;
	}

	/* The name is what gives way. The marks are one character wide each, so
	   truncating them saves nothing and costs the whole point of the chip. */
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}

	.drift {
		display: inline-flex;
		align-items: baseline;
		gap: 3px;
		flex: none;
		font-size: var(--fs-mono);
	}

	/* The same two colours as the divergence bar on the Branches screen. Two
	   places showing one fact should not disagree about which way is which. */
	.behind {
		color: var(--lane-2);
	}

	.ahead {
		color: var(--lane-1);
	}

	.marks {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		flex: none;
	}

	/* Sized off the type rather than fixed, so the marks keep their proportion
	   to the name at every text size and zoom. Slightly dimmed: they qualify the
	   name rather than competing with it. */
	.mark {
		width: 1em;
		height: 1em;
		opacity: 0.75;
	}

	/* The branch you are on: the accent, as a tint under the name rather than
	   an outline around it, which is what makes it findable in a gutter of
	   twenty labels. */
	.ref.current {
		border-color: color-mix(in srgb, var(--accent) 55%, transparent);
		color: var(--accent);
		background: color-mix(in srgb, var(--accent) 15%, var(--surface));
		box-shadow: none;
	}

	/* Somebody else's copy: flat on the row, dimmed, no lift. */
	.ref.remote {
		color: var(--muted);
		background: none;
		box-shadow: none;
	}

	/*
	 * A tag keeps its right-notched shape — that is what tells it from a branch
	 * without a word or an icon — but is tinted amber instead of dashed. The
	 * dash was the wireframe's way of saying "a different kind of thing"; a
	 * colour says it at a glance and reads as deliberate.
	 */
	.ref.tag {
		border-radius: 2px 5px 5px 2px;
		border-color: color-mix(in srgb, var(--warn) 50%, transparent);
		color: var(--warn);
		background: color-mix(in srgb, var(--warn) 14%, var(--surface));
	}
</style>
