<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	/*
	 * Named `DialogHost` rather than `Dialog` because of BUG-010: the store it
	 * reads is `dialog.svelte.ts`, and on a case-insensitive filesystem
	 * `$lib/ui/dialog.svelte` resolved to this file instead. `tools/case.test.ts`
	 * refuses the shape now.
	 */
	import Btn from '$lib/ui/Btn.svelte';
	import { dialog } from '$lib/ui/dialog.svelte';

	/**
	 * The one confirmation and prompt dialog, mounted by the shell.
	 *
	 * Every destructive action routes through here, so the sentence explaining
	 * what is about to be lost is written in one style and cannot be forgotten
	 * by a screen that adds an action later.
	 */

	let field = $state<HTMLInputElement | null>(null);
	const question = $derived(dialog.question);

	$effect(() => {
		if (question?.kind === 'prompt') field?.select();
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			dialog.dismiss();
		} else if (event.key === 'Enter') {
			event.preventDefault();
			dialog.accept();
		}
	}
</script>

{#if question}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="backdrop"
		role="presentation"
		onclick={(event) => {
			if (event.target === event.currentTarget) dialog.dismiss();
		}}
	>
		<div
			class="panel"
			role="dialog"
			aria-modal="true"
			aria-labelledby="dialog-title"
			tabindex="-1"
			{onkeydown}
		>
			<h2 class="title" id="dialog-title">{question.title}</h2>
			<p class="body note">{question.body}</p>

			{#if question.kind === 'prompt'}
				<label class="field">
					<span class="note">{question.label}</span>
					<input
						bind:this={field}
						type="text"
						spellcheck="false"
						autocomplete="off"
						placeholder={question.placeholder ?? ''}
						value={dialog.draft}
						oninput={(event) => dialog.setDraft(event.currentTarget.value)}
					/>
				</label>
			{/if}

			<div class="actions">
				<Btn onclick={() => dialog.dismiss()}>Cancel</Btn>
				{#if question.kind === 'choice' && question.alternativeLabel}
					<Btn onclick={() => dialog.alternative()}>{question.alternativeLabel}</Btn>
				{/if}
				<!--
					`quiet` on a destructive confirmation: the travelling glow says
					"this is the thing to do next", and it is not — the thing to do
					next is to read the sentence above it.
				-->
				<Btn primary quiet={question.danger} disabled={dialog.blocked} onclick={() => dialog.accept()}>
					{question.confirmLabel}
				</Btn>
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		/* Darkened rather than tinted with the page colour: a dialog over a
		   light theme needs the room behind it to *recede*, and 60% of the
		   background over the background is barely a change at all. */
		background: color-mix(in srgb, var(--umbra) 40%, transparent);
		z-index: 60;
	}

	.panel {
		width: min(420px, 92vw);
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 16px;
		background-color: var(--glass-thick);
		backdrop-filter: var(--blur-thick);
		-webkit-backdrop-filter: var(--blur-thick);
		/* The pane's own edge, lit along the top (TASK-024). */
		border: var(--glass-edge-line);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
		box-shadow: var(--shadow-3);
		animation: rise-in var(--t-enter-liquid) var(--spring-liquid);
	}

	.title {
		margin: 0;
		font-size: var(--fs-title);
		font-weight: inherit;
	}

	.body {
		margin: 0;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	/* Shape, fill and focus ring come from the field rules in `app.css`; only
	   the size is this dialog's business. */
	.field input {
		font-size: var(--fs-ui);
		padding: 5px 8px;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 4px;
	}
</style>
