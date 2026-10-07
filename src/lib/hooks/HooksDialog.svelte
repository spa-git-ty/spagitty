<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Btn from '$lib/ui/Btn.svelte';
	import Sheet from '$lib/ui/Sheet.svelte';
	import HooksView from './HooksView.svelte';

	/**
	 * The Hooks view in a window, opened from the commit bar (FEAT-107), so the
	 * hooks can be read where the commit is being written.
	 */
	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();
</script>

<Sheet labelledby="hooks-title" ondismiss={onclose}>
	<h2 class="title" id="hooks-title">Hooks</h2>
	<div class="body"><HooksView /></div>
	<div class="actions"><Btn primary onclick={onclose}>Done</Btn></div>
</Sheet>

<style>
	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: 600;
	}

	.body {
		min-height: 0;
		overflow: auto;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
	}
</style>
