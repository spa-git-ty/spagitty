<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import * as api from '$lib/api';
	import LogScreen from '$lib/search/LogScreen.svelte';
	import { search } from '$lib/search/store.svelte';

	/**
	 * Log (1I): find commits by author, path, message or date, and see who last
	 * touched each line. The screen is `LogScreen` (TASK-057); this is where it
	 * subscribes to the walk.
	 *
	 * Results stream as the walk finds them, so the first match appears long
	 * before the walk reaches the end of history.
	 */

	/** `Ctrl+F` lands here with `?focus=1`, which is what focuses the first field. */
	const focused = $derived(page.url.searchParams.get('focus') === '1');

	onMount(() => {
		if (!api.inTauri()) return;
		let detach: (() => void) | null = null;
		search.attach().then((off) => (detach = off));

		// Leaving cancels the walk. A query nobody is watching is a thread
		// reading history for no reason, and coming back to half a result set
		// from a question asked five minutes ago is worse than coming back
		// empty.
		return () => {
			detach?.();
			search.stop();
		};
	});

	/** `Alt+Enter` — its hunks, which are a different question and a different screen. */
	function openDiff(id: string) {
		search.select(id);
		goto(`/diff?commit=${id}`);
	}
</script>

<LogScreen autofocus={focused} ondiff={openDiff} />
