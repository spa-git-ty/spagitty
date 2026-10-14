<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { repo } from '$lib/repo.svelte';
	import Btn from '$lib/ui/Btn.svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Markdown from '$lib/ui/Markdown.svelte';
	import VirtualRows from '$lib/ui/VirtualRows.svelte';
	import { safeUrl } from '$lib/ui/markdown';
	import { issues } from './store.svelte';
	import { issueHandoff } from './handoff.svelte';
	import type { Label } from './types';

	let query = $state('is:open'), creating = $state(false), branching = $state(false);
	let title = $state(''), body = $state(''), comment = $state(''), branch = $state('');
	let handing = $state(false);
	let previousIssue = '';
	const thread = $derived(issues.current), issue = $derived(thread?.issue);
	const timeline = $derived([
		...(thread?.comments ?? []).map((comment) => ({ kind: 'comment' as const, value: comment })),
		...(thread?.events ?? []).map((event) => ({ kind: 'event' as const, value: event }))
	].sort((a, b) => a.value.at.localeCompare(b.value.at)));
	$effect(() => { query = issues.query; });
	$effect(() => { const key = `${repo.info?.path}/${issues.remote}/${issue?.id}`; if (key !== previousIssue) { previousIssue = key; branch = issue?.branchSuggestion ?? ''; branching = false; comment = ''; } });
	function term(key: string, value: string) {
		const expression = new RegExp(`(?:^|\\s)${key}:(?:"[^"]*"|\\S+)`, 'g');
		const text = issues.query.replace(expression, '').trim();
		const clean = value.replaceAll('"', '');
		void issues.filter(`${text}${clean ? ` ${key}:${/\s/.test(clean) ? `"${clean}"` : clean}` : ''}`.trim());
	}
	const color = (label: Label) => { const value = label.color || issues.metadata?.labels.find((known) => known.name === label.name)?.color || ''; return /^[a-f0-9]{6}$/i.test(value) ? `#${value}` : 'var(--muted)'; };
	const date = (at: string) => at ? new Date(at).toLocaleDateString() : '';
	async function create() { if (await issues.send({ kind: 'create', title, body })) { creating = false; title = ''; body = ''; } }
	async function post(close = false) { const number = issue?.number; if (!number) return; const sent = close ? await issues.closeWithComment(comment) : await issues.send({ kind: 'comment', number, body: comment }); if (sent) comment = ''; }
	async function startBranch() { if (await issues.branch(branch)) { branching = false; await repo.refresh(); } }
	async function hand() {
		if (!issue || !repo.info || handing) return;
		handing = true; const number = issue.number, path = repo.info.path;
		try {
			while (issues.current?.issue.number === number && issues.current.commentsMore) { await issues.moreThread('comments'); if (issues.errors.thread) return; }
			if (issues.current?.issue.number !== number || repo.info?.path !== path || issues.errors.thread) return;
			issueHandoff.prepare(path, issues.current); await goto('/farm?from=issue');
		} finally { handing = false; }
	}
</script>

