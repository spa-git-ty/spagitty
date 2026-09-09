<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import BrandMark from '$lib/ui/BrandMark.svelte';
	import { version } from '$lib/version';
	import { describeLicense, matching, undeclared } from './describe';
	import { settings } from './store.svelte';

	/**
	 * About: what this build is, and what it is made of.
	 *
	 * Both halves are GPL-3 obligations rather than conveniences. The commit is
	 * stamped in at compile time so a user can obtain the source corresponding
	 * to the exact binary they are running, and the dependency list is generated
	 * from the two lockfiles at build time so it cannot fall behind an update.
	 *
	 * The version, license and trademark notice were in the stub's footer before
	 * this screen existed. They move into this section; they do not disappear
	 * for a commit while it is rebuilt.
	 */
	let query = $state('');

	const about = $derived(settings.about);
	const licenses = $derived(settings.licenses);
	const rust = $derived(matching(licenses?.rust ?? [], query));
	const npm = $derived(matching(licenses?.npm ?? [], query));
	const total = $derived((licenses?.rust.length ?? 0) + (licenses?.npm.length ?? 0));
	const missing = $derived(undeclared([...(licenses?.rust ?? []), ...(licenses?.npm ?? [])]));
</script>

<section class="section">
	<header class="about-header">
		<BrandMark size={36} />
		<div>
			<h2 class="heading">Spagitty · About &amp; License</h2>
			<span class="note">This build, and every dependency in it.</span>
		</div>
	</header>

	<dl class="facts">
		<dt class="note">Version</dt>
		<dd>Spagitty v{about?.version ?? version.number}</dd>
		<dt class="note">Build</dt>
		<dd class="mono">{about?.commit ?? version.commit}</dd>
		<dt class="note">License</dt>
		<dd>{about?.license ?? version.license}</dd>
	</dl>

	<!-- The licence. Why the commit matters is a hover (TASK-044): it is an
	     argument, not a fact anybody is scanning for. -->
	<p
		class="note"
		title="The commit above identifies the source this binary was built from, so the corresponding source can be obtained for this exact build."
	>
		Free software under the GNU General Public License, version 3 or later.
	</p>

	<h3 class="heading">Dependency licenses</h3>

	{#if licenses === null}
		<p class="note">Reading the list…</p>
	{:else if !licenses.generated}
		<p class="note">This build did not generate a dependency license list.</p>
		{#each licenses.notes as note (note)}
			<p class="note">{note}</p>
		{/each}
	{:else}
		<div class="row">
			<input
				class="field"
				type="text"
				placeholder="Filter by package or license"
				value={query}
				oninput={(event) => (query = event.currentTarget.value)}
				aria-label="Filter dependencies by package or license"
			/>
			<span class="note">
				{total} package{total === 1 ? '' : 's'} · {licenses.rust.length} Rust, {licenses.npm
					.length} npm
			</span>
		</div>

		{#each licenses.notes as note (note)}
			<p class="note">{note}</p>
		{/each}

		{#if missing > 0}
			<p class="note" title="Listed as not declared rather than left out.">
				{missing} declare no license.
			</p>
		{/if}

		<div class="lists">
			<section class="group">
				<h4 class="note heading">Rust ({rust.length})</h4>
				<ul class="list">
					{#each rust as dependency (dependency.name + dependency.version)}
						<li class="entry">
							<span class="name">{dependency.name}</span>
							<span class="mono">{dependency.version}</span>
							<span class="note">{describeLicense(dependency)}</span>
						</li>
					{/each}
				</ul>
			</section>

			<section class="group">
				<h4 class="note heading">npm ({npm.length})</h4>
				<ul class="list">
					{#each npm as dependency (dependency.name + dependency.version)}
						<li class="entry">
							<span class="name">{dependency.name}</span>
							<span class="mono">{dependency.version}</span>
							<span class="note">{describeLicense(dependency)}</span>
						</li>
					{/each}
				</ul>
			</section>
		</div>

		<p
			class="note"
			title="Generated at build time from Cargo.lock and package-lock.json. The tools that build and test Spagitty are not linked into it and are not listed."
		>
			What is linked into this binary.
		</p>
	{/if}

	<p class="note trademark">
		Spagitty is not affiliated with, endorsed by, or sponsored by the Git project or the
		Software Freedom Conservancy. Git and the Git logo are trademarks of the Software Freedom
		Conservancy.
	</p>
</section>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 760px;
	}

	.about-header {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 4px;
	}
	.heading {
		margin: 0;
		font-size: var(--fs-ui);
		font-weight: inherit;
	}

	h4.heading {
		font-size: var(--fs-secondary);
	}

	.facts {
		display: grid;
		grid-template-columns: 64px 1fr;
		gap: 2px 10px;
		margin: 0;
	}

	.facts dd {
		margin: 0;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}

	.lists {
		display: flex;
		gap: 16px;
		align-items: flex-start;
		flex-wrap: wrap;
	}

	.group {
		flex: 1;
		min-width: 260px;
	}

	.list {
		margin: 4px 0 0;
		padding: 0;
		list-style: none;
		max-height: 260px;
		overflow: auto;
		border: 1px solid var(--soft);
		border-radius: var(--r-field);
	}

	.entry {
		display: flex;
		align-items: baseline;
		gap: 8px;
		padding: 2px 8px;
	}

	.entry:nth-child(odd) {
		background: var(--stripe);
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--fs-secondary);
	}

	.field {
		background: transparent;
		border: 1px solid var(--line);
		border-radius: var(--r-field);
		color: var(--ink);
		font-family: var(--font-ui);
		font-size: var(--fs-secondary);
		padding: 3px 6px;
		width: 260px;
	}

	.field:focus {
		outline: none;
		border-color: var(--accent);
	}

	.field::placeholder {
		color: var(--placeholder);
	}

	.trademark {
		max-width: 620px;
	}

	p {
		margin: 0;
	}
</style>
