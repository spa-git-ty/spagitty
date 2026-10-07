<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Loader from '$lib/ui/Loader.svelte';
	import { hooks, MANAGER_LABEL } from './store.svelte';

	/**
	 * What hooks this repository has and what each runs (FEAT-107): the switch
	 * that turns them off for commits made here, who manages them, and every
	 * hook git would run, opened to its script — for Husky the script a person
	 * wrote, not the stub git calls; for lefthook and pre-commit, the
	 * configuration file that holds the steps. Shown in Settings → Hooks and
	 * from the commit bar.
	 */
	const info = $derived(hooks.info);
	let open = $state<Record<string, boolean>>({});

	onMount(() => {
		void hooks.load();
	});

	const commitHooks = $derived(info?.hooks.filter((hook) => hook.onCommit) ?? []);
	const otherHooks = $derived(info?.hooks.filter((hook) => !hook.onCommit) ?? []);
</script>

<div class="hooks">
	{#if hooks.error}
		<p class="note error">{hooks.error}</p>
	{:else if !info}
		{#if hooks.loading}<Loader label="Reading hooks…" />{:else}<p class="note">No repository is open.</p>{/if}
	{:else}
		<div class="switch-row">
			<button
				class="switch"
				role="switch"
				aria-checked={info.enabled}
				aria-label="Run hooks when committing here"
				onclick={() => hooks.setEnabled(!info.enabled)}
			>
				<span class="knob"></span>
			</button>
			<span class="switch-text">
				<span class="strong">Run hooks when committing here</span>
				<span class="note small">
					{info.enabled ? 'On' : 'Off — every commit in this repository skips them'} · kept in this clone's .git/config
				</span>
			</span>
			<span class="manager">{MANAGER_LABEL[info.manager]}</span>
		</div>

		<span class="note small dir">Hooks run from <span class="mono">{info.dir}</span></span>

		{#if info.hooks.length === 0}
			<div class="empty">
				<span class="empty-mark" aria-hidden="true"><Icon name="check" size="1.4em" weight={1.8} /></span>
				<span class="note">This repository has no hooks. Commits run nothing extra.</span>
			</div>
		{/if}

		{#each [{ title: 'On commit', list: commitHooks }, { title: 'Other moments', list: otherHooks }] as group (group.title)}
			{#if group.list.length > 0}
				<span class="label">{group.title}</span>
				<ul class="list">
					{#each group.list as hook (hook.name)}
						<li class="hook" class:off={!info.enabled && hook.onCommit}>
							<button
								class="hook-head"
								aria-expanded={open[hook.name] ?? false}
								onclick={() => (open[hook.name] = !open[hook.name])}
							>
								<Icon name={open[hook.name] ? 'chevron-down' : 'chevron-right'} size="0.9em" weight={2} />
								<span class="name mono">{hook.name}</span>
								<span class="path mono">{hook.path}</span>
								{#if hook.onCommit && !info.enabled}<span class="tag">skipped</span>{/if}
							</button>
							{#if open[hook.name]}
								<pre class="script mono">{hook.script}{#if hook.truncated}<span class="quiet">
… cut at 64 KiB</span>{/if}</pre>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		{/each}

		{#if info.config}
			<span class="label">The steps · <span class="mono">{info.config.path}</span></span>
			<pre class="script mono">{info.config.text}</pre>
		{/if}
	{/if}
</div>

<style>
	.hooks {
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 0;
	}

	.switch-row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		border-radius: 14px;
		background: color-mix(in srgb, var(--sunken) 60%, transparent);
	}

	.switch {
		width: 38px;
		height: 22px;
		flex: none;
		border-radius: var(--r-pill);
		background: var(--line);
		position: relative;
		transition: background 0.2s var(--ease);
	}

	.switch[aria-checked='true'] {
		background: var(--ok);
	}

	.knob {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--surface);
		box-shadow: var(--shadow-1);
		transition: transform 0.2s var(--ease);
	}

	.switch[aria-checked='true'] .knob {
		transform: translateX(16px);
	}

	.switch-text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.strong {
		font-weight: 600;
	}

	.small {
		font-size: var(--fs-mono);
	}

	.manager {
		flex: none;
		font-size: var(--fs-mono);
		padding: 2px 10px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
		color: var(--accent);
	}

	.label {
		margin-top: 4px;
		font-size: var(--fs-mono);
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 600;
	}

	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.hook {
		border-radius: 12px;
		border: 1px solid var(--pane-edge);
		background: color-mix(in srgb, var(--surface-2) 70%, transparent);
		overflow: hidden;
	}

	.hook.off {
		opacity: 0.6;
	}

	.hook-head {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 12px;
		text-align: left;
		min-width: 0;
	}

	.hook-head:hover {
		background: var(--hover);
	}

	.name {
		font-weight: 650;
		flex: none;
	}

	.path {
		flex: 1;
		min-width: 0;
		color: var(--muted);
		font-size: var(--fs-mono);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tag {
		font-size: var(--fs-mono);
		padding: 1px 8px;
		border-radius: var(--r-pill);
		border: 1px solid color-mix(in srgb, var(--danger) 45%, transparent);
		color: var(--danger);
	}

	.script {
		margin: 0;
		max-height: 320px;
		overflow: auto;
		padding: 10px 14px;
		background: var(--sunken);
		border-top: 1px solid var(--pane-edge);
		font-size: var(--fs-mono);
		line-height: 1.55;
		white-space: pre;
	}

	.hooks > .script {
		border: 1px solid var(--pane-edge);
		border-radius: 12px;
	}

	.quiet {
		color: var(--muted);
	}

	.empty {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 12px;
	}

	.empty-mark {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		color: var(--ok);
		background: color-mix(in srgb, var(--ok) 14%, transparent);
	}

	.error {
		color: var(--danger);
	}
</style>
