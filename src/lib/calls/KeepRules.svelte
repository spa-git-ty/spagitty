<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { KeepRule } from './types';

	/**
	 * A call's keep rules (FEAT-160, FEAT-161): each a variable, the JSONPath
	 * or XPath it is read from, and whether it is read again after every
	 * call. Clicking a value in a response adds one; here they are written
	 * and changed by hand.
	 */

	interface Props {
		rules: KeepRule[];
		onchange: (rules: KeepRule[]) => void;
		/** What a new rule's path starts with: `$.` for JSON, `//` for XML. */
		start?: string;
	}

	let { rules, onchange, start = '$.' }: Props = $props();

	function set(index: number, change: Partial<KeepRule>) {
		onchange(rules.map((rule, at) => (at === index ? { ...rule, ...change } : rule)));
	}
</script>

{#each rules as rule, index (index)}
	<div class="rule">
		<input
			class="mono name"
			type="text"
			aria-label="Variable"
			placeholder="name"
			spellcheck="false"
			value={rule.var}
			oninput={(event) => set(index, { var: event.currentTarget.value })}
		/>
		<span class="from">from</span>
		<input
			class="mono path"
			type="text"
			aria-label="Path in the response"
			placeholder={start === '$.' ? '$.data.token' : '//ns:Response/ns:id'}
			spellcheck="false"
			value={rule.from}
			oninput={(event) => set(index, { from: event.currentTarget.value })}
		/>
		<label class="every" title="Read again after every send; off keeps the first value">
			<input
				type="checkbox"
				checked={rule.every !== false}
				onchange={(event) => set(index, { every: event.currentTarget.checked })}
			/>
			every call
		</label>
		<button
			type="button"
			class="remove"
			aria-label="Remove this rule"
			title="Remove this rule"
			onclick={() => onchange(rules.filter((_, at) => at !== index))}>✕</button
		>
	</div>
{/each}
<button type="button" class="add" onclick={() => onchange([...rules, { var: '', from: start, every: true }])}
	>+ Keep a value</button
>
<p class="hint">A rule reads a JSONPath or XPath from the response into a variable, so a later call can use it.</p>

<style>
	.rule {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.name {
		width: 140px;
		padding: 5px 8px;
		color: var(--ok);
		font-size: var(--fs-mono);
	}

	.path {
		flex: 1;
		padding: 5px 8px;
		font-size: var(--fs-mono);
	}

	.from,
	.every,
	.hint {
		color: var(--muted);
		font-size: var(--fs-secondary);
		white-space: nowrap;
	}

	.hint {
		margin: 0;
		white-space: normal;
	}

	.every {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}

	.remove {
		padding: 2px 6px;
		border-radius: var(--r-button);
		color: var(--muted);
	}

	.remove:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.add {
		align-self: flex-start;
		padding: 3px 10px;
		border: 1px dashed var(--soft);
		border-radius: var(--r-button);
		color: var(--muted);
		font-size: var(--fs-secondary);
	}

	.add:hover {
		color: var(--ink);
	}
</style>
