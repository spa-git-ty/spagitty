<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { tick, type Snippet } from 'svelte';
	import SideBadge from '$lib/merger/SideBadge.svelte';
	import ConflictCard from './ConflictCard.svelte';
	import { layout, type AgentProposal, type Choice, type Names, type ResolverFile, type SideKey } from './model';
	import { languageOf } from './paint';
	import ResolverPill from './ResolverPill.svelte';

	/**
	 * The three-column resolver (FEAT-102), shared by Merger and Conflicts.
	 *
	 * The conflicted files on the left, one dot per conflict — hollow red
	 * while unresolved, green once resolved — then the files that merge on
	 * their own, and the legend. The chosen file's conflicts in the middle as
	 * cards, and the pill over them. It holds no choices of its own: the
	 * screen does, and is told of each one.
	 */
	interface Props {
		files: ResolverFile[];
		choices: Record<string, (Choice | null)[]>;
		names: Names;
		/** What each column is: `lands here`, `comes in`, `lands in main`. */
		roles: { a: string; b: string; result: string };
		baseShort: string | null;
		/** Files that merge without help, with the side they come from. */
		others?: { path: string; tone: 'ok' | SideKey }[];
		/** The file to open on. */
		start?: string | null;
		onchoose: (path: string, index: number, choice: Choice | null) => void;
		onwhole: (path: string, side: SideKey) => void;
		/** More for the chosen file, beside All from A and All from B. */
		fileActions?: Snippet<[ResolverFile]>;
		/**
		 * An agent at work on these conflicts (2.0): its name for the legend,
		 * who chose what, what it proposes and where it is. All absent with no
		 * agent, and then nothing here mentions one.
		 */
		agent?: string | null;
		authors?: Record<string, ({ agent: string; decided: 'agent' | 'person' } | null)[]>;
		proposals?: Record<string, (AgentProposal | null)[]>;
		working?: { path: string; index: number } | null;
		onaccept?: (path: string, index: number) => void;
		onwhy?: (path: string, index: number) => void;
	}

	let {
		files,
		choices,
		names,
		roles,
		baseShort,
		others = [],
		start = null,
		onchoose,
		onwhole,
		fileActions,
		agent = null,
		authors = {},
		proposals = {},
		working = null,
		onaccept,
		onwhy
	}: Props = $props();

	let selected = $state<string | null>(null);
	let current = $state<{ path: string; index: number } | null>(null);
	let showBase = $state(false);
	let list = $state<HTMLElement | null>(null);

	const file = $derived(files.find((f) => f.path === selected) ?? files[0] ?? null);
	const places = $derived(file ? layout(file, choices[file.path] ?? []) : []);
	const language = $derived(file ? languageOf(file.path) : 'plain');

	/** Every conflict across every file, in order. */
	const all = $derived(files.flatMap((f) => f.regions.map((region) => ({ path: f.path, index: region.index }))));
	const position = $derived(
		current ? all.findIndex((c) => c.path === current!.path && c.index === current!.index) : -1
	);
	const unresolved = $derived(all.filter((c) => !choices[c.path]?.[c.index]));

	$effect(() => {
		// Opening on a file: the one asked for, or the first with something left.
		if (selected !== null && files.some((f) => f.path === selected)) return;
		const first = (start && files.find((f) => f.path === start)) || files.find((f) => f.regions.some((r) => !choices[f.path]?.[r.index])) || files[0];
		if (first) {
			selected = first.path;
			current = { path: first.path, index: first.regions[0]?.index ?? 0 };
		}
	});

	function base(path: string): string {
		return path.slice(path.lastIndexOf('/') + 1);
	}

	function split(path: string): { dir: string; name: string } {
		const cut = path.lastIndexOf('/') + 1;
		return { dir: path.slice(0, cut), name: path.slice(cut) };
	}

	function resolvedIn(f: ResolverFile): number {
		return f.regions.filter((r) => choices[f.path]?.[r.index]).length;
	}

	async function go(target: { path: string; index: number }) {
		selected = target.path;
		current = target;
		await tick();
		const card = list?.querySelector(`[id="conflict-${CSS.escape(target.path)}-${target.index}"]`);
		card?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
	}

	function step(by: number) {
		if (all.length === 0) return;
		const from = position < 0 ? 0 : position;
		void go(all[(from + by + all.length) % all.length]);
	}

	function nextUnresolved() {
		if (unresolved.length === 0) return;
		const from = position < 0 ? -1 : position;
		const after = unresolved.find((c) => all.indexOf(c) > from) ?? unresolved[0];
		void go(all.find((c) => c.path === after.path && c.index === after.index)!);
	}

	function take(side: SideKey) {
		if (!current) return;
		onchoose(current.path, current.index, { mode: side });
	}

	function numberOf(path: string, index: number): number {
		return all.findIndex((c) => c.path === path && c.index === index) + 1;
	}
