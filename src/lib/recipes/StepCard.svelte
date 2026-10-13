<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import { blockOf, groupOf } from './catalogue';
	import { MAX_MESSAGE } from './model';
	import type { Finding, MergeHow, Message, PullMode, Step, World } from './types';

	/**
	 * One block in the stack (FEAT-133): its parameters as small fields, its
	 * git command beside it, and what the validator found about it under it.
	 *
	 * Everything that can be chosen is chosen from a list: a branch from the
	 * branch menu, a remote from the remotes, a check from the repository's
	 * own. What is typed is what git needs typed, a message and a new branch's
	 * name, and a commit to cherry-pick. *Agent writes* is offered only with
	 * the agent's help on.
	 */
	interface Props {
		index: number;
		step: Step;
		/** Its command, as it will be run. */
		command: string;
		findings: Finding[];
		selected: boolean;
		agentHelp: boolean;
		world: World | null;
		first: boolean;
		onchange: (step: Step) => void;
		onremove: () => void;
		onup: () => void;
		onselect: () => void;
		/** Open the branch menu under `anchor`, holding `value`. */
		onbranch: (anchor: HTMLElement, value: string, pick: (name: string) => void) => void;
	}

	let {
		index,
		step,
		command,
		findings,
		selected,
		agentHelp,
		world,
		first,
		onchange,
		onremove,
		onup,
		onselect,
		onbranch
	}: Props = $props();

	const block = $derived(blockOf(step.block));
	const group = $derived(groupOf(step.block));
	const remotes = $derived(world?.remotes ?? []);
	const checks = $derived(world?.checks ?? []);

	const MODES: { id: PullMode; label: string }[] = [
		{ id: 'fastForwardOnly', label: 'fast-forward only' },
		{ id: 'merge', label: 'merge' },
		{ id: 'rebase', label: 'rebase' }
	];
	const HOWS: { id: MergeHow; label: string }[] = [
		{ id: 'commit', label: 'merge commit' },
		{ id: 'fastForward', label: 'fast-forward if possible' },
		{ id: 'squash', label: 'squash' }
	];

	/** The step with some of its parameters changed. */
	function patch(params: Record<string, unknown>) {
		onchange({ ...step, params: { ...step.params, ...params } } as Step);
	}

	const message = $derived(
		step.block === 'commit' || step.block === 'squash' || step.block === 'reword' || step.block === 'amend'
			? step.params.message
			: undefined
	);
	const typed = $derived(message && message.kind === 'text' ? message.text : '');

	function setMessage(next: Message) {
		patch({ message: next });
	}

	function branchButton(event: MouseEvent, value: string, field: string) {
		onbranch(event.currentTarget as HTMLElement, value, (name) => patch({ [field]: name }));
	}
</script>

