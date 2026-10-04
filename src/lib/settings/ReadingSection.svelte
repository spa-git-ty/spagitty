<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { pairWords } from '$lib/diff/words';
	import {
		CODE_FONTS,
		COLOURS,
		LH_MAX,
		LH_MIN,
		LH_STEP,
		LS_MAX,
		LS_MIN,
		LS_STEP,
		reading,
		RULERS,
		SIZE_MAX,
		SIZE_MIN,
		SIZE_STEP,
		UI_FONTS
	} from '$lib/reading.svelte';
	import type { DiffLine } from '$lib/types';
	import Chip from '$lib/ui/Chip.svelte';

	/**
	 * Settings › Reading (FEAT-090): how code, diffs and comments are set, with
	 * a preview that is drawn by the same tokens it changes.
	 */
	const now = $derived(reading.current);
	const codeNote = $derived(CODE_FONTS.find((font) => font.id === now.codeFont)?.note ?? '');
	const rulerNote = $derived(RULERS.find((ruler) => ruler.id === now.ruler)?.note ?? '');
	const colourNote = $derived(COLOURS.find((colour) => colour.id === now.colours)?.note ?? '');

	/** A few lines of a change, to read the choices against. */
	const SAMPLE: DiffLine[] = [
		{ origin: 'context', old: 13, new: 13, text: 'pub fn avatar(email: &str, size: u32) -> Result<Avatar> {' },
		{ origin: 'context', old: 14, new: 14, text: '    let key = cache_key(email, size);' },
		{ origin: 'removed', old: 15, new: null, text: '    if let Some(hit) = MEMORY.lock().get(&key) {' },
		{ origin: 'added', old: null, new: 15, text: '    let path = cache_dir()?.join(format!("{key}.png"));' },
		{ origin: 'added', old: null, new: 16, text: '    if path.exists() && !is_stale(&path, MAX_AGE)? {' },
		{ origin: 'added', old: null, new: 17, text: '        return Avatar::from_file(&path);' },
		{ origin: 'context', old: 16, new: 18, text: '    }' }
	];
	/** The line the preview's ruler sits on, and the chunk it belongs to. */
	const FOCUS = 4;
	const CHUNK = new Set([2, 3, 4, 5]);

	const marks = pairWords(SAMPLE);

	function sign(line: DiffLine): string {
		return line.origin === 'added' ? '+' : line.origin === 'removed' ? '−' : '';
	}
</script>

