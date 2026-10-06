<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import { notice } from '$lib/ui/notice.svelte';

	/**
	 * The result of the last operation, bottom-right, mounted by the shell.
	 *
	 * Named `NoticeToast` rather than `Notice` because of BUG-010: the store it
	 * reads is `notice.svelte.ts`, and on a case-insensitive filesystem —
	 * Windows, and macOS by default — `$lib/ui/notice.svelte` resolved to
	 * `Notice.svelte` instead. The component imported itself and the Windows
	 * build failed where the Linux one had always passed. A component and a
	 * rune store in one directory must not differ only by case.
	 *
	 * It is an ornament, glass like the toolbar and the pills (BUG-052): the
	 * kind of news is a filled circle in its colour with its mark — a tick, or
	 * an exclamation — rather than a coloured stripe down one edge, which read
	 * as a different design from everything around it.
	 */
	const current = $derived(notice.current);
</script>

{#if current}
	{#key current.id}
		<div class="notice ornament" class:error={current.tone === 'error'} role="status" aria-live="polite">
			<span class="mark" aria-hidden="true">
				{#if current.tone === 'error'}
					<span class="bang">!</span>
				{:else}
					<Icon name="check" size="0.95em" weight={2.6} />
				{/if}
			</span>
			<div class="text">
				<span class="title">{current.title}</span>
				{#if current.detail}<span class="note detail">{current.detail}</span>{/if}
			</div>
			<button class="close" aria-label="Dismiss" onclick={() => notice.dismiss()}>
				<Icon name="close" size="0.9em" weight={2} />
			</button>
		</div>
	{/key}
{/if}

<style>
	.notice {
		position: fixed;
		right: 16px;
		bottom: 16px;
		z-index: 55;
		max-width: min(440px, 60vw);
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 12px 12px 12px 14px;
		border-radius: 22px;
		/* It arrives from the corner it lives in. */
		animation: rise-in var(--t-enter-liquid) var(--spring-liquid);
	}

	.mark {
		flex: none;
		width: 26px;
		height: 26px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		background: var(--ok);
		color: var(--bg);
		box-shadow:
			0 0 0 4px color-mix(in srgb, var(--ok) 18%, transparent),
			0 2px 8px color-mix(in srgb, var(--ok) 35%, transparent);
	}

	.error .mark {
		background: var(--danger);
		box-shadow:
			0 0 0 4px color-mix(in srgb, var(--danger) 18%, transparent),
			0 2px 8px color-mix(in srgb, var(--danger) 35%, transparent);
	}

	.bang {
		font-weight: 800;
		font-size: var(--fs-secondary);
		line-height: 1;
	}

	.text {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
		padding-top: 3px;
	}

	.title {
		font-size: var(--fs-secondary);
		font-weight: 600;
	}

	/* Selectable and wrapped: git's message is often the thing worth copying. */
	.detail {
		white-space: pre-wrap;
		word-break: break-word;
		user-select: text;
	}

	.close {
		flex: none;
		width: 26px;
		height: 26px;
		display: grid;
		place-items: center;
		color: var(--muted);
		border-radius: 50%;
		transition:
			background var(--t-fast) var(--ease),
			color var(--t-fast) var(--ease);
	}

	.close:hover {
		color: var(--ink);
		background: var(--hover);
	}
</style>
