// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, fire, render, type Mounted } from '../testing/mount';
import { tag, log, openRepository } from '../testing/git-fixtures';
import { control } from '../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../testing/repo-store.svelte'));
vi.mock('$lib/tags/actions');
vi.mock('$lib/reflog/actions');
import * as api from '$lib/api';
import * as tagActions from '$lib/tags/actions';
import * as logActions from '$lib/reflog/actions';
import { tags } from '$lib/tags/store.svelte';
import { reflog } from '$lib/reflog/store.svelte';
import Tags from './tags/+page.svelte';
import Reflog from './reflog/+page.svelte';
let view: Mounted;
const button = (name: string) => view.all('button').find(b => b.textContent?.trim() === name)!;
const type = (selector: string, value: string) => { const input=view.get(selector) as HTMLInputElement; input.value=value; fire(input,'input'); };
beforeEach(() => {
 vi.clearAllMocks(); vi.mocked(api.inTauri).mockReturnValue(true); control.reset(); openRepository(); tags.clear(); reflog.clear();
 vi.mocked(api.tags).mockResolvedValue([tag('v1'),tag('v0',{annotated:false,message:''})]);
 vi.mocked(api.reflog).mockResolvedValue(log()); vi.mocked(api.reflogRefs).mockResolvedValue(['HEAD','main']);
});
afterEach(() => { view?.destroy(); tags.clear(); reflog.clear(); control.reset(); });
it('distinguishes annotated/lightweight tags and sends the selected tag to actions', async () => {
 view=render(Tags,{}); await vi.waitFor(() => expect(tags.loaded).toBe(true));
 expect(view.text()).toContain('annotated'); expect(view.text()).toContain('lightweight'); expect(view.text()).toContain('v1 released');
 click(button('edit message')); expect(tagActions.editMessage).toHaveBeenCalledWith(tag('v1'));
 click(button('check out')); expect(tagActions.checkoutTag).toHaveBeenCalledWith(tag('v1'));
 click(button('delete')); expect(tagActions.deleteTag).toHaveBeenCalledWith(tag('v1'));
 type('[aria-label="Filter tags"]','absent'); expect(view.text()).toContain('No tag matches'); expect(view.text()).toContain('2 hidden');
 type('[aria-label="Filter tags"]','v1'); expect(view.all('[role="row"]')).toHaveLength(1);
});
it('creates the entered annotated tag and shows a failed write', async () => {
 view=render(Tags,{}); await vi.waitFor(() => expect(tags.loaded).toBe(true));
 expect((button('Create') as HTMLButtonElement).disabled).toBe(true);
 type('[aria-label="New tag name"]','v2'); type('[aria-label="Commit to tag"]','main'); type('[aria-label="Tag message"]','Second release');
 vi.mocked(api.tagCreate).mockRejectedValueOnce(new Error('tag already exists'));
 click(button('Create')); await vi.waitFor(() => expect(view.text()).toContain('tag already exists'));
 expect(api.tagCreate).toHaveBeenCalledWith('v2','main','Second release');
});
it('preserves reflog revision identity when invoking recovery and switching refs', async () => {
 view=render(Reflog,{}); await vi.waitFor(() => expect(reflog.loaded).toBe(true));
 expect(view.text()).toContain('HEAD@{0}'); click(button('branch here')); expect(logActions.branchHere).toHaveBeenCalledWith(log().entries[0]);
 click(button('check out')); expect(logActions.checkoutHere).toHaveBeenCalledWith(log().entries[0]);
 click(button('reset here')); expect(logActions.resetHere).toHaveBeenCalledWith(log().entries[0]);
 type('[aria-label="Filter the reflog"]','missing'); expect(view.text()).toContain('Nothing matches');
 click(button('main')); await vi.waitFor(() => expect(api.reflog).toHaveBeenCalledWith('refs/heads/main',expect.any(Number)));
 click(button('HEAD')); await vi.waitFor(() => expect(reflog.reference).toBe(''));
});
it.each([
 [false,[], 'This ref has no reflog'],
 [true,[], 'Nothing has moved this ref']
] as const)('distinguishes absent and empty reflogs (%s)',async (exists, entries, expected) => {
 vi.mocked(api.reflog).mockResolvedValue(log({exists,entries:[...entries]}));
 view=render(Reflog,{}); await vi.waitFor(() => expect(view.text()).toContain(expected));
});
it.each([Tags, Reflog])('avoids reading a closed repository', Page => {
 control.setInfo(null); view=render(Page,{}); expect(view.text()).toContain('No repository open');
 expect(api.tags).not.toHaveBeenCalled(); expect(api.reflog).not.toHaveBeenCalled();
});
it('shows tag read errors and then an empty list after refresh', async () => {
 vi.mocked(api.tags).mockRejectedValue(new Error('read denied')); view=render(Tags,{});
 await vi.waitFor(() => expect(view.text()).toContain('read denied'));
 vi.mocked(api.tags).mockResolvedValue([]); click(button('Refresh'));
 await vi.waitFor(() => expect(view.text()).toContain('no tags yet'));
});
