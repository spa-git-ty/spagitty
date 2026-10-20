<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { mod } from '$lib/platform';
	import Loader from '$lib/ui/Loader.svelte';
	import type { FilesUsage, UsageRole } from '$lib/types';
	import FileGlyph from './FileGlyph.svelte';
	import { files } from './store.svelte';

	/**
	 * Where a name is defined and used, at the revision (FEAT-154).
	 *
	 * Three sections: where it is defined, where it is used, and where it is
	 * only mentioned — in a comment, a string, or a file of data or prose.
	 * Mentions are folded away at first; they are what makes a name search
	 * noisy. Each section lists its places by file, and a place opens the
	 * file there with the name marked.
	 *
	 * It is by name, not by type, and says so: two methods called `get` are
	 * one name here.
	 */
	interface Props {
		/** A file's right-click menu, as the tree has it. */
		oncontext?: (event: MouseEvent, path: string) => void;
	}

	let { oncontext }: Props = $props();

	const SECTIONS: { role: UsageRole; one: string; many: string }[] = [
		{ role: 'definition', one: 'definition', many: 'definitions' },
		{ role: 'use', one: 'use', many: 'uses' },
		{ role: 'mention', one: 'mention', many: 'mentions' }
	];

	let mentions = $state(false);
	/** A new name starts with its mentions folded again. */
	$effect(() => {
		void files.usagesOf;
		mentions = false;
	});

	const here = $derived(files.tab?.path ?? null);

	/** The places of one role, by file, in the order they came. */
	function grouped(role: UsageRole): { path: string; places: FilesUsage[] }[] {
		const out: { path: string; places: FilesUsage[] }[] = [];
		for (const usage of files.usages?.usages ?? []) {
			if (usage.role !== role) continue;
			const last = out.at(-1);
			if (last?.path === usage.path) last.places.push(usage);
			else out.push({ path: usage.path, places: [usage] });
		}
		return out;
	}

	const sections = $derived(
		SECTIONS.map((section) => {
			const groups = grouped(section.role);
			const count = groups.reduce((sum, group) => sum + group.places.length, 0);
			return { ...section, groups, count };
		})
	);

	/** `1 definition, 12 uses, 3 mentions`. */
	const counted = $derived(
		sections.map((section) => `${section.count} ${section.count === 1 ? section.one : section.many}`).join(', ')
	);

	function nameOf(path: string): { dir: string; name: string } {
		const cut = path.lastIndexOf('/');
		return cut === -1 ? { dir: '', name: path } : { dir: path.slice(0, cut + 1), name: path.slice(cut + 1) };
	}

	function sought(event: SubmitEvent) {
		event.preventDefault();
		const field = (event.currentTarget as HTMLFormElement).elements.namedItem('name');
		if (field instanceof HTMLInputElement) void files.findUsages(field.value);
	}
</script>

<form class="ask" onsubmit={sought}>
	<input
		type="text"
		name="name"
		aria-label="Find usages of a name"
		placeholder="A name, then Enter"
		spellcheck="false"
		autocomplete="off"
		value={files.usagesOf}
	/>
</form>

{#if files.usagesError}
	<span class="note said error" role="alert">{files.usagesError}</span>
{:else if files.usagesNow}
	<Loader size="inline" label="Finding usages…" />
{:else if !files.usages}
	<span class="note said">{mod()}click a name in the file to see where it is defined and used.</span>
{:else if files.usages.usages.length === 0}
	<span class="note said">“{files.usages.name}” is not in any file here</span>
{:else}
	<span class="note said" aria-live="polite">
		<span class="mono">{files.usages.name}</span> · {counted} in {files.usages.files}
		{files.usages.files === 1 ? 'file' : 'files'}{#if files.usages.more}, and more{/if}
	</span>
	<span class="note said small">By name, not by type: anything else called this is listed too.</span>

	{#each sections as section (section.role)}
		{#if section.count > 0}
			<div class="section" data-role={section.role}>
				{#if section.role === 'mention'}
					<button
						type="button"
						class="head"
						aria-expanded={mentions}
						onclick={() => (mentions = !mentions)}
					>
						<span class="arrow" aria-hidden="true">{mentions ? '▾' : '▸'}</span>
						Mentions <span class="note">{section.count}</span>
						<span class="note small">in comments, strings and documents</span>
					</button>
				{:else}
					<span class="head">
						{section.role === 'definition' ? 'Defined' : 'Used'}
						<span class="note">{section.count}</span>
					</span>
				{/if}
				{#if section.role !== 'mention' || mentions}
					{#each section.groups as group (group.path)}
						{@const parts = nameOf(group.path)}
						<div
							class="row file"
							title={group.path}
							role="presentation"
							oncontextmenu={(event) => oncontext?.(event, group.path)}
						>
							<FileGlyph path={group.path} />
							<span class="name">{parts.name}</span>
							<span class="dir note">{parts.dir}</span>
							<span class="count note">{group.places.length}</span>
						</div>
						{#each group.places as place (`${place.line}:${place.column}`)}
							<button
								class="row hit mono"
								class:here={group.path === here && files.match?.line === place.line - 1 && files.match?.start === place.column}
								title="{group.path}:{place.line}"
								onclick={() => files.openUsage(place)}
							>
								<span class="at note">{place.line}</span>
								<span class="text"
									>{place.text.slice(0, place.start)}<mark>{place.text.slice(place.start, place.end)}</mark
									>{place.text.slice(place.end)}</span
								>
								{#if place.kind}<span class="kind note">{place.kind}</span>{/if}
							</button>
						{/each}
					{/each}
				{/if}
			</div>
		{/if}
	{/each}
{/if}

<style>
	.ask {
		flex: none;
		display: flex;
	}

	.ask input {
		flex: 1;
		min-width: 0;
		height: 26px;
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
	}

	.said {
		padding: 6px;
		font-size: var(--fs-mono);
	}

	.small {
		font-size: calc(var(--fs-mono) * 0.92);
	}

	.error {
		color: var(--danger);
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: 1px;
		margin-top: 8px;
	}

	.head {
		display: flex;
		align-items: baseline;
		gap: 6px;
		padding: 4px 6px;
		font-size: var(--fs-secondary);
		font-weight: 600;
		text-align: left;
	}

	button.head:hover {
		background: var(--hover);
		border-radius: var(--r-field);
	}

	.head .note {
		font-weight: 400;
	}

	.arrow {
		width: 12px;
		color: var(--muted);
	}

	/* The rows are the tree's search results', so the two read alike. */
	.row {
		flex: none;
		display: flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 8px 0 6px;
		border-radius: var(--r-field);
		font-family: var(--font-mono);
		font-size: var(--fs-mono);
		text-align: left;
		white-space: nowrap;
	}

	button.row:hover {
		background: var(--hover);
	}

	.row.here {
		background: color-mix(in srgb, var(--accent) 18%, transparent);
		color: var(--accent);
		font-weight: 600;
	}

	.row.file {
		margin-top: 4px;
	}

	.name {
		flex: none;
		max-width: 64%;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dir {
		margin-left: auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		direction: rtl;
	}

	.count {
		flex: none;
		margin-left: 6px;
	}

	.row.hit {
		gap: 8px;
		padding-left: 12px;
	}

	.at {
		flex: none;
		min-width: 3ch;
		text-align: right;
	}

	.text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: pre;
	}

	.kind {
		flex: none;
		font-family: var(--font-ui);
		font-size: calc(var(--fs-mono) * 0.92);
	}

	mark {
		border-radius: 3px;
		background: color-mix(in srgb, var(--accent) 28%, transparent);
		color: inherit;
	}
</style>
