<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { inTauri } from '$lib/api';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Markdown from '$lib/ui/Markdown.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { safeUrl } from '$lib/ui/markdown';
	import { notice } from '$lib/ui/notice.svelte';
	import { releases } from './store.svelte';
	import type { Release } from './types';

	let preview = $state(false), since = $state(''), branch = $state(''), writeChangelog = $state(false), deleteTag = $state(false);
	let menu = $state<string | null>(null), pipeline = $state(false);
	const draft = $derived(releases.draft);
	const size = (bytes: number | null) => bytes == null ? '' : bytes < 1024 ? `${bytes} B` : bytes < 1048576 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1048576).toFixed(1)} MB`;
	const date = (at: string) => at ? new Date(at).toLocaleDateString() : 'Not published';
	async function copy(text: string) { try { await navigator.clipboard.writeText(text); notice.ok('Link copied'); } catch (error) { notice.failed('Could not copy link', error); } }
	async function pick() {
		const chosen = await openDialog({ multiple: true, directory: false, title: 'Attach release files' });
		if (chosen) releases.queue(Array.isArray(chosen) ? chosen : [chosen]);
	}
	async function select(release: Release) { await releases.select(release); if (releases.selected === release.id) { branch = release.target; since = ''; writeChangelog = false; deleteTag = false; pipeline = false; } }
	async function create() { await releases.create(); if (!releases.selected) { branch = ''; since = releases.rows.find((release) => !release.draft)?.tag ?? ''; writeChangelog = false; deleteTag = false; pipeline = false; } }
	onMount(() => {
		if (!inTauri()) return;
		let stopped = false, unlisten: (() => void) | undefined;
		void import('@tauri-apps/api/webviewWindow').then(async ({ getCurrentWebviewWindow }) => {
			const stop = await getCurrentWebviewWindow().onDragDropEvent(({ payload }) => { if (payload.type === 'drop' && releases.draft && releases.canWrite) releases.queue(payload.paths); });
			if (stopped) stop(); else unlisten = stop;
		}).catch((error) => notice.failed('File drop is unavailable', error));
		return () => { stopped = true; unlisten?.(); };
	});
</script>

<div class="screen">
	<header class="head">
		<h1>Releases</h1>
		{#if releases.hosts.length}<select aria-label="Release host" value={releases.remote} onchange={(event) => releases.choose(event.currentTarget.value)}>{#each releases.hosts as host (host.remote)}<option value={host.remote}>{host.repo.kind === 'gitHub' ? 'GitHub' : host.repo.kind === 'gitLab' ? 'GitLab' : 'Bitbucket'} · {host.repo.owner}/{host.repo.name}</option>{/each}</select>{/if}
		<span class="grow"></span><label class="check"><input type="checkbox" bind:checked={releases.showHidden} />Show hidden · {releases.hidden.length}</label>
		<Btn busy={releases.reading} onclick={() => releases.read(true)}>Refresh</Btn>
		{#if releases.canWrite}<Btn primary quiet onclick={create}>Draft a release</Btn>{/if}
	</header>
	{#each Object.values(releases.errors).filter(Boolean) as error}<p class="error" role="alert">{error}</p>{/each}
	{#if releases.loading}<p class="empty note">Reading releases…</p>
	{:else if !releases.host}<p class="empty note">{repo.info ? 'Add a supported hosted remote to read its releases.' : 'Open a repository to read its releases.'}</p>
	{:else if releases.host.repo.kind === 'bitbucket'}<p class="empty note">Bitbucket · soon</p>
	{:else}<div class="body">
		<section class="list pane" aria-label="Release list">
			<div class="filters">{#each [['all', 'All'], ['drafts', 'Drafts'], ['prereleases', 'Pre-releases']] as [key, label]}<button class:on={releases.filter === key} onclick={() => { releases.filter = key as 'all' | 'drafts' | 'prereleases'; }}>{label} {releases.rows.filter((release) => key === 'all' || (key === 'drafts' ? release.draft : release.prerelease)).length}{releases.more ? '+' : ''}</button>{/each}</div>
			{#if releases.provisional}<p class="note cached">Saved list · checking the host</p>{/if}
			<VirtualRows items={releases.visible} key={(release) => release.id} estimate={() => 90} label="Releases" onrange={(_, end) => { if (end >= releases.visible.length - 3 && releases.more && !releases.reading) void releases.read(); }}>
				{#snippet row(release)}
					<div class="release-row" class:selected={releases.selected === release.id}>
						<button class="release-copy" onclick={() => select(release)}><span class="row-title"><strong class="mono">{release.tag}</strong>{#if release.draft}<span class="badge draft">Draft</span>{:else if release.prerelease}<span class="badge pre">Pre-release</span>{:else if release.latest}<span class="badge latest">Latest</span>{/if}</span><span class="note">{date(release.created)}{release.author ? ` · ${release.author}` : ''}</span>{#if release.assets.some((asset) => asset.downloads != null)}<span class="note">{release.assets.reduce((sum, asset) => sum + (asset.downloads ?? 0), 0).toLocaleString()} ↓</span>{/if}</button>
						<button class="dots" aria-label={`Actions for ${release.tag}`} aria-expanded={menu === release.id} onclick={() => { menu = menu === release.id ? null : release.id; }}>⋯</button>
						{#if menu === release.id}<div class="row-menu">{#if safeUrl(release.url)}<a href={safeUrl(release.url) ?? ''} target="_blank" rel="noopener noreferrer">Open on host ↗</a><button onclick={() => copy(release.url)}>Copy link</button>{/if}<button title="Only hidden on this machine; the release remains on the host" onclick={() => releases.hide(release)}>{releases.hidden.includes(release.id) ? 'Unhide' : 'Hide'}</button>{#if releases.canWrite}<button onclick={async () => { await select(release); if (releases.selected === release.id) await releases.remove(false); }}>Delete</button>{/if}</div>{/if}
					</div>
				{/snippet}
			</VirtualRows>
			{#if !releases.visible.length && !releases.reading && !Object.values(releases.errors).some(Boolean)}<p class="empty note">No releases match this filter.</p>{/if}
			{#if releases.more}<Btn busy={releases.reading} onclick={() => releases.read()}>More releases</Btn>{/if}
		</section>
		<section class="editor pane" aria-label="Release editor">
			{#if draft}
				<div class="editor-head"><h2 class="mono">{draft.tag || 'New release'}</h2>{#if draft.draft}<span class="badge draft">Draft</span><span class="note">{draft.local || releases.host.repo.kind === 'gitLab' ? 'Saved on this machine until published' : 'Only people with push access can see it'}</span>{/if}<span class="grow"></span>{#if releases.canWrite}{#if draft.id}<Btn danger busy={releases.busy} onclick={() => releases.remove(deleteTag)}>Delete</Btn><Btn title="Only hidden on this machine" onclick={() => releases.hide(draft)}>{releases.hidden.includes(draft.id) ? 'Unhide' : 'Hide'}</Btn>{/if}{#if draft.draft}<Btn busy={releases.busy} onclick={() => releases.save(false)}>Save draft</Btn>{/if}<Btn primary quiet busy={releases.busy} onclick={() => releases.save(true, writeChangelog, branch)}>{draft.draft ? 'Publish' : 'Save changes'}</Btn>{/if}</div>
				<div class="edit-body"><fieldset class="content" disabled={releases.busy}>
					<div class="fields"><label>Tag<input aria-label="Release tag" class="mono" bind:value={draft.tag} readonly={!releases.canWrite || (!draft.draft && !draft.local)} placeholder="v3.1.0" /></label><label>From<input aria-label="Release target" class="mono" bind:value={draft.target} readonly={!releases.canWrite} placeholder="main or commit id" /></label><label>Since<input aria-label="Release since" class="mono" bind:value={since} placeholder="previous tag" /></label></div>
					<label class="title-field">Title<input aria-label="Release title" bind:value={draft.title} readonly={!releases.canWrite} /></label>
					<div class="options">{#if releases.host.repo.kind === 'gitHub'}<label class="check"><input type="checkbox" bind:checked={draft.prerelease} disabled={!releases.canWrite} />Pre-release</label><label class="check"><input type="checkbox" bind:checked={draft.latest} disabled={!releases.canWrite || draft.prerelease} />Latest</label>{/if}<label class="check"><input type="checkbox" bind:checked={writeChangelog} disabled={!releases.canWrite || !draft.draft} />Write to CHANGELOG.md</label></div>
					{#if writeChangelog}<label class="title-field">Target branch for changelog<input aria-label="Changelog target branch" class="mono" bind:value={branch} placeholder="main" /></label><p class="note">An ordinary commit runs hooks before the tag is created. Commit and push each ask for confirmation.</p>{/if}
					{#if releases.pendingChangelog}<p class="note" role="status">Changelog committed at {releases.pendingChangelog.commit.slice(0, 12)}. Publish resumes at the push confirmation.</p>{/if}
					<div class="notes"><div class="notes-head"><div class="tabs"><button class:on={!preview} onclick={() => { preview = false; }}>Write</button><button class:on={preview} onclick={() => { preview = true; }}>Preview</button></div><span class="grow"></span><Btn onclick={() => releases.generate(since)}>Generate from commits</Btn>{#if releases.host.repo.kind === 'gitHub' && releases.canWrite}<Btn busy={releases.busy} onclick={() => releases.generate(since, true)}>GitHub notes</Btn>{/if}</div><div class="notes-body">{#if preview}<Markdown source={draft.body || 'No release notes.'} />{:else}<textarea aria-label="Release notes" bind:value={draft.body} readonly={!releases.canWrite} placeholder="Write release notes in Markdown…"></textarea>{/if}</div></div>
					{#if draft.id && !draft.local && releases.canWrite}<label class="check delete-tag"><input type="checkbox" bind:checked={deleteTag} />Also delete Git tag when deleting this release</label>{/if}
				</fieldset><aside class="files" aria-label="Release files"><h3>Files · {draft.assets.length}{draft.assets.some((asset) => asset.size != null) ? ` · ${size(draft.assets.reduce((sum, asset) => sum + (asset.size ?? 0), 0))}` : ''}</h3>
					{#each draft.assets as asset (asset.id)}<div class="asset"><div class="asset-copy"><strong class="mono" title={asset.name}>{asset.name}</strong><span class="note">{size(asset.size)}{asset.downloads != null ? ` · ${asset.downloads.toLocaleString()} downloads` : ''}</span></div>{#if safeUrl(asset.url)}<button aria-label={`Copy link for ${asset.name}`} onclick={() => copy(asset.url)}>↗</button>{/if}{#if asset.removable && releases.canWrite}<button aria-label={`Remove ${asset.name}`} onclick={() => releases.removeAsset(asset)}>×</button>{/if}</div>{/each}
					{#each releases.files as file (file.key)}<div class="transfer"><strong class="mono">{file.name}</strong><progress aria-label={`Upload ${file.name}`} max={file.total || 1} value={file.sent}></progress><span class="note">{file.state === 'uploading' ? `${size(file.sent)} / ${size(file.total)}` : file.state}</span>{#if file.error}<p class="error" role="alert">{file.error}</p>{/if}{#if file.state === 'queued' || file.state === 'error'}<Btn onclick={() => releases.upload(file.key)}>{file.state === 'error' ? 'Retry file' : 'Upload'}</Btn><button aria-label={`Remove queued ${file.name}`} onclick={() => releases.forgetFile(file.key)}>×</button>{/if}</div>{/each}
					{#if releases.canWrite}<div class="drop"><Btn onclick={pick}>Choose files to attach</Btn><span class="note">or drop files here</span><Btn onclick={() => { pipeline = !pipeline; if (pipeline) void releases.readArtifacts(); }}>From pipeline</Btn></div>{/if}
					{#if pipeline}<div class="artifacts"><h3>Finished runs on target</h3>
						{#each releases.artifacts as artifact (artifact.id)}
							{@const download = releases.downloads.find((file) => file.id === artifact.id)}
							<div class="transfer"><strong>{artifact.name}</strong><span class="note">{artifact.run} · {size(artifact.size)}</span>
								{#if download}<progress aria-label={`Download ${artifact.name}`} max={download.total || 1} value={download.sent}></progress><span class="note">{size(download.sent)} / {size(download.total)}</span>{#if download.error}<p class="error" role="alert">{download.error}</p>{/if}{/if}
								<Btn busy={download?.busy} onclick={() => releases.importArtifact(artifact)}>{download?.error ? 'Retry download' : download?.path ? 'Queued' : 'Attach'}</Btn>
							</div>
						{:else}<p class="note">No finished artifacts found on this target.</p>{/each}
					</div>{/if}
					<p class="source-note note">Source code archives are supplied by the host after publishing.</p>
				</aside></div>
			{:else}<p class="empty note">Select a release or draft a new one.</p>{/if}
		</section>
	</div>{/if}
</div>

<style>
	.screen { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; }
	.head { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; padding: 12px 16px; }
	h1 { font-size: var(--fs-title); font-weight: 500; margin: 0; }
	h2 { font-size: var(--fs-title); font-weight: 600; margin: 0; overflow-wrap: anywhere; }
	h3 { font-size: var(--fs-secondary); font-weight: 500; text-transform: uppercase; color: var(--muted); letter-spacing: 0.06em; margin: 0 0 12px; }
	.head select { max-width: 340px; font-size: var(--fs-secondary); }
	.grow { flex: 1; }
	.body { display: flex; flex: 1; min-height: 0; gap: 12px; padding: 0 12px 12px; }
	.pane { background: var(--surface); border: 1px solid var(--pane-edge); border-radius: var(--r-panel); overflow: hidden; }
	.list { display: flex; flex-direction: column; flex: 1; min-width: 220px; max-width: 390px; }
	.filters { display: flex; flex-wrap: wrap; gap: 6px; padding: 12px; border-bottom: 1px solid var(--line); }
	.filters button, .tabs button { font-size: var(--fs-secondary); border: 1px solid var(--line); border-radius: var(--r-pill); padding: 4px 9px; color: var(--muted); }
	.filters button.on, .tabs button.on { background: var(--hover); color: var(--ink); }
	.release-row { display: flex; flex-wrap: wrap; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--soft); gap: 8px; }
	.release-row.selected { background: var(--hover); box-shadow: inset 3px 0 var(--accent); }
	.release-copy { display: flex; flex: 1; flex-direction: column; align-items: start; gap: 8px; text-align: left; min-width: 0; }
	.row-title { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
	.row-title strong { font-size: var(--fs-ui); }
	.badge { font-size: var(--fs-secondary); border-radius: var(--r-pill); padding: 2px 8px; background: var(--sunken); }
	.badge.draft { color: var(--warn); }.badge.pre { color: var(--lane-2); }.badge.latest { color: var(--ok); }
	.dots { padding: 6px; }.row-menu { width: 100%; display: flex; flex-wrap: wrap; gap: 10px; font-size: var(--fs-secondary); }
	.editor { flex: 3; min-width: 0; display: flex; flex-direction: column; }
	.editor-head { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; padding: 16px 18px; border-bottom: 1px solid var(--line); }
	.edit-body { display: flex; flex: 1; min-height: 0; overflow: auto; }.content { border: 0; margin: 0; flex: 2; min-width: 220px; padding: 18px; display: flex; flex-direction: column; gap: 16px; }
	.fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px; }
	.fields label, .title-field { display: flex; flex-direction: column; gap: 8px; color: var(--muted); font-size: var(--fs-secondary); }
	.fields input, .title-field input { width: 100%; min-width: 0; padding: 9px 10px; color: var(--ink); background: var(--sunken); }
	.options { display: flex; flex-wrap: wrap; gap: 14px; }.check { display: inline-flex; align-items: center; gap: 7px; font-size: var(--fs-secondary); }
	.notes { flex: 1; min-height: 280px; display: flex; flex-direction: column; background: var(--sunken); border-radius: var(--r-panel); overflow: hidden; border: 1px solid var(--soft); }
	.notes-head { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; padding: 10px; border-bottom: 1px solid var(--soft); }.tabs { display: flex; gap: 3px; }
	.notes-body { flex: 1; padding: 16px; }.notes-body textarea { width: 100%; min-height: 320px; height: 100%; resize: vertical; background: transparent; border: 0; font-family: var(--font-mono); font-size: var(--fs-mono); line-height: 1.6; }
	.files { width: var(--detail-w); min-width: 230px; border-left: 1px solid var(--line); padding: 18px; display: flex; flex-direction: column; gap: 10px; }
	.asset { display: flex; align-items: center; gap: 8px; padding: 12px; background: var(--hover); border: 1px solid var(--soft); border-radius: var(--r-field); }.asset-copy { display: flex; flex: 1; flex-direction: column; gap: 5px; min-width: 0; }.asset-copy strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--fs-mono); font-weight: 500; }
	.drop { display: flex; flex-direction: column; align-items: center; gap: 10px; border: 1px dashed var(--line); border-radius: var(--r-panel); padding: 20px 10px; }.transfer { padding: 10px; border: 1px solid var(--line); border-radius: var(--r-field); display: flex; flex-direction: column; gap: 8px; overflow-wrap: anywhere; }.transfer strong { font-size: var(--fs-mono); }
	.source-note { margin-top: auto; padding-top: 24px; }.note { color: var(--muted); font-size: var(--fs-secondary); }.empty { margin: auto; padding: 24px; text-align: center; }.error { color: var(--danger); font-size: var(--fs-secondary); margin: 0; padding: 0 16px 8px; }.cached { padding: 0 12px; }.delete-tag { color: var(--muted); }
	@media (max-width: 1150px) { .list { min-width: 190px; }.files { width: calc(var(--detail-w) * 0.8); min-width: 190px; padding: 12px; }.fields { grid-template-columns: 1fr; gap: 8px; }.content { padding: 14px; } }
</style>