</script>

<div class="resolver">
	<aside class="files" aria-label="Files">
		<span class="label">Conflicted files</span>
		{#each files as f (f.path)}
			{@const done = resolvedIn(f)}
			<button
				class="file"
				aria-pressed={file?.path === f.path}
				title={f.path}
				onclick={() => go({ path: f.path, index: f.regions[0]?.index ?? 0 })}
			>
				<span class="name mono">{base(f.path)}</span>
				{#if split(f.path).dir}<span class="dir mono">{split(f.path).dir}</span>{/if}
				<span class="dots">
					{#each f.regions as region (region.index)}
						{@const chosen = Boolean(choices[f.path]?.[region.index])}
						<span
							class="dot"
							class:resolved={chosen}
							class:proposed={!chosen && Boolean(proposals[f.path]?.[region.index])}
							class:working={!chosen && working?.path === f.path && working.index === region.index}
						></span>
					{/each}
					<span class="note small">{done === f.regions.length ? 'all resolved' : `${done} of ${f.regions.length}`}</span>
				</span>
			</button>
		{/each}
		{#if others.length > 0}
			<span class="label later">Merge on their own</span>
			{#each others as other (other.path)}
				<span class="other" title={other.path}>
					<span class="pip tone-{other.tone}"></span>
					<span class="mono muted">{base(other.path)}</span>
				</span>
			{/each}
		{/if}
		<span class="grow"></span>
		<div class="legend note small">
			<span><SideBadge side="a" />{names.a}</span>
			<span><SideBadge side="b" />{names.b}</span>
			<span><SideBadge side="mine" />Typed by you</span>
			{#if agent}<span><SideBadge side="agent" />{agent}</span>{/if}
		</div>
	</aside>

	<section class="conflicts" aria-label="Conflicts">
		{#if file}
			{@const parts = split(file.path)}
			<div class="file-head">
				<span class="mono path"><span class="muted">{parts.dir}</span>{parts.name}</span>
				<span class="note small">{file.regions.length} {file.regions.length === 1 ? 'conflict' : 'conflicts'}</span>
				<span class="grow"></span>
				{@render fileActions?.(file)}
				<span class="note small">Whole file:</span>
				<button class="chip" onclick={() => onwhole(file.path, 'a')}><SideBadge side="a" />All from {names.a}</button>
				<button class="chip" onclick={() => onwhole(file.path, 'b')}><SideBadge side="b" />All from {names.b}</button>
			</div>
			<div class="column-heads">
				<span class="column-head"><SideBadge side="a" />{names.a}<span class="note">· {roles.a}</span></span>
				<span class="column-head center">Result<span class="note">· {roles.result}</span></span>
				<span class="column-head end"><span class="note">{roles.b} ·</span>{names.b}<SideBadge side="b" /></span>
			</div>
			<div class="list" bind:this={list}>
				{#each file.regions as region, k (region.index)}
					<ConflictCard
						{file}
						{region}
						place={places[k]}
						choice={choices[file.path]?.[region.index] ?? null}
						number={numberOf(file.path, region.index)}
						{names}
						{language}
						{showBase}
						{baseShort}
						current={current?.path === file.path && current.index === region.index}
						onchoose={(choice) => {
							current = { path: file.path, index: region.index };
							onchoose(file.path, region.index, choice);
						}}
						onfocus={() => (current = { path: file.path, index: region.index })}
						author={authors[file.path]?.[region.index] ?? null}
						proposal={proposals[file.path]?.[region.index] ?? null}
						onaccept={onaccept ? () => onaccept(file.path, region.index) : undefined}
						onwhy={onwhy ? () => onwhy(file.path, region.index) : undefined}
					/>
				{/each}
			</div>
			<ResolverPill
				position={Math.max(position, 0) + 1}
				total={all.length}
				file={current ? base(current.path) : ''}
				{showBase}
				done={unresolved.length === 0}
				canTake={current !== null}
				{names}
				onstep={step}
				onbase={() => (showBase = !showBase)}
				ontake={take}
				onnext={nextUnresolved}
			/>
		{:else}
			<p class="note">Nothing is in conflict.</p>
		{/if}
	</section>
</div>

<style>
	.resolver {
		flex: 1;
		min-height: 0;
		display: flex;
		gap: 8px;
		padding: 0 10px;
	}

	.files {
		width: 250px;
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-height: 0;
		overflow: auto;
		padding-bottom: 12px;
	}

	.label {
		padding: 4px 10px 6px;
		font-size: var(--fs-mono);
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 600;
	}

	.label.later {
		padding-top: 16px;
	}

	.file {
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 9px 10px;
		border-radius: 12px;
	}

	.file:hover {
		background: var(--hover);
	}

	.file[aria-pressed='true'] {
		background: color-mix(in srgb, var(--accent) 14%, transparent);
	}

	.name {
		font-size: var(--fs-secondary);
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* The folder, so two files of one name are told apart. */
	.dir {
		font-size: var(--fs-mono);
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		direction: rtl;
		text-align: left;
	}

	.dots {
		display: flex;
		align-items: center;
		gap: 5px;
		flex-wrap: wrap;
	}

	.dot {
		width: 9px;
		height: 9px;
		border-radius: 50%;
		box-sizing: border-box;
		border: 1.5px solid var(--danger);
	}

	.dot.resolved {
		border: none;
		background: var(--ok);
	}

	/* An agent at this conflict, and one it has proposed for (2.0). Green
	   still means resolved. */
	.dot.working {
		border-color: var(--agent);
	}

	.dot.proposed {
		position: relative;
		border-color: var(--agent);
		overflow: hidden;
	}

	/* Half filled: proposed, not yet decided. */
	.dot.proposed::after {
		content: '';
		position: absolute;
		inset: 0 50% 0 0;
		background: var(--agent);
	}

	.dots .note {
		margin-left: 4px;
	}

	.other {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 10px;
		font-size: var(--fs-mono);
		min-width: 0;
	}

	.other .mono {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.pip {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex: none;
	}

	.tone-ok {
		background: var(--ok);
	}

	.tone-a {
		background: var(--side-a);
	}

	.tone-b {
		background: var(--side-b);
	}

	.legend {
		padding: 14px 10px 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.legend span {
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.grow {
		flex: 1;
	}

	.conflicts {
		flex: 1;
		min-width: 0;
		position: relative;
		display: flex;
		flex-direction: column;
	}

	.file-head {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
		padding: 4px 8px 10px;
	}

	.path {
		font-size: var(--fs-secondary);
	}

	.chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		border: 1px solid var(--soft);
		border-radius: var(--r-pill);
		padding: 3px 10px 3px 4px;
		font-size: var(--fs-mono);
		white-space: nowrap;
		background-color: var(--surface-veil);
	}

	.chip:hover {
		background-color: var(--hover);
	}

	.column-heads {
		flex: none;
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr) minmax(0, 1fr);
		gap: 10px;
		padding: 0 21px 8px;
	}

	.column-head {
		display: flex;
		align-items: center;
		gap: 7px;
		font-size: var(--fs-secondary);
		font-weight: 600;
		min-width: 0;
	}

	.column-head .note {
		font-weight: 400;
	}

	.center {
		justify-content: center;
	}

	.end {
		justify-content: flex-end;
	}

	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 0 8px 100px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.small {
		font-size: var(--fs-mono);
	}
</style>
