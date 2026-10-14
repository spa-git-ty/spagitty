// SPDX-License-Identifier: GPL-3.0-or-later
import { beforeEach, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render } from '../../testing/mount';
import type { Asset, Hosted, Page, Release } from './types';
import { empty } from './types';
vi.mock('./api', () => ({ remotes: vi.fn(), preferences: vi.fn(), hide: vi.fn(), cached: vi.fn(), page: vi.fn(), write: vi.fn(), notes: vi.fn(), target: vi.fn(), changelog: vi.fn(), pushTarget: vi.fn(), upload: vi.fn(), artifacts: vi.fn(), download: vi.fn() }));
vi.mock('$lib/notifications/watcher.svelte', () => ({ watcher: { subscribe: vi.fn(() => vi.fn()) } }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
import * as api from './api';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { releases } from './store.svelte';
import ReleasesScreen from './ReleasesScreen.svelte';
import { dialog } from '$lib/ui/dialog.svelte';

const host: Hosted = { remote: 'origin', repo: { kind: 'gitHub', host: 'github.com', owner: 'team', name: 'app' }, user: 'ada' };
const release = (id = '12', extra: Partial<Release> = {}): Release => ({ ...empty('main'), id, tag: `v${id}`, title: 'Refunds', body: '**Changes**', url: `https://github.com/team/app/releases/tag/v${id}`, ...extra });
const page = (rows = [release()]): Page => ({ releases: rows, page: 1, more: false, etag: 'tag' });
const asset: Asset = { id: '8', name: 'app.zip', size: 1024, downloads: 0, url: 'https://github.com/file', removable: true };
const settle = async () => { await new Promise((resolve) => setTimeout(resolve, 0)); flushSync(); };
beforeEach(() => {
	vi.clearAllMocks(); releases.clear(); dialog.dismiss();
	vi.mocked(api.remotes).mockResolvedValue([host]); vi.mocked(api.preferences).mockResolvedValue({ hidden: [], drafts: [] });
	vi.mocked(api.cached).mockResolvedValue(null); vi.mocked(api.page).mockResolvedValue({ data: page(), cacheWarning: null });
	vi.mocked(api.write).mockResolvedValue(null); vi.mocked(api.notes).mockResolvedValue('### New\n\n- Refunds (#12)');
	vi.mocked(api.target).mockResolvedValue('a'.repeat(40)); vi.mocked(api.changelog).mockResolvedValue('b'.repeat(40)); vi.mocked(api.pushTarget).mockResolvedValue();
	vi.mocked(api.upload).mockResolvedValue(asset); vi.mocked(api.artifacts).mockResolvedValue([]); vi.mocked(api.download).mockResolvedValue('/cache/app.zip');
});

it('paints saved releases and keeps them when the host is offline', async () => {
	vi.mocked(api.cached).mockResolvedValue(page()); vi.mocked(api.page).mockRejectedValueOnce('Offline');
	await releases.load('/repo'); expect(releases.rows[0].id).toBe('12'); expect(releases.provisional).toBe(true); expect(releases.errors.read).toContain('Offline');
});

it('drops old remote and repository answers', async () => {
	await releases.load('/repo'); let answer!: (value: { data: Page; cacheWarning: null }) => void;
	vi.mocked(api.page).mockImplementationOnce(() => new Promise((resolve) => { answer = resolve; }));
	const old = releases.read(true); await settle(); await releases.load('/other'); answer({ data: page([release('99')]), cacheWarning: null }); await old;
	expect(releases.rows[0].id).toBe('12');
});

it('hides locally, filters drafts and can show hidden releases without a host write', async () => {
	vi.mocked(api.hide).mockResolvedValue({ hidden: ['12'], drafts: [] });
	await releases.load('/repo'); await releases.hide(releases.rows[0]); expect(releases.visible).toHaveLength(0);
	releases.showHidden = true; releases.filter = 'drafts'; expect(releases.visible).toHaveLength(1); expect(api.write).not.toHaveBeenCalled();
});

it('confirms publication and keeps uncertain delivery visible without retry', async () => {
	await releases.load('/repo'); await releases.select(releases.rows[0]); vi.mocked(api.write).mockRejectedValueOnce('Delivery uncertain. Refresh.');
	const saving = releases.save(true); expect(api.write).not.toHaveBeenCalled(); expect(dialog.question?.body).toContain('github.com/team/app');
	dialog.accept(); expect(await saving).toBe(false); expect(api.write).toHaveBeenCalledOnce();
	await releases.read(true); expect(releases.errors.write).toContain('uncertain'); expect(releases.draft?.body).toBe('**Changes**');
});

it('deletes the release before separately asking to delete its tag', async () => {
	await releases.load('/repo'); await releases.select(releases.rows[0]);
	const removing = releases.remove(true); dialog.accept(); await settle();
	expect(dialog.question?.title).toBe('Delete tag'); expect(api.write).toHaveBeenCalledTimes(1); dialog.dismiss(); await removing;
	expect(api.write).toHaveBeenCalledWith('origin', { kind: 'delete', id: '12', tag: 'v12' }); expect(releases.draft).toBeNull();
});

it('resumes a committed changelog at push confirmation without making another commit', async () => {
	await releases.load('/repo'); await releases.select(releases.rows[0]);
	const first = releases.save(true, true, 'main'); dialog.accept(); await settle(); expect(dialog.question?.title).toBe('Push changelog commit?'); dialog.dismiss(); expect(await first).toBe(false);
	expect(releases.pendingChangelog?.commit).toBe('b'.repeat(40));
	const second = releases.save(true, true, 'main'); dialog.accept(); await settle(); expect(dialog.question?.title).toBe('Publish release'); dialog.accept(); expect(await second).toBe(true);
	expect(api.changelog).toHaveBeenCalledOnce(); expect(api.pushTarget).toHaveBeenCalledWith('origin', 'main', 'a'.repeat(40), 'b'.repeat(40));
});

it('tracks each file and retries only the failed upload', async () => {
	await releases.load('/repo'); await releases.select(releases.rows[0]); releases.queue(['/tmp/app.zip', '/tmp/other.zip']);
	vi.mocked(api.upload).mockImplementationOnce(async (_, __, ___, progress) => { progress({ sent: 512, total: 1024 }); throw new Error('offline'); });
	const first = releases.upload(releases.files[0].key); dialog.accept(); await first;
	expect(releases.files[0].state).toBe('error'); expect(releases.files[0].sent).toBe(512); expect(releases.files[1].state).toBe('queued');
	const retry = releases.upload(releases.files[0].key); dialog.accept(); await retry;
	expect(releases.files[0].state).toBe('uploaded'); expect(releases.draft?.assets[0].id).toBe('8'); expect(api.upload).toHaveBeenCalledTimes(2);
});

it('a GitLab package can be linked after cancellation without uploading again', async () => {
	vi.mocked(api.remotes).mockResolvedValue([{ ...host, repo: { ...host.repo, kind: 'gitLab' } }]);
	await releases.load('/repo'); await releases.select(releases.rows[0]); releases.queue(['/tmp/app.zip']);
	const first = releases.upload(releases.files[0].key); dialog.accept(); await settle(); expect(dialog.question?.title).toBe('Link uploaded file'); dialog.dismiss(); await first;
	expect(releases.files[0].state).toBe('error'); vi.mocked(api.write).mockResolvedValueOnce({ id: 99, url: asset.url });
	const retry = releases.upload(releases.files[0].key); dialog.accept(); await settle(); dialog.accept(); await retry;
	expect(api.upload).toHaveBeenCalledOnce(); expect(releases.draft?.assets[0].id).toBe('99');
});

it('imports a completed artifact with download progress, then queues it for confirmed upload', async () => {
	await releases.load('/repo'); await releases.select(releases.rows[0]);
	vi.mocked(api.download).mockImplementationOnce(async (_, __, update) => { update({ sent: 1024, total: 1024 }); return '/cache/app.zip'; });
	await releases.importArtifact({ id: '7', name: 'app.zip', size: 1024, run: 'CI #12', commit: 'a'.repeat(40) });
	expect(releases.downloads[0].sent).toBe(1024); expect(releases.files[0].state).toBe('queued'); expect(api.upload).not.toHaveBeenCalled();
});

it('mounted editor generates notes, previews safe Markdown and queues native file picks', async () => {
	await releases.load('/repo'); const view = render(ReleasesScreen, {}); click(view.get('.release-copy')); await settle();
	click(view.all('button').find((button) => button.textContent?.trim() === 'Generate from commits')!); await settle(); expect((view.get('[aria-label="Release notes"]') as HTMLTextAreaElement).value).toContain('Refunds (#12)');
	click(view.all('button').find((button) => button.textContent?.trim() === 'Preview')!); expect(view.text()).toContain('Refunds (#12)');
	vi.mocked(openDialog).mockResolvedValueOnce(['/tmp/app.zip']); click(view.all('button').find((button) => button.textContent?.trim() === 'Choose files to attach')!); await settle(); expect(view.text()).toContain('app.zip');
	view.destroy();
});

it('late generated notes do not replace a newer request or text edited while waiting', async () => {
	await releases.load('/repo'); await releases.select(releases.rows[0]);
	let answer!: (text: string) => void;
	vi.mocked(api.notes).mockImplementationOnce(() => new Promise((resolve) => { answer = resolve; }));
	const slow = releases.generate('v1'); await releases.generate('v2'); answer('older notes'); await slow;
	expect(releases.draft?.body).toContain('Refunds (#12)');
	vi.mocked(api.notes).mockImplementationOnce(() => new Promise((resolve) => { answer = resolve; }));
	const editing = releases.generate('v1'); releases.draft!.body = 'My revised wording'; answer('generated'); await editing;
	expect(releases.draft?.body).toBe('My revised wording');
});

it('mounted creation retains the draft after cancelling publication and hides writes for read-only accounts', async () => {
	await releases.load('/repo'); const view = render(ReleasesScreen, {}); click(view.all('button').find((button) => button.textContent?.trim() === 'Draft a release')!); await settle();
	const tag = view.get('[aria-label="Release tag"]') as HTMLInputElement; tag.value = 'v3'; fire(tag, 'input');
	const target = view.get('[aria-label="Release target"]') as HTMLInputElement; target.value = 'main'; fire(target, 'input');
	click(view.all('button').find((button) => button.textContent?.trim() === 'Publish')!); dialog.dismiss(); await settle(); expect(releases.draft?.tag).toBe('v3'); expect(api.write).not.toHaveBeenCalled(); view.destroy();
	vi.mocked(api.remotes).mockResolvedValue([{ ...host, user: null }]); await releases.load('/repo'); const readonly = render(ReleasesScreen, {}); expect(readonly.text()).not.toContain('Draft a release'); readonly.destroy();
});
