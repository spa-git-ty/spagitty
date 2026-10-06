<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * The frame the extension dialogs share: the shell dialog's backdrop and
	 * glass, wider, with focus taken on open and Escape to close.
	 */
	interface Props {
		title: string;
		onclose: () => void;
		wide?: boolean;
		children: Snippet;
		actions: Snippet;
	}

	let { title, onclose, wide = false, children, actions }: Props = $props();
	let panel = $state<HTMLDivElement | null>(null);
	const id = `modal-${Math.random().toString(36).slice(2)}`;

	$effect(() => {
		panel?.focus();
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			onclose();
		}
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	class="backdrop"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) onclose();
	}}
>
	<div
		class="panel"
		class:wide
		role="dialog"
		aria-modal="true"
		aria-labelledby={id}
		tabindex="-1"
		bind:this={panel}
		{onkeydown}
	>
		<h2 class="title" {id}>{title}</h2>
		<div class="content">{@render children()}</div>
		<div class="actions">{@render actions()}</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: color-mix(in srgb, var(--umbra) 40%, transparent);
		z-index: 60;
	}

	.panel {
		width: min(480px, 92vw);
		max-height: 86vh;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 16px;
		background-color: var(--glass-thick);
		backdrop-filter: var(--blur-thick);
		-webkit-backdrop-filter: var(--blur-thick);
		border: var(--glass-edge-line);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		box-shadow: var(--shadow-3);
		animation: rise-in var(--t-enter-liquid) var(--spring-liquid);
	}

	.panel.wide {
		width: min(680px, 94vw);
	}

	.panel:focus-visible {
		outline: 2px solid var(--ring);
		outline-offset: 2px;
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: inherit;
	}

	.content {
		display: flex;
		flex-direction: column;
		gap: 8px;
		overflow: auto;
		min-height: 0;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 4px;
	}
</style>
