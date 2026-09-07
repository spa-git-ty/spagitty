<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import Chip from '$lib/ui/Chip.svelte';
	import { settings, type BooleanSetting } from './store.svelte';

	/**
	 * Spagitty's own preferences, stored in its config directory rather than in
	 * `.git/config` — none of them is a fact about a repository.
	 *
	 * **A toggle that does nothing yet says so.** Each one below persists, and
	 * each names the work item that will make it take effect. Narrowing the
	 * claim to the truth is the honest half of this screen: a switch that
	 * silently does nothing is worse than one that says it is waiting.
	 *
	 * "Sign my commits" is no longer here. It was the one toggle nothing read,
	 * and FEAT-019 answered it by moving the preference rather than wiring it
	 * up: `commit.gpgsign` is the same switch in the place every other tool
	 * looks, so it lives under **You** with the identity and is written with
	 * `git config`.
	 *
	 * # The label is the setting; `what` is for hovering (TASK-038)
	 *
	 * Every row used to print its `what` under its label, so three switches
	 * filled the screen with six lines of prose and the reader had to find the
	 * three that were the actual controls. A label that needs a paragraph under
	 * it is a label that has not been written yet, so the labels were rewritten
	 * to stand alone and the paragraph moved to the chip's `title`. Nothing was
	 * deleted — it is one hover away, where a person who wants the detail can
	 * ask for it and a person who does not is not made to read it.
	 */
	const TOGGLES: { key: BooleanSetting; label: string; what: string; pending: string | null }[] = [
		{
			key: 'confirmHistoryRewrite',
			label: 'Ask before rewriting history',
			what: 'Confirms before anything that changes commits that already exist.',
			pending: 'nothing in this build rewrites history'
		},
		{
			key: 'showGitCommands',
			label: 'Show the git command behind each action',
			// Not "the equivalent command line": what is shown is the command that
			// ran, recorded where it was spawned. Reads never run one, and the
			// panel says so rather than inventing one.
			what: 'Adds a Commands panel listing every git command Spagitty executes.',
			pending: null
		},
		{
			key: 'fetchAvatars',
			label: "Show authors' real pictures on the graph",
			// FEAT-079. The label says what is gained; the hover says what is
			// sent, because that is the part somebody turning this off is
			// deciding about.
			what: 'Fetches each author\'s picture once, from the address in their commits. Off draws the generated face instead and empties the cache.',
			pending: null
		},
		{
			key: 'pruneOnFetch',
			// The label carries the reassurance that used to be a sentence
			// under it: what a prune deletes is a remote-tracking ref, never a
			// local branch and never a commit (FEAT-018).
			label: 'Prune deleted remote branches when fetching',
			what: 'Deletes remote-tracking refs for branches the remote no longer has. Your own local branches are untouched.',
			pending: null
		}
	];
</script>

<section class="section">
	<header>
		<h2 class="heading">Behaviour</h2>
	</header>

	{#each TOGGLES as toggle (toggle.key)}
		<div class="row">
			<Chip
				active={settings.settings[toggle.key]}
				onclick={() => settings.toggle(toggle.key)}
				title={toggle.what}
			>
				{settings.settings[toggle.key] ? 'on' : 'off'}
			</Chip>
			<div class="text">
				<div title={toggle.what}>{toggle.label}</div>
				{#if toggle.pending}
					<!--
						The one line that survives the trim, because it is not a
						description of the switch — it is the switch admitting it
						does not work yet, which nothing else on screen says.
					-->
					<div class="note">Not honoured yet: {toggle.pending}.</div>
				{/if}
			</div>
		</div>
	{/each}
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 10px;
		max-width: 640px;
	}

	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	.row {
		display: flex;
		align-items: flex-start;
		gap: 8px;
	}

	.text {
		min-width: 0;
	}
</style>