<section class="reading">
	<div class="controls">
		<h2 class="heading">Reading</h2>
		<span class="note lead">How code, diffs and comments are set on Review, Diff, Working copy and History.</span>

		<div class="row">
			<span class="note label">Code font</span>
			{#each CODE_FONTS as font (font.id)}
				<span class="face" style:font-family={font.stack}>
					<Chip active={now.codeFont === font.id} title={font.note} onclick={() => reading.set({ codeFont: font.id })}>
						{font.label}
					</Chip>
				</span>
			{/each}
		</div>
		{#if codeNote}<span class="note hint">{codeNote}</span>{/if}

		<div class="row">
			<span class="note label">Interface font</span>
			{#each UI_FONTS as font (font.id)}
				<span class="face" style:font-family={font.stack}>
					<Chip active={now.uiFont === font.id} onclick={() => reading.set({ uiFont: font.id })}>
						{font.label}
					</Chip>
				</span>
			{/each}
		</div>

		<div class="hr"></div>

		<label class="row">
			<span class="note label">Code size</span>
			<input
				class="slider"
				type="range"
				min={SIZE_MIN}
				max={SIZE_MAX}
				step={SIZE_STEP}
				value={now.size}
				oninput={(event) => reading.set({ size: Number(event.currentTarget.value) })}
			/>
			<span class="mono muted value">{now.size}px</span>
		</label>
		<label class="row">
			<span class="note label">Line spacing</span>
			<input
				class="slider"
				type="range"
				min={LH_MIN}
				max={LH_MAX}
				step={LH_STEP}
				value={now.lineHeight}
				oninput={(event) => reading.set({ lineHeight: Number(event.currentTarget.value) })}
			/>
			<span class="mono muted value">{now.lineHeight.toFixed(2)}</span>
		</label>
		<label class="row">
			<span class="note label">Letter spacing</span>
			<input
				class="slider"
				type="range"
				min={LS_MIN}
				max={LS_MAX}
				step={LS_STEP}
				value={now.letterSpacing}
				oninput={(event) => reading.set({ letterSpacing: Number(event.currentTarget.value) })}
			/>
			<span class="mono muted value">{now.letterSpacing.toFixed(2)}em</span>
		</label>

		<div class="hr"></div>

		<div class="row">
			<span class="note label">Focus ruler</span>
			{#each RULERS as ruler (ruler.id)}
				<Chip active={now.ruler === ruler.id} onclick={() => reading.set({ ruler: ruler.id })}>
					{ruler.label}
				</Chip>
			{/each}
		</div>
		<span class="note hint">{rulerNote} Move it with j and k.</span>

		<div class="row">
			<span class="note label">Diff colours</span>
			{#each COLOURS as colour (colour.id)}
				<Chip active={now.colours === colour.id} onclick={() => reading.set({ colours: colour.id })}>
					{colour.label}
				</Chip>
			{/each}
			<Chip active={now.words} onclick={() => reading.set({ words: !now.words })}>
				Highlight changed words
			</Chip>
		</div>
		<span class="note hint">{colourNote}</span>
	</div>

	<aside class="preview" aria-label="Preview">
		<span class="note">Preview</span>
		<div class="sample">
			<div class="sample-head mono">avatars.rs · @@ -13,8 +13,10 @@</div>
			{#each SAMPLE as line, index (index)}
				<div
					class="line {line.origin}"
					class:ruled={now.ruler !== 'off' && index === FOCUS}
					class:faded={now.ruler === 'chunk' && !CHUNK.has(index)}
				>
					<span class="num mono">{line.new ?? line.old}</span>
					<span class="sign">{sign(line)}</span>
					<span class="text"
						>{#each marks.get(index) ?? [{ text: line.text, changed: false }] as piece, p (p)}<span
								class:word={piece.changed}>{piece.text}</span
							>{/each}</span
					>
				</div>
			{/each}
		</div>
		<div class="comment">
			<span class="avatar" aria-hidden="true">NH</span>
			<span class="said">
				<span class="note"><span class="who">nour.h</span> · 2 h ago</span>
				<span class="words">A crash mid-write leaves half a PNG on disk. Write to a temp file, then rename?</span>
			</span>
		</div>
	</aside>
</section>

<style>
	.reading {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: 16px;
	}

	.controls {
		flex: 1 1 520px;
		max-width: 680px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.lead {
		line-height: 1.6;
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}

	.label {
		width: 110px;
		flex: none;
	}

	/* Each face's chip is set in that face, so the choice is seen, not read. */
	.face {
		display: contents;
	}

	.face :global(.chip) {
		font-family: inherit;
		font-size: var(--fs-secondary);
	}

	.hint {
		padding-left: 118px;
		font-size: var(--fs-mono);
	}

	.slider {
		flex: 1;
		min-width: 0;
		max-width: 260px;
		accent-color: var(--accent);
	}

	.value {
		width: 60px;
		flex: none;
	}

	.preview {
		flex: 0 1 420px;
		min-width: 300px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 14px;
		background: var(--surface);
		border: 1px solid var(--pane-edge);
		border-top-color: var(--glass-edge);
		border-radius: var(--r-floating);
	}

	.sample {
		overflow: hidden;
		background: var(--bg);
		border: 1px solid var(--pane-edge);
		border-radius: var(--r-panel);
	}

	.sample-head {
		padding: 5px 12px;
		color: var(--muted);
		background: var(--sunken);
	}

	.line {
		display: grid;
		grid-template-columns: 36px 18px minmax(0, 1fr);
		font-family: var(--code-font);
		font-size: var(--fs-code);
		line-height: var(--code-lh);
		letter-spacing: var(--code-ls);
		transition: opacity var(--t-fast) var(--ease);
	}

	.line.added {
		background: var(--diff-add-bg);
		box-shadow: inset var(--diff-marker) 0 0 var(--ok);
	}

	.line.removed {
		background: var(--diff-del-bg);
		box-shadow: inset var(--diff-marker) 0 0 var(--danger);
		color: var(--diff-del-ink);
	}

	.line.ruled {
		background: var(--ruler);
	}

	.line.faded {
		opacity: 0.42;
	}

	.num {
		text-align: right;
		padding-right: 6px;
		color: var(--muted);
		line-height: inherit;
	}

	.sign {
		text-align: center;
		font-weight: 700;
	}

	.line.added .sign {
		color: var(--ok);
	}

	.line.removed .sign {
		color: var(--danger);
	}

	.text {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		padding-right: 10px;
	}

	.line.added .word {
		background: var(--diff-add-hl);
		border-radius: 3px;
	}

	.line.removed .word {
		background: var(--diff-del-hl);
		border-radius: 3px;
	}

	.comment {
		display: flex;
		gap: 10px;
		padding: 12px 14px;
		border-radius: var(--r-floating);
		background: var(--surface-2);
		border: 1px solid var(--pane-edge);
	}

	.avatar {
		width: 26px;
		height: 26px;
		flex: none;
		display: grid;
		place-items: center;
		border-radius: 50%;
		background: var(--lane-4);
		color: var(--on-accent);
		font-size: 0.7em;
		font-weight: 700;
	}

	.said {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.who {
		color: var(--ink);
		font-weight: 600;
	}

	.words {
		font-family: var(--read-font);
		line-height: 1.65;
		letter-spacing: var(--code-ls);
	}
</style>
