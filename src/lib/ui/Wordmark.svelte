<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	/**
	 * The name, set as the brand sets it (FEAT-085): "spagitty" in Sora
	 * SemiBold, with "git" in the brand's tomato — the lockups' wordmark, live.
	 */
	interface Props {
		/** Font size in px. */
		size?: number;
	}

	let { size = 15 }: Props = $props();
</script>

<span class="wordmark" style="font-size: {size}px">spa<span class="git">git</span>ty</span>

<style>
	.wordmark {
		font-family: 'Sora', system-ui, sans-serif;
		font-weight: 600;
		letter-spacing: -0.02em;
		line-height: 1;
		white-space: nowrap;
		color: var(--ink);
	}

	.git {
		color: var(--brand);
	}
</style>
