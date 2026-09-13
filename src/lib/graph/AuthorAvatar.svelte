<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	/**
	 * One person's mark, wherever a person is shown beside a commit (FEAT-081).
	 *
	 * The Author column and the commit detail each used to write this markup out
	 * for themselves, and they had drifted: one was `2em` — so it grew and shrank
	 * with the type scale — and the other a fixed 20px, while the node on the
	 * graph is 22px. Three sizes for one face, on the one screen whose whole point
	 * is that the same person reads as the same mark. One component means one
	 * size and one decision about which face to show.
	 *
	 * The size is the graph node's diameter at rest, published by `applyMetrics`
	 * as `--avatar-d` so it follows the zoom the way the node does. It is square
	 * by construction — fixed inline and block size, `aspect-ratio`, no flex
	 * shrink — because a flex row squeezed by a long name used to turn the circle
	 * into an ellipse. A deliberately larger treatment would be its own named
	 * variant; nothing needs one today.
	 *
	 * The face: the fetched picture once `avatars` has one, and until then the
	 * same generated initials on the same stable colour the canvas paints, so
	 * every surface changes from the placeholder to the picture together.
	 */
	import { avatars } from '$lib/graph/avatars.svelte';
	import { lettersOf, portraitBackground, seedOf } from '$lib/graph/portrait';

	interface Props {
		email: string | null | undefined;
		name: string;
		/** Letters to show, when the caller already has them (a graph row does). */
		letters?: string;
		title?: string;
	}

	let { email, name, letters, title }: Props = $props();

	const address = $derived(email ?? '');
	const face = $derived(avatars.drawable(address, name));
</script>

<span
	class="avatar"
	class:photo={face !== null}
	style="background: {face
		? `url(${face.src}) center / cover no-repeat`
		: portraitBackground(seedOf(address, name))}"
	{title}
	aria-hidden="true"
	>{#if !face}<span class="letters">{letters ?? lettersOf(name, address)}</span>{/if}</span
>

<style>
	/*
		Initials on a lane colour, matching the node, until a fetched picture
		covers the disc. `--bg` is the letter colour: it is the theme's ground,
		so it reads on a filled lane in both light and dark.
	*/
	.avatar {
		flex: none;
		box-sizing: border-box;
		inline-size: var(--avatar-d, 22px);
		block-size: var(--avatar-d, 22px);
		aspect-ratio: 1;
		border-radius: 50%;
		overflow: hidden;
		display: grid;
		place-items: center;
		line-height: 1;
		color: var(--bg);
		user-select: none;
		box-shadow: 0 0 0 1px var(--line);
	}

	.letters {
		/* Sized against the disc rather than the surrounding type, so the letters
		   fit the same circle on every surface. */
		font-size: calc(var(--avatar-d, 22px) * 0.4);
		font-weight: 600;
	}

	.avatar.photo {
		color: transparent;
	}
</style>
