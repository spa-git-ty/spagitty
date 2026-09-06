// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import { openRepository } from '../../testing/git-fixtures';
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/ui/dialog.svelte', () => ({ dialog: { confirm: vi.fn(async () => false) } }));
vi.mock('$lib/ui/notice.svelte', () => ({ notice: { ok: vi.fn(), failed: vi.fn() } }));
vi.mock('$lib/delight/sound', () => ({ play: vi.fn() }));
import { delight } from '$lib/delight/store.svelte';
import { settings } from '$lib/settings/store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import { notice } from '$lib/ui/notice.svelte';
import Page from './+page.svelte';
let view: Mounted;
const button=(name:string)=>view.all('button').find(b=>b.textContent?.trim()===name)!;
beforeEach(()=>{
 vi.clearAllMocks(); localStorage.clear(); control.reset(); openRepository(); delight.clear(); delight.bind('/test');
 settings.settings.personality='fullSpagitty';
});
afterEach(()=>{view?.destroy(); delight.clear(); control.reset(); vi.restoreAllMocks();});
it('distinguishes no repository from no achievements',()=>{
 control.setInfo(null); view=render(Page,{}); expect(view.text()).toContain('No repository open'); view.destroy();
 openRepository(); view=render(Page,{}); expect(view.text()).toContain('Nothing has been earned');
});
it('equips an earned title, copies cards and lets the user select an agent record',async()=>{
 delight.record({kind:'recovery',how:'reflog'});
 delight.record({kind:'agentTask',testsPassed:true,approved:true,corrections:0,difficulty:'hard',handoff:true,failedElsewhere:false},{id:'agent',name:'Agent Ada',kind:'agent'});
 const copy=vi.spyOn(navigator.clipboard,'writeText').mockResolvedValue();
 view=render(Page,{}); expect(view.text()).toContain('Agents, in this repository');
 const title=view.all('.titles button')[1]; click(title);
 expect(delight.me.title).not.toBeNull();
 click(button('Copy card')); await vi.waitFor(()=>expect(copy).toHaveBeenCalled());
 expect(copy.mock.calls[0][0]).toContain('You');
 click(button('Copy markdown')); await vi.waitFor(()=>expect(copy).toHaveBeenCalledTimes(2));
 click(button('Agent Ada')); expect(view.text()).toContain('One equipped badge, shown beside Agent Ada');
 click(button('none')); expect(delight.get('agent')?.title).toBeNull();
});
it('reports clipboard refusal and confirms before forgetting the record',async()=>{
 delight.record({kind:'recovery',how:'reflog'});
 vi.spyOn(navigator.clipboard,'writeText').mockRejectedValue(new Error('denied'));
 view=render(Page,{}); click(button('Copy markdown')); await vi.waitFor(()=>expect(notice.failed).toHaveBeenCalled());
 const forget=view.all('button').find(b=>b.textContent?.includes('Forget'))!;
 click(forget); await vi.waitFor(()=>expect(dialog.confirm).toHaveBeenCalled()); expect(delight.list.length).toBeGreaterThan(0);
 vi.mocked(dialog.confirm).mockResolvedValue(true); click(forget);
 await vi.waitFor(()=>expect(delight.list).toEqual([]));
});
