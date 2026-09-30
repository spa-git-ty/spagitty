<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	interface Props {
		size?: number;
		class?: string;
	}

	let { size = 24, class: className = '' }: Props = $props();
</script>

<!-- Use the canonical brand source. Keeping a second inline copy allowed its
	 paths to drift until the mark rendered as an amber tile with one stray line. -->
<img
	class="brand-mark {className}"
	src="/favicon-64.png"
	alt=""
	width={size}
	height={size}
	draggable="false"
/>

<style>
	.brand-mark {
		display: inline-block;
		vertical-align: middle;
		flex: none;
		object-fit: contain;
	}
</style>
