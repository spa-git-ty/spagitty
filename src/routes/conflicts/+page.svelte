<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import ConflictPager from '$lib/conflicts/ConflictPager.svelte';
	import ResolveBar from '$lib/conflicts/ResolveBar.svelte';
	import SidePane from '$lib/conflicts/SidePane.svelte';
	import { abortOperation, continueOperation } from '$lib/conflicts/actions';
	import { conflicts } from '$lib/conflicts/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * A repository stopped mid-operation: ours, the merged result, and theirs.
	 *
	 * It writes since FEAT-016. Three ways out of a file — a whole side, one
	 * marker region, or text typed into the merged pane — and one way out of the
	 * operation, which is Continue once nothing is conflicted any more.
	 *
	 * Continue is offered only when the list is empty. git refuses it otherwise
	 * and says so clearly, but a button that is live for the whole time it
	 * cannot work reads as broken rather than as guarded.
	 *
	 * There is no footer (BUG-014). It carried one sentence — that the screen
	 * only reads and resolving was not built — which stopped being true when
	 * FEAT-016 landed, and an escape-hatch command that Abort in the header now
	 * runs. Both were duplicated by the header, and a footer that repeats the
	 * header is what TASK-008 took off Branches.
	 */

	onMount(() => {
		if (api.inTauri() && repo.info) conflicts.load();
	});

	const kind = $derived(conflicts.sides?.kind ?? 'bothModified');
</script>

<div class="screen">
	<header class="head">
		<div class="left">
			<span class="title">Conflicts</span>
			{#if conflicts.operation !== 'none'}
				<span class="note">{conflicts.operationLabel} in progress</span>
			{/if}
			{#if conflicts.files.length > 0}
				<span class="note">
					{conflicts.files.length}
					{conflicts.files.length === 1 ? 'file' : 'files'}
				</span>
			{/if}
		</div>
		<div class="right">
			{#if conflicts.loading}<span class="note">Reading…</span>{/if}
			<Btn disabled={conflicts.busy} onclick={() => conflicts.load()}>Refresh</Btn>
			{#if conflicts.operation !== 'none'}
				<Btn
					primary
					disabled={conflicts.busy || conflicts.files.length > 0}
					title={conflicts.files.length > 0
						? 'Resolve every file first'
						: `Finish the ${conflicts.operationLabel}`}
					onclick={() => continueOperation(conflicts.operation)}
				>
					Continue
				</Btn>
				<Btn
					disabled={conflicts.busy}
					title="Abandon it and put the repository back"
					onclick={() => abortOperation(conflicts.operation)}
				>
					Abort {conflicts.operationLabel}
				</Btn>
			{/if}
		</div>
	</header>

	{#if conflicts.files.length > 0}
		<div class="bar">
			<ConflictPager />
		</div>
	{/if}

	<div class="body">
		{#if conflicts.error}
			<p class="note error">{conflicts.error}</p>
		{:else if !conflicts.loaded}
			<p class="note">Reading…</p>
		{:else if conflicts.files.length === 0}
			<div class="empty">
				{#if conflicts.operation === 'none'}
					<p class="note">Nothing is conflicted.</p>
				{:else}
					<p class="note">
						Every file is resolved. Continue to finish the {conflicts.operationLabel}.
					</p>
				{/if}
			</div>
		{:else if conflicts.sidesError}
			<p class="note error">{conflicts.sidesError}</p>
		{:else if !conflicts.sides}
			<p class="note">Reading…</p>
		{:else}
			<div class="path mono muted">{conflicts.sides.path}</div>
			<ResolveBar />
			<div class="panes">
				<SidePane
					title="Ours"
					subtitle="HEAD"
					side={conflicts.sides.ours}
					which="ours"
					{kind}
				/>
				<SidePane
					title="Merged result"
					subtitle="on disk, with markers"
					side={conflicts.sides.merged}
					which="merged"
					{kind}
					middle
				/>
				<SidePane
					title="Theirs"
					subtitle="the incoming side"
					side={conflicts.sides.theirs}
					which="theirs"
					{kind}
				/>
			</div>
			<details class="base">
				<summary class="note">Common ancestor</summary>
				<div class="basepane">
					<SidePane
						title="Base"
						subtitle="what both sides started from"
						side={conflicts.sides.base}
						which="base"
						{kind}
					/>
				</div>
			</details>
		{/if}
	</div>
</div>

<style>
	.screen {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 10px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
		flex: none;
	}

	.left,
	.right {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.title {
		font-size: var(--fs-title);
	}

	.bar {
		flex: none;
		padding: 8px 12px;
		background-color: var(--chrome-veil);
		border-bottom: 1px solid var(--band-rule, color-mix(in srgb, var(--line) 55%, transparent));
		box-shadow: none;
		position: relative;
		z-index: 1;
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.path {
		flex: none;
	}

	/* Three equal panes: the widths say the three sides are equally real, which
	   is the whole point of showing them together. */
	.panes {
		flex: 1;
		min-height: 240px;
		display: flex;
		gap: 8px;
		min-width: 0;
	}

	.base {
		flex: none;
	}

	.basepane {
		display: flex;
		height: 200px;
		padding-top: 8px;
	}

	.empty {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 520px;
	}
</style>
