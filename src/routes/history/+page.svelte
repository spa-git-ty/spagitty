<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	/**
	 * File history became Investigate (FEAT-132), and this address goes there
	 * with the path it was given. A link written before the move — a bookmark,
	 * another screen's — still arrives at the file it named.
	 */
	onMount(() => {
		const path = page.url.searchParams.get('path');
		const query = path ? `?${new URLSearchParams({ kind: 'file', path })}` : '';
		void goto(`/investigate${query}`, { replaceState: true });
	});
</script>
