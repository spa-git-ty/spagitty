<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { repo } from '$lib/repo.svelte';
	import ReviewInbox from '$lib/review/ReviewInbox.svelte';
	import ReviewRoom from '$lib/review/ReviewRoom.svelte';
	import { review } from '$lib/review/store.svelte';

	/**
	 * Review (1R, FEAT-087): a place to read a pull request.
	 *
	 * The inbox, and the review room once one is opened. Pull requests (1H)
	 * stays the place to browse, create and merge them; this one is for
	 * reading a change closely enough to answer it.
	 *
	 * The list is read when a repository opens (the rail's dot needs it) and
	 * on Refresh — not on a timer, for the reason the Pull requests screen
	 * gives: polling spends somebody's rate limit while they are not looking.
	 */
	$effect(() => {
		if (repo.info !== null) review.prime(repo.generation);
	});
</script>

{#if review.room}
	<ReviewRoom />
{:else}
	<ReviewInbox />
{/if}
