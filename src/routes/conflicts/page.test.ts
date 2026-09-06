// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { openRepository, state, sides, region } from '../../testing/git-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/conflicts/actions');
import * as api from '$lib/api';
import * as actions from '$lib/conflicts/actions';
import { conflicts } from '$lib/conflicts/store.svelte';
import Page from './+page.svelte';
let view: Mounted;
const button=(name:string)=>view.all('button').find(b=>b.textContent?.trim()===name)!;
beforeEach(()=>{
 vi.clearAllMocks();control.reset();openRepository();conflicts.clear();vi.mocked(api.inTauri).mockReturnValue(true);
 vi.mocked(api.conflicts).mockResolvedValue(state()); vi.mocked(api.conflictSides).mockImplementation(async path=>sides(path));
 vi.mocked(api.conflictRegions).mockResolvedValue([region()]);
});
afterEach(()=>{view?.destroy();conflicts.clear();control.reset();});
it('blocks continuation until every conflicted file is resolved',async()=>{
 view=render(Page,{});await vi.waitFor(()=>expect(conflicts.sides).not.toBeNull());
 expect(view.text()).toContain('shared.txt');expect((button('Continue') as HTMLButtonElement).disabled).toBe(true);
 expect(button('Continue').title).toBe('Resolve every file first');
 click(button('Abort merge'));expect(actions.abortOperation).toHaveBeenCalledWith('merge');
 vi.mocked(api.conflicts).mockResolvedValue(state({files:[]}));click(button('Refresh'));
 await vi.waitFor(()=>expect((button('Continue') as HTMLButtonElement).disabled).toBe(false));
 click(button('Continue'));expect(actions.continueOperation).toHaveBeenCalledWith('merge');
});
it('reports failures then recovers to a repository with no operation',async()=>{
 vi.mocked(api.conflicts).mockRejectedValueOnce(new Error('index unavailable'));view=render(Page,{});
 await vi.waitFor(()=>expect(view.text()).toContain('index unavailable'));
 vi.mocked(api.conflicts).mockResolvedValue(state({operation:'none',files:[]}));click(button('Refresh'));
 await vi.waitFor(()=>expect(conflicts.loaded).toBe(true));expect(button('Continue')).toBeUndefined();
 expect(view.text()).not.toContain('index unavailable');
});