{#snippet branch(value: string, field: string, label: string)}
	<button
		class="pick mono"
		class:unset={!value}
		aria-label="{label}: {value || 'none chosen'}"
		aria-haspopup="listbox"
		onclick={(event) => branchButton(event, value, field)}
	>
		{value || 'choose a branch'}
		<span class="caret" aria-hidden="true">▾</span>
	</button>
{/snippet}

{#snippet messageField()}
	{#if message}
		<div class="message">
			{#if message.kind === 'agent'}
				<span class="agent-writes mono">the agent writes it when the step runs</span>
			{:else}
				<input
					class="mono text"
					type="text"
					aria-label="Step {index + 1} message"
					placeholder="message"
					spellcheck="false"
					autocomplete="off"
					value={typed}
					oninput={(event) => setMessage({ kind: 'text', text: event.currentTarget.value })}
				/>
				<span class="count mono" class:over={typed.length > MAX_MESSAGE}>{typed.length}/{MAX_MESSAGE}</span>
			{/if}
			<button
				class="toggle"
				class:on={message.kind === 'agent'}
				aria-pressed={message.kind === 'agent'}
				disabled={!agentHelp}
				title={agentHelp
					? 'Leave the message to the agent, written from the changes when the step runs'
					: 'Agent help is off'}
				onclick={() => setMessage(message.kind === 'agent' ? { kind: 'text', text: '' } : { kind: 'agent' })}
				>agent writes</button
			>
		</div>
	{/if}
{/snippet}

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="row" class:selected onclick={onselect}>
	<div class="card" data-step={index}>
		<div class="line">
			<span class="square" style:background-color={group.tone} aria-hidden="true"></span>
			<span class="number mono">{index + 1}</span>
			<span class="name">{block.label}</span>

			{#if step.block === 'stash'}
				<select
					aria-label="Step {index + 1}: what is stashed"
					value={step.params.untracked ? 'untracked' : 'tracked'}
					onchange={(event) => patch({ untracked: event.currentTarget.value === 'untracked' })}
				>
					<option value="untracked">+ untracked</option>
					<option value="tracked">tracked only</option>
				</select>
			{:else if step.block === 'pop'}
				<span class="note mono">on conflict: stop</span>
			{:else if step.block === 'commit'}
				<select
					aria-label="Step {index + 1}: what is committed"
					value={step.params.all ? 'all' : 'staged'}
					onchange={(event) => patch({ all: event.currentTarget.value === 'all' })}
				>
					<option value="all">all changes</option>
					<option value="staged">what is staged</option>
				</select>
			{:else if step.block === 'amend'}
				<select
					aria-label="Step {index + 1}: the message"
					value={step.params.message === null ? 'keep' : 'new'}
					onchange={(event) =>
						patch({ message: event.currentTarget.value === 'keep' ? null : { kind: 'text', text: '' } })}
				>
					<option value="keep">keep the message</option>
					<option value="new">new message</option>
				</select>
			{:else if step.block === 'fetch'}
				<select
					aria-label="Step {index + 1}: remote"
					value={step.params.remote}
					onchange={(event) => patch({ remote: event.currentTarget.value })}
				>
					<option value="">every remote</option>
					{#each remotes as remote (remote)}<option value={remote}>{remote}</option>{/each}
				</select>
			{:else if step.block === 'pull'}
				<select
					aria-label="Step {index + 1}: remote"
					value={step.params.remote}
					onchange={(event) => patch({ remote: event.currentTarget.value })}
				>
					<option value="">the upstream</option>
					{#each remotes as remote (remote)}<option value={remote}>{remote}</option>{/each}
				</select>
				<select
					aria-label="Step {index + 1}: how"
					value={step.params.mode}
					onchange={(event) => patch({ mode: event.currentTarget.value })}
				>
					{#each MODES as mode (mode.id)}<option value={mode.id}>{mode.label}</option>{/each}
				</select>
			{:else if step.block === 'push'}
				<span class="word">to</span>
				<select
					aria-label="Step {index + 1}: remote"
					value={step.params.remote}
					onchange={(event) => patch({ remote: event.currentTarget.value })}
				>
					{#if !remotes.includes(step.params.remote)}
						<option value={step.params.remote}>{step.params.remote || 'choose a remote'}</option>
					{/if}
					{#each remotes as remote (remote)}<option value={remote}>{remote}</option>{/each}
				</select>
				<span class="note mono" title="There is no way to force a push from a recipe">no force</span>
			{:else if step.block === 'merge'}
				{@render branch(step.params.branch, 'branch', `Step ${index + 1}: branch to merge`)}
				<span class="word">into this</span>
				<select
					aria-label="Step {index + 1}: how"
					value={step.params.how}
					onchange={(event) => patch({ how: event.currentTarget.value })}
				>
					{#each HOWS as how (how.id)}<option value={how.id}>{how.label}</option>{/each}
				</select>
			{:else if step.block === 'rebase'}
				<span class="word">onto</span>
				{@render branch(step.params.onto, 'onto', `Step ${index + 1}: branch to rebase onto`)}
			{:else if step.block === 'cherryPick'}
				<input
					class="mono text"
					type="text"
					aria-label="Step {index + 1}: commits"
					placeholder="commits, separated by spaces"
					spellcheck="false"
					autocomplete="off"
					value={step.params.commits.join(' ')}
					oninput={(event) => patch({ commits: event.currentTarget.value.split(/[\s,]+/).filter(Boolean) })}
				/>
			{:else if step.block === 'switch'}
				<span class="word">to</span>
				{@render branch(step.params.branch, 'branch', `Step ${index + 1}: branch to switch to`)}
			{:else if step.block === 'newBranch'}
				<input
					class="mono text"
					type="text"
					aria-label="Step {index + 1}: the new branch's name"
					placeholder="name"
					spellcheck="false"
					autocomplete="off"
					value={step.params.name}
					oninput={(event) => patch({ name: event.currentTarget.value.trim() })}
				/>
				<span class="word">from</span>
				<input
					class="mono text short"
					type="text"
					aria-label="Step {index + 1}: where it starts"
					placeholder="here"
					spellcheck="false"
					autocomplete="off"
					value={step.params.from}
					oninput={(event) => patch({ from: event.currentTarget.value.trim() })}
				/>
			{:else if step.block === 'deleteBranch'}
				{@render branch(step.params.branch, 'branch', `Step ${index + 1}: branch to delete`)}
				<span class="note mono" title="git branch -d: a branch that is not merged is refused">only if merged</span>
			{:else if step.block === 'squash'}
				<span class="word">the last</span>
				<input
					class="mono number-field"
					type="number"
					min="2"
					max="100"
					aria-label="Step {index + 1}: how many commits"
					value={step.params.count}
					oninput={(event) => patch({ count: Math.max(0, Math.floor(Number(event.currentTarget.value) || 0)) })}
				/>
				<span class="word">commits</span>
			{:else if step.block === 'check'}
				{#if checks.length === 0 && !step.params.command}
					<span class="note">This repository has no checks. Add them in Farm.</span>
				{:else}
					<select
						class="mono"
						aria-label="Step {index + 1}: check"
						value={step.params.command}
						onchange={(event) => patch({ command: event.currentTarget.value })}
					>
						{#if !checks.includes(step.params.command)}
							<option value={step.params.command}>{step.params.command || 'choose a check'}</option>
						{/if}
						{#each checks as check (check)}<option value={check}>{check}</option>{/each}
					</select>
				{/if}
			{/if}
		</div>

		{@render messageField()}
	</div>

	<span class="command mono" title="What this step runs">{command}</span>
	<span class="tools">
		<button aria-label="Move step {index + 1} up" title="Move up (Alt ↑)" disabled={first} onclick={onup}>
			<Icon name="chevron-up" size={14} />
		</button>
		<button aria-label="Remove step {index + 1}" title="Remove (Del)" onclick={onremove}>
			<Icon name="close" size={12} weight={2} />
		</button>
	</span>
</div>

{#each findings as finding (finding.code + finding.text)}
	<p class="finding" class:error={finding.severity === 'error'} role={finding.severity === 'error' ? 'alert' : 'note'}>
		<span class="mark" aria-hidden="true">{finding.severity === 'error' ? '!' : '·'}</span>
		{finding.text}
	</p>
{/each}

<style>
	.row {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.card {
		flex: 0 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 9px 12px;
		border: 1px solid var(--soft);
		border-radius: var(--r-button);
		background: var(--surface-2);
	}

	.row.selected .card {
		border-color: color-mix(in srgb, var(--accent) 62%, transparent);
		box-shadow: var(--shadow-1);
	}

	.line {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}

	.square {
		flex: none;
		width: 9px;
		height: 9px;
		border-radius: 2px;
		opacity: 0.75;
	}

	.number {
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.name {
		font-weight: 600;
		white-space: nowrap;
	}

	.word {
		white-space: nowrap;
	}

	/* The sides are left alone: a select keeps the room it has for its arrow. */
	select,
	.pick,
	.text,
	.number-field {
		padding-top: 3px;
		padding-bottom: 3px;
		font-size: var(--fs-mono);
	}

	.pick {
		padding-left: 8px;
		padding-right: 8px;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		border: 1px solid var(--soft);
		border-radius: var(--r-field);
		background: var(--sunken);
		white-space: nowrap;
	}

	.pick:hover {
		background: var(--hover);
	}

	.pick.unset {
		color: var(--muted);
	}

	.caret {
		color: var(--muted);
	}

	.text {
		width: 220px;
		min-width: 0;
	}

	.text.short {
		width: 110px;
	}

	.number-field {
		width: 58px;
	}

	.message {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.message .text {
		flex: 1;
		width: auto;
		min-width: 160px;
	}

	.count {
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.count.over {
		color: var(--danger);
	}

	.agent-writes {
		flex: 1;
		font-size: var(--fs-mono);
		color: var(--agent);
	}

	.toggle {
		padding: 3px 8px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		font-size: var(--fs-mono);
		white-space: nowrap;
	}

	.toggle.on {
		border-color: color-mix(in srgb, var(--agent) 62%, transparent);
		color: var(--agent);
	}

	.toggle:disabled {
		opacity: 0.45;
		cursor: default;
	}

	.command {
		flex: 1 1 0;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-mono);
		color: var(--muted);
	}

	.tools {
		flex: none;
		display: flex;
		gap: 2px;
	}

	.tools button {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		border-radius: 50%;
		color: var(--muted);
	}

	.tools button:hover:not(:disabled) {
		background: var(--hover);
		color: var(--ink);
	}

	.tools button:disabled {
		opacity: 0.3;
		cursor: default;
	}

	.finding {
		display: flex;
		gap: 8px;
		margin: 0 0 0 14px;
		font-size: var(--fs-secondary);
		color: var(--warn);
	}

	.finding.error {
		color: var(--danger);
	}

	.mark {
		flex: none;
		width: 10px;
		text-align: center;
		font-weight: 700;
	}
</style>