<div class="screen">
	<header class="head">
		<h1>Issues</h1>
		{#if issues.hosts.length}<select aria-label="Issue host" value={issues.remote} onchange={(event) => issues.choose(event.currentTarget.value)}>{#each issues.hosts as host (host.remote)}<option value={host.remote}>{host.repo.kind === 'gitHub' ? 'GitHub' : host.repo.kind === 'gitLab' ? 'GitLab' : 'Bitbucket'} · {host.repo.owner}/{host.repo.name}</option>{/each}</select>{/if}
		<span class="grow"></span>
		<form class="search" onsubmit={(event) => { event.preventDefault(); void issues.filter(query); }}><Icon name="search" /><input aria-label="Filter issues" class="mono" bind:value={query} placeholder="is:open label:bug" /><button title="Apply filter" aria-label="Apply issue filter">↵</button></form>
		<Btn busy={issues.reading} onclick={() => issues.refresh()}>Refresh</Btn>
		{#if issues.canWrite}<Btn primary quiet onclick={() => { creating = true; }}>New issue</Btn>{/if}
	</header>
	{#each Object.entries(issues.errors).filter(([, value]) => value) as [name, error] (name)}<p class="error" role="alert">{error}</p>{/each}
	{#if issues.loading}<p class="empty note">Reading issue hosts…</p>
	{:else if !issues.host}<p class="empty note">{repo.info ? 'Add a supported hosted remote to read its issues.' : 'Open a repository to read its issues.'}</p>
	{:else if issues.host.repo.kind === 'bitbucket'}<p class="empty note">Bitbucket · soon</p>
	{:else}
		<div class="body">
			<section class="list pane" aria-label="Issue list">
				<div class="filters">
					<button class:on={issues.query.includes('is:open')} onclick={() => term('is', 'open')}>Open {issues.metadata?.open ?? '·'}</button>
					<button class:on={issues.query.includes('is:closed')} onclick={() => term('is', 'closed')}>Closed {issues.metadata?.closed ?? '·'}</button>
					<button class:on={issues.query.includes('assignee:')} onclick={() => term('assignee', issues.query.includes('assignee:') ? '' : '@me')}>Mine</button>
					<select aria-label="Issue label" onchange={(event) => term('label', event.currentTarget.value)}><option value="">Label</option>{#each issues.metadata?.labels ?? [] as label}<option value={label.name}>{label.name}</option>{/each}</select>
					<select aria-label="Issue milestone" onchange={(event) => term('milestone', event.currentTarget.value)}><option value="">Milestone</option>{#each issues.metadata?.milestones ?? [] as milestone}<option value={milestone.title}>{milestone.title}</option>{/each}</select>
					<select aria-label="Sort issues" onchange={(event) => term('sort', event.currentTarget.value)}><option value="updated">Updated</option><option value="created">Newest</option>{#if issues.host.repo.kind === 'gitHub'}<option value="comments">Replies</option>{/if}</select>
				</div>
				{#if issues.provisional}<p class="cached note">Saved list · checking the host</p>{/if}
				<VirtualRows items={issues.rows} key={(row) => row.id} estimate={() => 100} label="Issues" onrange={(_, end) => { if (end >= issues.rows.length - 4 && issues.more && !issues.reading) void issues.read(); }}>
					{#snippet row(item)}
						<button class="issue-row" class:selected={issue?.number === item.number} onclick={() => { creating = false; void issues.open(item.number); }}>
							<span class="state" class:closed={item.state === 'closed'} title={item.state}>{item.state === 'closed' ? '✓' : '○'}</span>
							<span class="row-copy"><strong>{item.title}</strong><span class="labels">{#each item.labels as label}<span class="label" style:border-color={color(label)}><span class="dot" style:background={color(label)}></span>{label.name}</span>{/each}</span><span class="note"><span class="mono">#{item.number}</span> · {item.author} · {date(item.updated)}</span></span>
							<span class="replies note" title="Replies">{item.replies}</span>
						</button>
					{/snippet}
				</VirtualRows>
				{#if !issues.rows.length && !issues.reading}<p class="empty note">{issues.errors.list ? 'No saved issues for this filter.' : 'No issues match this filter.'}</p>{/if}
				{#if issues.more}<Btn busy={issues.reading} onclick={() => issues.read()}>More issues</Btn>{/if}
			</section>
			<section class="thread pane" aria-label="Issue thread">
				{#if creating}
					<div class="new-issue"><h2>New issue</h2><label>Title<input aria-label="New issue title" bind:value={title} maxlength="256" /></label><label>Description<textarea aria-label="New issue description" bind:value={body} rows="10"></textarea></label><div class="actions"><Btn primary quiet disabled={!title.trim()} busy={issues.busy} onclick={create}>Create issue</Btn><Btn onclick={() => { creating = false; }}>Cancel</Btn></div></div>
				{:else if issue && thread}
					<div class="issue-head"><div class="heading"><h2>{issue.title} <span class="note">#{issue.number}</span></h2>{#if safeUrl(issue.url)}<a class="host-link" href={safeUrl(issue.url) ?? ''} target="_blank" rel="noopener noreferrer">Open on host ↗</a>{/if}</div><div class="actions"><span class="badge" class:closed={issue.state === 'closed'}>{issue.state === 'closed' ? 'Closed' : 'Open'}</span><span class="note">{issue.author} · {date(issue.created)}</span><span class="grow"></span><Btn disabled={!issues.metadata?.defaultBranch} onclick={() => { branching = !branching; }}>Start branch</Btn><Btn busy={handing} onclick={hand}>Hand to agent</Btn>{#if issues.canWrite && issue.state === 'open'}<Btn busy={issues.busy} onclick={() => issues.send({ kind: 'close', number: issue.number })}>Close</Btn>{/if}</div>{#if branching}<form class="branch-form" onsubmit={(event) => { event.preventDefault(); void startBranch(); }}><input aria-label="Issue branch name" class="mono" bind:value={branch} /><Btn busy={issues.busy} onclick={startBranch}>Create branch</Btn></form>{/if}</div>
					<div class="discussion">
						<article><div class="by"><strong>{issue.author}</strong><span class="note">{date(issue.created)}</span></div><Markdown source={issue.body || 'No description.'} /></article>
						{#each timeline as item (`${item.kind}:${item.value.id}`)}
							{#if item.kind === 'comment'}{@const reply = item.value}<article><div class="by"><strong>{reply.author}</strong><span class="note">{date(reply.at)}</span></div><Markdown source={reply.body} compact /></article>
							{:else}{@const event = item.value}<div class="event note"><span>{event.author}</span>{#if safeUrl(event.url)}<a href={safeUrl(event.url) ?? ''} target="_blank" rel="noopener noreferrer">{event.title}</a>{:else}<span>{event.title}</span>{/if}<span>{date(event.at)}</span></div>{/if}
						{/each}
						{#if thread.commentsMore}<Btn onclick={() => issues.moreThread('comments')}>More comments</Btn>{/if}
						{#if thread.eventsMore}<Btn onclick={() => issues.moreThread('events')}>More events</Btn>{/if}
					</div>
					{#if issues.canWrite && !issue.locked}<form class="comment" onsubmit={(event) => { event.preventDefault(); void post(); }}><textarea aria-label="Issue comment" bind:value={comment} rows="3" placeholder="Leave a comment…"></textarea><div class="actions"><span class="note">Markdown supported</span><span class="grow"></span>{#if issue.state === 'open'}<Btn disabled={!comment.trim()} busy={issues.busy} onclick={() => post(true)}>Close with comment</Btn>{/if}<Btn primary quiet disabled={!comment.trim()} busy={issues.busy} onclick={() => post()}>Comment</Btn></div></form>{/if}
				{:else}<p class="empty note">Select an issue to read its thread.</p>{/if}
			</section>
			{#if issue && !creating}<aside class="details pane" aria-label="Issue details"><section><h3>Assignees</h3>{#each issue.assignees as assignee}<p>{assignee}</p>{:else}<p class="note">None</p>{/each}</section><section><h3>Labels</h3><div class="labels">{#each issue.labels as label}<span class="label" style:border-color={color(label)}><span class="dot" style:background={color(label)}></span>{label.name}</span>{:else}<span class="note">None</span>{/each}</div></section><section><h3>Milestone</h3><p>{issue.milestone?.title ?? 'None'}</p>{#if issue.milestone?.open != null && issue.milestone.closed != null}<progress aria-label="Milestone progress" max={issue.milestone.open + issue.milestone.closed || 1} value={issue.milestone.closed}></progress><p class="note">{issue.milestone.closed} of {issue.milestone.open + issue.milestone.closed} closed</p>{/if}</section><section><h3>Development</h3>{#each thread?.events.filter((event) => event.url || event.branch) ?? [] as event}<p>{#if safeUrl(event.url)}<a href={safeUrl(event.url) ?? ''} target="_blank" rel="noopener noreferrer">{event.title}</a>{:else}{event.branch}{/if}</p>{:else}<p class="note">No linked branch or request reported.</p>{/each}</section></aside>{/if}
		</div>
	{/if}
</div>

<style>
	.screen { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; }
	.head { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; padding: 12px 16px; }
	h1 { font-size: var(--fs-title); font-weight: 500; margin: 0; }
	.head select { max-width: 360px; font-size: var(--fs-secondary); }
	.grow { flex: 1; }
	.search { display: flex; align-items: center; gap: 8px; max-width: 320px; border: 1px solid var(--line); border-radius: var(--r-pill); background: var(--sunken); padding: 4px 10px; }
	.search input { background: transparent; border: 0; outline: 0; min-width: 0; width: 230px; font-size: var(--fs-mono); }
	.body { display: flex; flex: 1; min-height: 0; gap: 10px; padding: 0 12px 12px; }
	.pane { border: 1px solid var(--pane-edge); border-radius: var(--r-panel); background: var(--surface); overflow: hidden; }
	.list { flex: 1.1; min-width: 240px; max-width: 430px; display: flex; flex-direction: column; }
	.filters { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 12px; border-bottom: 1px solid var(--line); }
	.filters button { font-size: var(--fs-secondary); border: 1px solid var(--line); border-radius: var(--r-pill); padding: 4px 9px; color: var(--muted); }
	.filters button.on { color: var(--ink); background: var(--hover); }
	.filters select { width: auto; max-width: 105px; font-size: var(--fs-secondary); }
	.issue-row { display: flex; gap: 10px; width: 100%; padding: 12px 14px; text-align: left; border-bottom: 1px solid var(--soft); }
	.issue-row.selected { background: var(--hover); box-shadow: inset 3px 0 var(--accent); }
	.row-copy { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 7px; overflow-wrap: anywhere; }
	.row-copy strong { font-weight: 550; font-size: var(--fs-ui); }
	.state, .badge { color: var(--ok); }
	.closed { color: var(--lane-4); }
	.labels { display: flex; flex-wrap: wrap; gap: 5px; }
	.label { display: inline-flex; align-items: center; gap: 5px; color: var(--ink); border: 1px solid var(--line); border-radius: var(--r-pill); padding: 1px 7px; font-size: var(--fs-secondary); }
	.dot { width: 5px; height: 5px; border-radius: 50%; }
	.thread { flex: 2; min-width: 0; display: flex; flex-direction: column; }
	.issue-head { padding: 16px 18px; border-bottom: 1px solid var(--soft); }
	.heading { display: flex; align-items: start; gap: 10px; margin-bottom: 12px; }
	h2 { font-size: var(--fs-title); font-weight: 600; line-height: 1.35; margin: 0; flex: 1; overflow-wrap: anywhere; }
	.host-link { font-size: var(--fs-secondary); white-space: nowrap; }
	.actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
	.badge { border: 1px solid currentColor; border-radius: var(--r-pill); padding: 3px 9px; font-size: var(--fs-secondary); }
	.discussion { flex: 1; min-height: 0; overflow: auto; padding: 16px 20px; }
	article + article { padding-top: 14px; margin-top: 14px; border-top: 1px solid var(--soft); }
	.by { display: flex; gap: 10px; align-items: center; margin-bottom: 10px; }
	.event { display: flex; flex-wrap: wrap; gap: 7px; padding: 10px 0; border-top: 1px solid var(--soft); }
	.comment { display: flex; flex-direction: column; gap: 10px; padding: 12px 18px; border-top: 1px solid var(--soft); }
	textarea { width: 100%; resize: vertical; padding: 9px 10px; font-size: var(--fs-ui); }
	.details { width: var(--detail-w); flex: none; padding: 16px; overflow: auto; }
	.details section + section { border-top: 1px solid var(--soft); padding-top: 12px; margin-top: 16px; }
	h3 { text-transform: uppercase; font-size: var(--fs-secondary); font-weight: 500; letter-spacing: 0.06em; color: var(--muted); margin: 0 0 10px; }
	.details p { margin: 7px 0; overflow-wrap: anywhere; font-size: var(--fs-secondary); }
	.new-issue { padding: 20px; display: flex; flex-direction: column; gap: 16px; overflow: auto; }
	.new-issue label { display: flex; flex-direction: column; gap: 7px; }
	.new-issue input, .branch-form input { padding: 8px 10px; width: 100%; }
	.branch-form { display: flex; gap: 8px; margin-top: 12px; }
	.error { color: var(--danger); margin: 0; padding: 0 16px 8px; font-size: var(--fs-secondary); }
	.note { color: var(--muted); font-size: var(--fs-secondary); }
	.empty { padding: 24px; text-align: center; margin: auto; }
	.cached { padding: 0 12px; }
	@media (max-width: 1100px) { .details { width: calc(var(--detail-w) * 0.75); padding: 12px; } .list { min-width: 210px; } .host-link { display: none; } }
	@media (max-width: 900px) { .details { width: calc(var(--detail-w) * 0.6); } .head select { max-width: 240px; } }
</style>
