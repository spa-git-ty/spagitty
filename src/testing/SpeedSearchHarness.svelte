<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Hit from '$lib/ui/Hit.svelte';
	import SpeedSearchBox from '$lib/ui/SpeedSearchBox.svelte';
	import { createSearch, speedSearch } from '$lib/ui/speedsearch.svelte';

	/** Two searchable lists, the second with its own filter field, and the box. */

	interface Props {
		names: string[];
		others?: string[];
		reserve?: string;
		onenter?: (name: string) => void;
	}

	let { names, others = [], reserve, onenter }: Props = $props();

	const search = createSearch();
	const shown = $derived(search.filter(names, (name) => name));

	let field = $state<HTMLInputElement | null>(null);
	let typed = $state('');
	const filtered = $derived(others.filter((name) => name.includes(typed)));
</script>

<ul
	class="first"
	use:speedSearch={{
		search,
		label: 'names',
		reserve,
		count: () => [shown.length, names.length],
		onenter: onenter && (() => shown[0] !== undefined && onenter(shown[0]))
	}}
>
	{#each shown as name (name)}
		<li><button class="row">{name}</button> <span class="drawn"><Hit text={name} {search} /></span></li>
	{/each}
</ul>

<div class="second" use:speedSearch={{ field: () => field, primary: false }}>
	<input bind:this={field} class="filter" bind:value={typed} />
	<button class="other">x</button>
	{#each filtered as name (name)}<span class="other-row">{name}</span>{/each}
</div>

<input class="elsewhere" />

<SpeedSearchBox />
