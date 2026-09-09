<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { clone } from './store.svelte';
	import Btn from '$lib/ui/Btn.svelte';

	/**
	 * Bring a repository in: an address, a folder, and the exact path it will
	 * land at shown before anything runs.
	 *
	 * Mounted by the layout rather than by a screen, because a clone survives
	 * navigation and a modal owned by a screen would not.
	 *
	 * The clone goes through the `git` binary, so credential helpers and the OS
	 * keychain work exactly as they do on the command line. Spagitty never asks
	 * for a password itself: a repository whose credentials no helper can supply
	 * fails with git's own message.
	 */
	const plan = $derived(clone.plan);
	const percent = $derived(clone.step?.percent ?? null);

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			clone.hide();
		}
	}
</script>

{#if clone.open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="scrim"
		role="dialog"
		aria-modal="true"
		aria-label="Clone a repository"
		tabindex="-1"
		onkeydown={keydown}
	>
		<div class="modal">
			<header class="head">
				<span class="title">Clone</span>
				<button class="close" aria-label="Close" onclick={() => clone.hide()}>✕</button>
			</header>

			<div class="body">
				<div class="field-row">
					<label class="label" for="clone-url">Address</label>
					<input
						id="clone-url"
						class="field"
						type="text"
						placeholder="https://example.com/owner/project.git"
						value={clone.url}
						disabled={clone.running}
						oninput={(event) => clone.setUrl(event.currentTarget.value)}
					/>
				</div>

				<div class="field-row">
					<span class="label">Into</span>
					<span class="path mono">{clone.parent || 'no folder chosen'}</span>
					<Btn disabled={clone.running} onclick={() => clone.chooseParent()}>Choose…</Btn>
				</div>

				<div class="field-row">
					<span class="label">Result</span>
					{#if plan.destination}
						<span class="path mono">{plan.destination}</span>
					{:else}
						<span class="note">nothing yet</span>
					{/if}
				</div>

				{#if plan.message}
					<p class="note">{plan.message}</p>
				{/if}

				{#if clone.running}
					<div class="progress">
						<div class="bar" aria-hidden="true">
							<div class="fill" style="width: {percent ?? 0}%"></div>
						</div>
						<span class="note">
							{#if clone.step}
								{clone.step.phase}{percent === null ? '' : ` · ${percent}%`}
							{:else}
								Starting…
							{/if}
						</span>
					</div>
					{#if clone.step && percent === null}
						<p class="note mono">{clone.step.line}</p>
					{/if}
				{/if}

				{#if clone.cloned}
					<p class="note">Cloned into <span class="mono">{clone.cloned}</span>.</p>
				{/if}

				{#if clone.error}
					<p class="note error">{clone.error}</p>
				{/if}
			</div>

			<footer class="foot">
				<span class="note">
					Cloning goes through <span class="mono">git</span>, so your credential helper and
					keychain work as they do on the command line. Spagitty never asks for a password
					itself.
				</span>
				<span class="spacer"></span>
				{#if clone.running}
					<Btn onclick={() => clone.cancel()}>Stop</Btn>
				{:else if clone.cloned}
					<Btn primary disabled={clone.busy} onclick={() => clone.openCloned()}>Open it</Btn>
				{:else}
					<Btn onclick={() => clone.hide()}>Cancel</Btn>
					<Btn primary disabled={!clone.runnable || clone.busy} onclick={() => clone.start()}>
						Clone
					</Btn>
				{/if}
			</footer>
		</div>
	</div>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		background: color-mix(in srgb, var(--umbra) 40%, transparent);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 20;
	}

	.modal {
		width: min(620px, calc(100vw - 40px));
		background: var(--glass-thick);
		backdrop-filter: var(--blur-thick);
		-webkit-backdrop-filter: var(--blur-thick);
		border: var(--glass-edge-line);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		box-shadow: var(--shadow-3);
		animation: rise-in var(--t-enter) var(--spring);
		display: flex;
		flex-direction: column;
		max-height: calc(100vh - 80px);
		overflow: hidden;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 12px;
		border-bottom: 1px solid var(--soft);
	}

	.title {
		font-size: var(--fs-title);
	}

	.close {
		color: var(--muted);
	}

	.close:hover {
		color: var(--accent);
	}

	.body {
		padding: 12px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		overflow: auto;
	}

	.field-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.label {
		width: 56px;
		flex: none;
		font-size: var(--fs-secondary);
		color: var(--muted);
	}

	.path {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.field {
		flex: 1;
		min-width: 0;
		background: transparent;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		padding: 3px 6px;
	}

	.field:focus {
		outline: none;
		border-color: var(--accent);
	}

	.field::placeholder {
		color: var(--placeholder);
	}

	.progress {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.bar {
		flex: 1;
		height: 6px;
		border-radius: 3px;
		background: var(--stripe);
		overflow: hidden;
	}

	.fill {
		height: 100%;
		background: var(--accent);
	}

	.foot {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border-top: 1px solid var(--soft);
	}

	.spacer {
		flex: 1;
	}

	.error {
		color: var(--danger);
	}

	p {
		margin: 0;
	}
</style>
