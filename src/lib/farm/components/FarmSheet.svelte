<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import Sheet from '$lib/ui/Sheet.svelte';

	/**
	 * A Farm window over the screen: the task editor, the Rules, a new CLI
	 * agent (FEAT-109).
	 *
	 * The material is the shared `Sheet`. What this adds is the keyboard: focus
	 * moves into the window when it opens, Tab stays inside it, and focus goes
	 * back to whatever opened it when it closes.
	 */
	interface Props {
		title: string;
		onclose: () => void;
		children: Snippet;
	}

	let { title, onclose, children }: Props = $props();

	const heading = `farm-sheet-${Math.random().toString(36).slice(2)}`;
	let body: HTMLDivElement;

	const FOCUSABLE =
		'button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]';

	onMount(() => {
		const previous = document.activeElement as HTMLElement | null;
		body.querySelector<HTMLElement>('input,select,textarea,button')?.focus();
		return () => previous?.focus();
	});

	/** Keep Tab inside the window. */
	function trap(event: KeyboardEvent) {
		if (event.key !== 'Tab') return;
		const controls = Array.from(body.querySelectorAll<HTMLElement>(FOCUSABLE));
		const first = controls[0];
		const last = controls.at(-1);
		if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last?.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first?.focus();
		}
	}
</script>

<Sheet labelledby={heading} width="600px" ondismiss={onclose} onkeydown={trap}>
	<h2 id={heading} class="title">{title}</h2>
	<div class="body" bind:this={body}>
		{@render children()}
	</div>
</Sheet>

<style>
	.title {
		font-size: var(--fs-title);
		font-weight: 600;
		margin: 0;
	}

	.body {
		display: flex;
		flex-direction: column;
		gap: 12px;
		min-height: 0;
		overflow: auto;
		font-size: var(--fs-ui);
	}
</style>
