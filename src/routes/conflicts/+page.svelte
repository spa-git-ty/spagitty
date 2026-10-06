<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Loader from '$lib/ui/Loader.svelte';
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { abortOperation, continueOperation } from '$lib/conflicts/actions';
	import { conflicts } from '$lib/conflicts/store.svelte';
	import { repo } from '$lib/repo.svelte';
	import Resolver from '$lib/resolver/Resolver.svelte';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * A repository stopped mid-operation (1D): what git stopped on by itself —
	 * a pull, a cherry-pick, a revert, a rebase from the graph. Merges started
	 * in Merger are resolved there; both use the same three-column resolver
	 * (FEAT-102).
	 *
	 * A is ours (`HEAD`), B is theirs, the side coming in. Every region is a
	 * choice on screen until *Mark resolved*, which writes the file and stages
	 * it. Continue is offered only once nothing is conflicted: git refuses it
	 * otherwise, and a button live for the whole time it cannot work reads as
	 * broken rather than guarded.
	 */

	onMount(() => {
		if (api.inTauri() && repo.info) void conflicts.load();
	});

	const names = $derived({ a: repo.info?.head.branch ?? 'ours', b: 'theirs' });
	const roles = { a: 'HEAD', b: 'coming in', result: 'what lands' };
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
			{#if conflicts.loading}<Loader size="inline" label="Reading…" />{/if}
			{#if conflicts.writeError}<span class="note error" role="alert">{conflicts.writeError}</span>{/if}
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

	<div class="body">
		{#if conflicts.error}
			<p class="note error pad">{conflicts.error}</p>
		{:else if !conflicts.loaded}
			<Loader label="Reading…" />
		{:else if conflicts.files.length === 0}
			<p class="note pad">
				{#if conflicts.operation === 'none'}
					Nothing is conflicted.
				{:else}
					Every file is resolved. Continue to finish the {conflicts.operationLabel}.
				{/if}
			</p>
		{:else}
			<Resolver
				files={conflicts.read}
				choices={conflicts.choices}
				{names}
				{roles}
				baseShort={null}
				onchoose={(path, index, choice) => conflicts.choose(path, index, choice)}
				onwhole={(path, side) => conflicts.whole(path, side)}
			>
				{#snippet fileActions(file)}
					<Btn
						primary
						busy={conflicts.busy}
						disabled={!conflicts.settleable(file.path)}
						title={conflicts.settleable(file.path) ? 'Write the result and stage it — git add' : 'Resolve every conflict in this file first'}
						onclick={() => conflicts.settle(file.path)}
					>
						{conflicts.asOnDisk(file.path) ? 'Mark resolved as it is' : 'Mark resolved'}
					</Btn>
				{/snippet}
			</Resolver>
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
		padding: 14px 18px 10px;
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

	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	.pad {
		padding: 10px 18px;
	}

	.error {
		color: var(--danger);
	}
</style>
