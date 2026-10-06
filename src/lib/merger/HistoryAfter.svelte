<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Strategy } from './plan';

	/**
	 * History after (FEAT-100): a small lane drawing of the receiving branch
	 * once the merge has landed, redrawn for each strategy. A sketch of the
	 * shape, not the graph: three commits a side, whatever the counts.
	 */
	interface Props {
		strategy: Strategy;
		/** The receiving branch's colour: A's, B's, or green for a new one. */
		target: 'a' | 'b' | 'new';
		source: 'a' | 'b';
		targetName: string;
		sourceName: string;
		baseShort: string;
	}

	let { strategy, target, source, targetName, sourceName, baseShort }: Props = $props();

	const tone = (side: 'a' | 'b' | 'new') =>
		side === 'a' ? 'var(--side-a)' : side === 'b' ? 'var(--side-b)' : 'var(--ok)';
	const t = $derived(tone(target));
	const s = $derived(tone(source));
	const label = $derived(
		`History of ${targetName} after the ${strategy === 'ff' ? 'fast-forward' : strategy}`
	);
</script>

<section class="card history" aria-label="History after">
	<span class="label">History after</span>
	<svg width="222" height="250" viewBox="0 0 222 250" role="img" aria-label={label}>
		{#if strategy === 'merge'}
			<g fill="none" stroke-width="2.5">
				<path d="M40 232V44" stroke={t}></path>
				<path d="M40 232C40 214 110 220 110 200V112C110 86 40 76 40 50" stroke={s}></path>
			</g>
			{#each [190, 160, 130] as y (y)}
				<circle cx="40" cy={y} r="5" fill={t}></circle>
				<circle cx="110" cy={y} r="5" fill={s}></circle>
			{/each}
			<circle cx="40" cy="40" r="9" fill="var(--bg)" stroke="var(--ok)" stroke-width="3"></circle>
			<text x="56" y="44" font-size="11" fill="var(--ok)">new merge commit</text>
		{:else if strategy === 'squash'}
			<g fill="none" stroke-width="2.5">
				<path d="M40 232V44" stroke={t}></path>
				<path d="M40 232C40 214 110 220 110 200V120" stroke={s} opacity=".45" stroke-dasharray="4 4"></path>
				<path d="M110 112C110 80 70 60 52 46" stroke="var(--ok)" stroke-width="1.5" stroke-dasharray="3 4"></path>
			</g>
			{#each [190, 160, 130] as y (y)}
				<circle cx="40" cy={y} r="5" fill={t}></circle>
				<circle cx="110" cy={y} r="5" fill={s} opacity=".45"></circle>
			{/each}
			<circle cx="40" cy="40" r="9" fill="var(--ok)"></circle>
			<text x="56" y="38" font-size="11" fill="var(--ok)">1 squashed commit</text>
		{:else if strategy === 'rebase'}
			<g fill="none" stroke-width="2.5">
				<path d="M40 232V30" stroke={t}></path>
				<path d="M40 232C40 214 130 220 130 200V140" stroke={s} opacity=".4" stroke-dasharray="4 4"></path>
			</g>
			{#each [205, 180, 155] as y (y)}
				<circle cx="40" cy={y} r="5" fill={t}></circle>
			{/each}
			{#each [120, 95, 70, 45] as y (y)}
				<circle cx="40" cy={y} r="6" fill="var(--bg)" stroke={s} stroke-width="2.5"></circle>
			{/each}
			<circle cx="130" cy="185" r="5" fill={s} opacity=".4"></circle>
			<circle cx="130" cy="160" r="5" fill={s} opacity=".4"></circle>
			<text x="56" y="86" font-size="11" fill="var(--ok)">replayed, new hashes</text>
			<text x="142" y="176" font-size="11" fill="var(--muted)">old copies</text>
		{:else}
			<g fill="none" stroke-width="2.5">
				<path d="M40 232V150" stroke={t}></path>
				<path d="M40 150V40" stroke={s}></path>
			</g>
			{#each [205, 180] as y (y)}
				<circle cx="40" cy={y} r="5" fill={t}></circle>
			{/each}
			{#each [130, 100, 70, 40] as y (y)}
				<circle cx="40" cy={y} r="5" fill={s}></circle>
			{/each}
			<text x="56" y="44" font-size="11" fill="var(--ok)">moves forward, no new commit</text>
		{/if}
		<circle cx="40" cy="232" r="5" fill="var(--bg)" stroke="var(--muted)" stroke-width="2"></circle>
		<text x="54" y="236" font-size="11" fill="var(--muted)">split · {baseShort}</text>
	</svg>
	<div class="legend mono">
		<span><span class="swatch" style:background={t}></span><span style:color={t}>{targetName}</span></span>
		<span><span class="swatch" style:background={s}></span><span class="muted">{sourceName}</span></span>
	</div>
</section>

<style>
	.history {
		flex: 0 0 250px;
		padding: 14px;
		display: flex;
		flex-direction: column;
		gap: 6px;
		border-radius: 16px;
	}

	svg {
		font-family: var(--font-mono);
	}

	.label {
		padding: 0 4px;
		font-size: var(--fs-mono);
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 600;
	}

	.legend {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: var(--fs-mono);
	}

	.legend > span {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}

	.swatch {
		width: 9px;
		height: 9px;
		border-radius: 50%;
		flex: none;
	}
</style>
