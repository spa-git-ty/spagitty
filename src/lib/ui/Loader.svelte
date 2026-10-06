<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	/**
	 * Something is being read or done (BUG-052): the brand's three strands,
	 * weaving under a small glass orb, in the palette's lane colours.
	 *
	 * Two sizes. `pane` sits in the middle of whatever is empty while it fills
	 * — a screen, a card — with the words under it. `inline` is the strands
	 * alone at text size, for a header or a button that is waiting. Under
	 * reduced motion the shell's rule stops the weave and the strands stay.
	 */
	interface Props {
		/** What is happening, for the eye and for a screen reader. */
		label?: string;
		size?: 'pane' | 'inline';
	}

	let { label = 'Reading…', size = 'pane' }: Props = $props();
</script>

{#if size === 'inline'}
	<span class="inline" role="status" aria-label={label} title={label}>
		<span class="strand s1"></span><span class="strand s2"></span><span class="strand s3"></span>
	</span>
{:else}
	<div class="pane" role="status" aria-live="polite">
		<div class="orb">
			<svg viewBox="0 0 64 40" width="64" height="40" aria-hidden="true">
				<path class="lane l1" d="M4 20 C 16 4, 28 4, 32 20 S 48 36, 60 20" />
				<path class="lane l2" d="M4 20 C 16 36, 28 36, 32 20 S 48 4, 60 20" />
				<path class="lane l3" d="M4 20 H 60" />
				<circle class="node n1" r="3.2" cx="4" cy="20" />
				<circle class="node n2" r="3.2" cx="4" cy="20" />
				<circle class="node n3" r="3.2" cx="4" cy="20" />
			</svg>
		</div>
		{#if label}<span class="label note">{label}</span>{/if}
	</div>
{/if}

<style>
	.pane {
		flex: 1;
		min-height: 120px;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding: 24px;
		animation: fade-in 0.3s var(--ease) both;
	}

	.orb {
		width: 92px;
		height: 92px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		background:
			radial-gradient(circle at 32% 26%, color-mix(in srgb, var(--glass-edge) 80%, transparent), transparent 55%),
			color-mix(in srgb, var(--surface-2) 70%, transparent);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		box-shadow: var(--ornament-shadow);
		animation: breathe 2.4s ease-in-out infinite;
	}

	.lane {
		fill: none;
		stroke-width: 3;
		stroke-linecap: round;
		stroke-dasharray: 64 200;
		animation: weave 1.8s ease-in-out infinite;
	}

	.l1 {
		stroke: var(--lane-1);
	}

	.l2 {
		stroke: var(--lane-5);
		animation-delay: -0.6s;
	}

	.l3 {
		stroke: var(--lane-2);
		opacity: 0.55;
		animation-delay: -1.2s;
	}

	.node {
		offset-rotate: 0deg;
		animation: travel 1.8s ease-in-out infinite;
	}

	.n1 {
		fill: var(--lane-1);
		offset-path: path('M4 20 C 16 4, 28 4, 32 20 S 48 36, 60 20');
	}

	.n2 {
		fill: var(--lane-5);
		offset-path: path('M4 20 C 16 36, 28 36, 32 20 S 48 4, 60 20');
		animation-delay: -0.6s;
	}

	.n3 {
		fill: var(--lane-2);
		offset-path: path('M4 20 H 60');
		animation-delay: -1.2s;
	}

	.label {
		font-size: var(--fs-secondary);
	}

	.inline {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		height: 1em;
		vertical-align: middle;
	}

	.strand {
		width: 4px;
		height: 0.9em;
		border-radius: 999px;
		animation: wave 1s ease-in-out infinite;
	}

	.s1 {
		background: var(--lane-1);
	}

	.s2 {
		background: var(--lane-5);
		animation-delay: 0.15s;
	}

	.s3 {
		background: var(--lane-2);
		animation-delay: 0.3s;
	}

	@keyframes weave {
		from {
			stroke-dashoffset: 64;
		}
		to {
			stroke-dashoffset: -200;
		}
	}

	@keyframes travel {
		0% {
			offset-distance: 0%;
			opacity: 0;
		}
		15%,
		85% {
			opacity: 1;
		}
		100% {
			offset-distance: 100%;
			opacity: 0;
		}
	}

	@keyframes breathe {
		0%,
		100% {
			transform: translateY(0) scale(1);
		}
		50% {
			transform: translateY(-3px) scale(1.03);
		}
	}

	@keyframes wave {
		0%,
		100% {
			transform: scaleY(0.45);
			opacity: 0.55;
		}
		50% {
			transform: scaleY(1);
			opacity: 1;
		}
	}

	@keyframes fade-in {
		from {
			opacity: 0;
		}
	}
</style>
